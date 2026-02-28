section .data
    msg_child db "IPC message from Child via Pipe!", 10
    msg_child_len equ $ - msg_child

    msg_parent_wait db "Parent waiting for pipe data...", 10
    msg_parent_wait_len equ $ - msg_parent_wait

section .bss
    pipe_fds resd 2   ; Array of [fd_read, fd_write]
    read_buf resb 64

section .text
global _start

_start:
    ; 1. Create Pipe
    mov rax, 12           ; SYS_PIPE
    lea rdi, [rel pipe_fds]
    int 0x80

    ; 2. Fork
    mov rax, 4            ; SYS_FORK
    int 0x80

    test rax, rax         ; Is Child?
    jz .child

.parent:
    ; PARENT PROCESS
    
    ; Print waiting message
    mov rax, 1            ; SYS_WRITE
    mov rdi, 1            ; stdout
    lea rsi, [rel msg_parent_wait]
    mov rdx, msg_parent_wait_len
    int 0x80
    
    ; Close Write End (Parent only reads)
    mov rax, 8            ; SYS_CLOSE
    mov edi, dword [rel pipe_fds + 4] ; fd_write
    int 0x80

    ; Read from Pipe (Will BLOCK until child writes!)
    mov rax, 9            ; SYS_READ
    mov edi, dword [rel pipe_fds]     ; fd_read
    lea rsi, [rel read_buf]
    mov rdx, 64
    int 0x80

    ; Print what we read from the pipe
    mov rdx, rax          ; Number of bytes read
    mov rax, 1            ; SYS_WRITE
    mov rdi, 1            ; stdout
    lea rsi, [rel read_buf]
    int 0x80

    ; Wait for child to exit fully before we exit
    mov rax, 6            ; SYS_WAIT
    mov rdi, -1
    int 0x80

    ; Exit parent
    mov rax, 0            ; SYS_EXIT
    mov rdi, 0
    int 0x80

.child:
    ; CHILD PROCESS

    ; Close Read End (Child only writes)
    mov rax, 8            ; SYS_CLOSE
    mov edi, dword [rel pipe_fds] ; fd_read
    int 0x80

    ; Yield a few times to force parent to block on empty pipe
    mov rax, 2            ; SYS_YIELD
    int 0x80
    mov rax, 2            ; SYS_YIELD
    int 0x80

    ; Write message to pipe!
    mov rax, 1            ; SYS_WRITE
    mov edi, dword [rel pipe_fds + 4] ; fd_write
    lea rsi, [rel msg_child]
    mov rdx, msg_child_len
    int 0x80

    ; Child exit
    mov rax, 0            ; SYS_EXIT
    mov rdi, 0
    int 0x80
