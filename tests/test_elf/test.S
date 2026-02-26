section .data
    msg db "Hello from Ring 3!", 10 ; \n
    msg_len equ $ - msg

section .text
global _start

_start:
    ; syscall: sys_write(1) — rdi=ptr, rsi=len
    mov rax, 1          ; SYS_WRITE
    lea rdi, [rel msg]  ; pointer to message
    mov rsi, msg_len    ; length
    int 0x80

    ; syscall: sys_exit(0)
    mov rax, 0          ; SYS_EXIT
    int 0x80

    ; Should never reach here
    jmp $
