section .data
    msg_parent db "FORK SUCCESS - Parent Wait finished!", 10
    msg_p_len equ $ - msg_parent

    msg_child db "FORK SUCCESS - Child Running!", 10
    msg_c_len equ $ - msg_child

    file_path db "/disk/test.elf", 0
    file_path_len equ $ - file_path

section .text
global _start

_start:
    ; syscall: fork
    mov rax, 4          ; SYS_FORK
    int 0x80
    
    cmp rax, 0
    je .is_child

.is_parent:
    ; wait for child (-1 => u64::MAX)
    mov rdi, -1
    mov rax, 6          ; SYS_WAIT
    int 0x80
    
    ; write parent done
    mov rax, 1
    lea rdi, [rel msg_parent]
    mov rsi, msg_p_len
    int 0x80
    
    mov rax, 0          ; SYS_EXIT
    mov rdi, 0
    int 0x80

.is_child:
    ; write child
    mov rax, 1
    lea rdi, [rel msg_child]
    mov rsi, msg_c_len
    int 0x80

    ; exec /test.elf
    mov rax, 5          ; SYS_EXEC
    lea rdi, [rel file_path]
    mov rsi, file_path_len - 1
    int 0x80
    
    mov rax, 0          ; SYS_EXIT
    mov rdi, 123
    int 0x80
