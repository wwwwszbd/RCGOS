# Building
# - build: build kernel (ELF + stripped BIN) and generate user filesystem image (fs.img)
# - run/debug: start QEMU with bootloader + kernel BIN + virtio-blk(fs.img)
TARGET := riscv64gc-unknown-none-elf
MODE := release
KERNEL_ELF := target/$(TARGET)/$(MODE)/os
KERNEL_BIN := $(KERNEL_ELF).bin
DISASM_TMP := target/$(TARGET)/$(MODE)/asm
FS_IMG := ../user/target/$(TARGET)/$(MODE)/fs.img
APPS := ../user/src/bin/*

# BOARD
BOARD := qemu
SBI ?= rustsbi
BOOTLOADER := ../bootloader/$(SBI)-$(BOARD).bin

# Building mode argument
ifeq ($(MODE), release)
	MODE_ARG := --release
endif

# KERNEL ENTRY
KERNEL_ENTRY_PA := 0x80200000

# Binutils
OBJDUMP := rust-objdump --arch-name=riscv64
OBJCOPY := rust-objcopy --binary-architecture=riscv64

# Disassembly
DISASM ?= -x
TEST ?=
# Ensure toolchain components are ready.
# - cargo-binutils provides rust-objcopy/rust-objdump wrappers
# - rust-src/llvm-tools-preview are required by some binutils backends
build: env $(KERNEL_BIN) fs-img
env:
	(rustup target list | grep "riscv64gc-unknown-none-elf (installed)") || rustup target add $(TARGET)
	(cargo install --list | grep -q "^cargo-binutils v") || cargo install cargo-binutils
	rustup component add rust-src
	rustup component add llvm-tools-preview
$(KERNEL_BIN): kernel
	@$(OBJCOPY) $(KERNEL_ELF) --strip-all -O binary $@
# Build user programs and pack them into fs.img.
# - -s: enumerate app names from user/src/bin/*.rs (strip extension)
# - -t: load built app ELFs from user/target/.../release/ (files without extension)
fs-img: $(APPS)
	@cd ../user && make build TEST=$(TEST)
	@rm -f $(FS_IMG)
	@cd ../easy-fs-fuse && cargo run --release -- -s ../user/src/bin/ -t ../user/target/riscv64gc-unknown-none-elf/release/

$(APPS):

kernel:
	@echo Platform: $(BOARD)
	@cp src/linker-$(BOARD).ld src/linker.ld
	@cargo build $(MODE_ARG)
	@rm src/linker.ld
clean:
	@cargo clean


disasm: kernel
	@$(OBJDUMP) $(DISASM) $(KERNEL_ELF) | less

disasm-vim: kernel
	@$(OBJDUMP) $(DISASM) $(KERNEL_ELF) > $(DISASM_TMP)
	@vim $(DISASM_TMP)
	@rm $(DISASM_TMP)

run: run-inner

# QEMU boot chain:
# - -bios $(BOOTLOADER): RustSBI-based bootloader image
# - loader device: loads kernel BIN to KERNEL_ENTRY_PA (matches kernel linker script entry)
# - virtio-blk: exposes fs.img to the kernel as a block device (MMIO bus on QEMU virt machine)
QEMU_ARGS := -machine virt \
			 -nographic \
			 -bios $(BOOTLOADER) \
			 -device loader,file=$(KERNEL_BIN),addr=$(KERNEL_ENTRY_PA) \
			 -drive file=$(FS_IMG),if=none,format=raw,id=x0 \
			 -device virtio-blk-device,drive=x0,bus=virtio-mmio-bus.0

QEMU_NAME := qemu-system-riscv64
qemu-version-check:
	@sh scripts/qemu-ver-check.sh $(QEMU_NAME)

run-inner: qemu-version-check build
	@qemu-system-riscv64 $(QEMU_ARGS)

debug: qemu-version-check build
	@tmux new-session -d \
		"qemu-system-riscv64 $(QEMU_ARGS) -s -S" && \
		tmux split-window -h "riscv64-unknown-elf-gdb -ex 'file $(KERNEL_ELF)' -ex 'set arch riscv:rv64' -ex 'target remote localhost:1234'" && \
		tmux -2 attach-session -d

gdbserver: qemu-version-check build
	@qemu-system-riscv64 $(QEMU_ARGS) -s -S

gdbclient:
	@riscv64-unknown-elf-gdb -ex 'file $(KERNEL_ELF)' -ex 'set arch riscv:rv64' -ex 'target remote localhost:1234'

.PHONY: build env kernel clean disasm disasm-vim run-inner fs-img gdbserver gdbclient qemu-version-check
