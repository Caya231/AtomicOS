# AtomicOS — Build Dependencies

## Required Tools

### GNU Toolchain
| Tool | Package (Ubuntu/Debian) | Purpose |
|------|------------------------|---------|
| `nasm` | `nasm` | Assemble bootloader (.asm → .o) |
| `ld` | `binutils` or `binutils-x86-64-linux-gnu` | Link ELF64 kernel binary |
| `gcc` | `gcc` or `gcc-x86-64-linux-gnu` | Compile C bootloader stubs (optional) |
| `objcopy` | `binutils` | Generate flat binary for baremetal |
| `make` | `build-essential` | Build automation |
| `gdb` | `gdb-multiarch` | Debug kernel in QEMU (optional) |

### GRUB & ISO
| Tool | Package | Purpose |
|------|---------|---------|
| `grub-mkrescue` | `grub-pc-bin` + `grub-common` | Generate bootable ISO |
| `xorriso` | `xorriso` | ISO filesystem backend |

### Emulation
| Tool | Package | Purpose |
|------|---------|---------|
| `qemu-system-x86_64` | `qemu-system-x86` | Run/test the OS |

### Rust
| Tool | How to install | Purpose |
|------|---------------|---------|
| `rustup` | https://rustup.rs | Rust toolchain manager |
| `rust nightly` | `rustup override set nightly` | Required for `abi_x86_interrupt`, `alloc_error_handler` |
| `rust-src` | `rustup component add rust-src` | Build `core`/`alloc` from source |

## One-liner Install (Ubuntu/Debian)

```bash
sudo apt update && sudo apt install -y nasm build-essential qemu-system-x86 grub-pc-bin grub-common xorriso gdb-multiarch
rustup override set nightly && rustup component add rust-src llvm-tools-preview
```

## Build & Run

```bash
make          # Build ISO
make run      # Build + boot in QEMU
make debug    # Build + boot paused for GDB on :1234
make flat     # Generate flat binary via objcopy
make clean    # Remove all build artifacts
make help     # Show all targets
```

## GDB Debugging

Terminal 1:
```bash
make debug
```

Terminal 2:
```bash
gdb build/kernel.bin -ex 'target remote :1234'
```
