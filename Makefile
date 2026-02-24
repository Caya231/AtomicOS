# ============================================================================
#  AtomicOS — GNU Toolchain Makefile
#  Compiles ASM bootloader + Rust kernel → ELF64 → Bootable ISO
# ============================================================================

# --- Output Paths ---
KERNEL_LIB   := target/target/release/libatomic_os.a
KERNEL_BIN   := build/kernel.bin
KERNEL_FLAT  := build/kernel.flat
ISO_FILE     := build/atomic_os.iso

# --- GNU Toolchain ---
AS       := nasm
ASFLAGS  := -f elf64
CC       := $(shell command -v x86_64-elf-gcc 2>/dev/null || echo gcc)
CFLAGS   := -ffreestanding -nostdlib -mno-red-zone -mcmodel=large -c
LD       := $(shell command -v x86_64-elf-ld 2>/dev/null || echo ld)
LDFLAGS  := -n -T linker.ld
OBJCOPY  := $(shell command -v x86_64-elf-objcopy 2>/dev/null || echo objcopy)
GDB      := $(shell command -v gdb-multiarch 2>/dev/null || echo gdb)

# --- Rust ---
CARGO_FLAGS := -Z build-std=core,alloc,compiler_builtins -Z json-target-spec
CARGO_TARGET := --target target.json --release

# --- Sources ---
AS_SRCS  := $(wildcard boot/*.asm)
AS_OBJS  := $(patsubst boot/%.asm, build/boot/%.o, $(AS_SRCS))
C_SRCS   := $(wildcard boot/*.c)
C_OBJS   := $(patsubst boot/%.c, build/boot/%.o, $(C_SRCS))
BOOT_OBJS := $(AS_OBJS) $(C_OBJS)

# --- QEMU ---
QEMU     := qemu-system-x86_64
DISK_IMG  := build/disk.img
QEMU_ARGS := -drive format=raw,file=$(DISK_IMG),if=ide,index=0 -cdrom $(ISO_FILE) -boot d -serial stdio -m 128M
QEMU_DBG  := $(QEMU_ARGS) -s -S -d int -no-reboot -no-shutdown

# ============================================================================
.PHONY: all bootloader kernel link iso run debug flat clean help

all: iso

# --- Directory setup ---
build/boot:
	mkdir -p build/boot

# --- Bootloader (ASM) ---
$(AS_OBJS): build/boot/%.o : boot/%.asm | build/boot
	@echo "[ASM]  $<"
	$(AS) $(ASFLAGS) $< -o $@

# --- Bootloader (C, optional) ---
$(C_OBJS): build/boot/%.o : boot/%.c | build/boot
	@echo "[CC]   $<"
	$(CC) $(CFLAGS) $< -o $@

bootloader: $(BOOT_OBJS)
	@echo "[BOOT] Bootloader objects ready."

# --- Kernel (Rust) ---
kernel:
	@echo "[RUST] Compiling kernel..."
	cargo build $(CARGO_FLAGS) $(CARGO_TARGET)

# --- Link ---
link: $(KERNEL_BIN)

$(KERNEL_BIN): kernel $(BOOT_OBJS) linker.ld
	@echo "[LD]   Linking kernel..."
	mkdir -p build
	$(LD) $(LDFLAGS) -o $(KERNEL_BIN) $(BOOT_OBJS) $(KERNEL_LIB)

# --- Flat binary (for baremetal boot without GRUB) ---
flat: $(KERNEL_BIN)
	@echo "[OBJCOPY] Generating flat binary..."
	$(OBJCOPY) -O binary $(KERNEL_BIN) $(KERNEL_FLAT)

# --- ISO ---
iso: $(KERNEL_BIN)
	@echo "[ISO]  Building bootable ISO..."
	mkdir -p build/isodir/boot/grub
	cp $(KERNEL_BIN) build/isodir/boot/kernel.bin
	echo -e 'serial --unit=0 --speed=115200\nterminal_input console serial\nterminal_output console serial\nset timeout=0\nset default=0\nmenuentry "AtomicOS" {\n  echo "Loading AtomicOS..."\n  multiboot2 /boot/kernel.bin\n  echo "Booting..."\n  boot\n}' > build/isodir/boot/grub/grub.cfg
	grub-mkrescue -o $(ISO_FILE) build/isodir

# --- Run in QEMU ---
run: iso
	@echo "[QEMU] Booting AtomicOS..."
	@test -f $(DISK_IMG) || (echo "[DISK] Creating 16MB disk image..." && dd if=/dev/zero of=$(DISK_IMG) bs=1M count=16 2>/dev/null)
	$(QEMU) $(QEMU_ARGS)

# --- Debug with GDB ---
debug: iso
	@echo "[QEMU] Booting in debug mode (waiting for GDB on :1234)..."
	@echo "       Run: $(GDB) build/kernel.bin -ex 'target remote :1234'"
	$(QEMU) $(QEMU_DBG)

# --- Clean ---
clean:
	@echo "[CLEAN] Removing build artifacts..."
	cargo clean
	rm -rf build

# --- Help ---
help:
	@echo ""
	@echo "  AtomicOS Build System"
	@echo "  ====================="
	@echo "  make            — Build ISO (default)"
	@echo "  make bootloader — Compile bootloader only (ASM + C)"
	@echo "  make kernel     — Compile Rust kernel only"
	@echo "  make link       — Link kernel + bootloader into ELF"
	@echo "  make flat       — Generate flat binary via objcopy"
	@echo "  make iso        — Generate bootable GRUB ISO"
	@echo "  make run        — Build and boot in QEMU"
	@echo "  make debug      — Boot in QEMU paused for GDB"
	@echo "  make clean      — Remove all build artifacts"
	@echo ""
	@echo "  Dependencies:"
	@echo "    nasm, ld, gcc (or x86_64-elf-*), grub-mkrescue,"
	@echo "    xorriso, qemu-system-x86_64, rust nightly"
	@echo ""
