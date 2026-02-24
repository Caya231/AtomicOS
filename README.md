# AtomicOS

AtomicOS é um sistema operacional escrito em Rust (arquitetura x86_64) projetado para demonstrar princípios de segurança de memória, modularidade e expansão futura. Ele possui um bootloader nativo compatível com Multiboot2 escrito em ASM, lidando com a GDT, IDT (Interrupções de Hardware/Exceções da CPU), e uma estrutura preparatória para Paginação, Alocadores de Frame e módulos futuros de SO (Scheduler, Syscalls e Drivers).

## Pré-requisitos

O sistema baseia-se fortemente em ferramentas GNU para gerar o empacotamento da `.iso` bootável suportada por BIOS e UEFI através do GRUB.

Você necessita ter instalado (em Ubuntu/Debian):
```bash
sudo apt update
sudo apt install nasm qemu-system-x86 build-essential grub-pc-bin grub-common xorriso
```

Você também precisa da ferramenta fundamental da arquitetura Rust, setada para a versão `nightly`:
```bash
rustup override set nightly
rustup component add rust-src llvm-tools-preview
```

## Estrutura de Diretórios e Componentes

- `boot/*.asm`: Entrada em Assembly para conformidade com Multiboot2, setagem de Paginação para Identidade, e salto de 32-bit (Protected Mode) para 64-bit (Long Mode).
- `linker.ld`: Linker script para definir que o Kernel é carregado confortavelmente a partir de 1M no espaço de memória.
- `src/lib.rs` (e módulos submetidos em `src/`): Kernel em Rust puro (`no_std`) que assume a execução após o bootloader, inicializando a VGA, Serial (COM1), GDT, IDT, PIC e alocadores iniciais.

## Instruções de Uso

### 1. Compilação (Apenas Kernel e Bootloader)
Para compilar apenas os objetos (Assembly) e a library do Kernel (Rust), além de mesclá-los utilizando o `ld`:

```bash
make
```
*(Ele irá executar implicitamente `make iso`).*

### 2. Rodando no QEMU
Você pode automaticamente testar o sistema no emulador oficial x86 usando um script local ou via `make`:

```bash
./run.sh
```
Ou:
```bash
make run
```
Isso invoca o `qemu-system-x86_64` exibindo a interface VGA e embutindo a UART Serial no `stdio` do terminal para leitura dos logs kernel.

### 3. Limpeza do Projeto
Para limpar as pastas de alvo geradas (`/target` e `/build`), basta rodar:
```bash
make clean
```
