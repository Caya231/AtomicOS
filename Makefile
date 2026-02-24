KERNEL_FILE := target/target/release/libatomic_os.a
KERNEL_BIN := build/kernel.bin
ISO_FILE := build/atomic_os.iso

AS := nasm
ASFLAGS := -f elf64
LD := $(shell command -v x86_64-elf-ld 2> /dev/null || echo ld)

AS_SRCS := $(wildcard boot/*.asm)
AS_OBJS := $(patsubst boot/%.asm, build/boot/%.o, $(AS_SRCS))

.PHONY: all kernel bootloader iso run clean

all: iso

build/boot:
	mkdir -p build/boot

$(AS_OBJS): build/boot/%.o : boot/%.asm | build/boot
	$(AS) $(ASFLAGS) $< -o $@

kernel:
	cargo build -Z build-std=core,compiler_builtins -Z json-target-spec --target target.json --release

$(KERNEL_BIN): kernel $(AS_OBJS) linker.ld
	mkdir -p build
	$(LD) -n -T linker.ld -o $(KERNEL_BIN) $(AS_OBJS) $(KERNEL_FILE)

iso: $(KERNEL_BIN)
	mkdir -p build/isodir/boot/grub
	cp $(KERNEL_BIN) build/isodir/boot/kernel.bin
	echo -e 'serial --unit=0 --speed=115200\nterminal_input console serial\nterminal_output console serial\nset timeout=0\nset default=0\nmenuentry "AtomicOS" {\n  echo "Loading AtomicOS..."\n  multiboot2 /boot/kernel.bin\n  echo "Booting..."\n  boot\n}' > build/isodir/boot/grub/grub.cfg
	grub-mkrescue -o $(ISO_FILE) build/isodir

run: iso
	qemu-system-x86_64 -cdrom $(ISO_FILE) -serial stdio

clean:
	cargo clean
	rm -rf build
