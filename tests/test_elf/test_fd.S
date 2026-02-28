section .text
global _start
_start:
mov rax, 2
int 0x80
mov rax, 2
int 0x80
mov rax, 2
int 0x80
mov rax, 0
mov rdi, 42
int 0x80
