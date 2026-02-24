# AtomicOS

AtomicOS is an operating system written in Rust (x86_64 architecture) designed to demonstrate principles of memory safety, modularity, and future expansion. It features a native Multiboot2-compliant bootloader written in Assembly, handling the GDT, IDT (Hardware Interrupts/CPU Exceptions), and a preparatory structure for Paging, Frame Allocators, and future OS modules (Scheduler, Syscalls, and Drivers).

## Prerequisites

The system relies heavily on GNU tools to generate the bootable `.iso` packaging supported by BIOS and UEFI via GRUB.

You need to have installed (on Ubuntu/Debian):
```bash
sudo apt update
sudo apt install nasm qemu-system-x86 build-essential grub-pc-bin grub-common xorriso
```

You also need the fundamental tool of the Rust architecture, set to the `nightly` version:
```bash
rustup override set nightly
rustup component add rust-src llvm-tools-preview
```

## Directory Structure and Components

- `boot/*.asm`: Assembly entry for Multiboot2 compliance, Identity Paging setup, and 32-bit (Protected Mode) to 64-bit (Long Mode) jump.
- `linker.ld`: Linker script defining that the Kernel is comfortably loaded from 1M in the memory space.
- `src/lib.rs` (and submodules in `src/`): Pure Rust Kernel (`no_std`) that takes over execution after the bootloader, initializing the VGA, Serial (COM1), GDT, IDT, PIC, advanced input drivers (PS/2 Keyboard & Mouse), Virtual TTY, and initial allocators.

## Usage Instructions

### 1. Compilation (Kernel and Bootloader Only)
To compile only the objects (Assembly) and the Kernel library (Rust), as well as merging them using `ld`:

```bash
make
```
*(This will implicitly run `make iso`).*

### 2. Running in QEMU
You can automatically test the system in the official x86 emulator using a local script or via `make`:

```bash
./run.sh
```
Or:
```bash
make run
```
This invokes `qemu-system-x86_64` displaying the VGA interface and embedding the Serial UART in the terminal's `stdio` for reading kernel logs.

### 3. Cleaning the Project
To clean the generated target folders (`/target` and `/build`), simply run:
```bash
make clean
```
