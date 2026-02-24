section .text
global _start

_start:
    ; Simple test: set registers and loop
    mov rax, 0xDEADBEEF
    mov rbx, 0xCAFEBABE
.loop:
    nop
    jmp .loop
