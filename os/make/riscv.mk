# Building
TARGET := riscv64gc-unknown-none-elf
MODE := release
KERNEL_ELF := target/$(TARGET)/$(MODE)/os
KERNEL_BIN := $(KERNEL_ELF).bin
DISASM_TMP := target/$(TARGET)/$(MODE)/asm
BLK_MODE := mem
FS_MODE ?= ext4
ROOTFS_IMG_NAME = rootfs-rv.img
ROOTFS_IMG_DIR := ../fs-img-dir
CORE_NUM := 1
ifeq ($(BOARD), vf2)
	ROOTFS_IMG := /dev/sdc
else
	ROOTFS_IMG := ${ROOTFS_IMG_DIR}/${ROOTFS_IMG_NAME}
endif

APPS := ../user/src/bin/*

# BOARD
BOARD ?= rvqemu
SBI ?= opensbi-1.0
ifeq ($(BOARD), rvqemu)
	ifeq ($(SBI), rustsbi)
		BOOTLOADER := ../bootloader/$(SBI)-$(BOARD).bin
	else ifeq ($(SBI), default)
		BOOTLOADER := default
	else
		BOOTLOADER := ../bootloader/fw_payload.bin
	endif
else ifeq ($(BOARD), vf2)
	BOOTLOADER := ../bootloader/rustsbi-$(BOARD).bin
endif

ifndef LOG
	LOG_OPTION := "log_off"
endif

# KERNEL ENTRY
ifeq ($(BOARD), rvqemu)
	KERNEL_ENTRY_PA := 0x80200000
else ifeq ($(BOARD), vf2)
	KERNEL_ENTRY_PA := 0x80020000
endif

# Binutils
OBJDUMP := rust-objdump --arch-name=riscv64
OBJCOPY := rust-objcopy --binary-architecture=riscv64

# Disassembly
DISASM ?= -x

build: env $(KERNEL_BIN)

env:
	(rustup target list | grep "riscv64gc-unknown-none-elf (installed)") || rustup target add $(TARGET)
	rustup component add rust-src
	rustup component add llvm-tools-preview

# build all user programs
user:
	@cd ../user && make rust-user BOARD=$(BOARD) MODE=$(MODE)

$(KERNEL_BIN): kernel
	@$(OBJCOPY) $(KERNEL_ELF) --strip-all -O binary $@

$(APPS):

fs-img: user
	./buildfs.sh "$(ROOTFS_IMG)" "rvqemu" $(MODE) $(FS_MODE)

kernel:
	@echo Platform: $(BOARD)
	@cp src/hal/arch/riscv/linker-$(BOARD).ld src/hal/arch/riscv/linker.ld
    ifeq ($(MODE), debug)
		@cargo build --features "board_$(BOARD) $(LOG_OPTION) block_$(BLK_MODE) oom_handler" --no-default-features
    else
		@cargo build --release --features "board_$(BOARD) $(LOG_OPTION) block_$(BLK_MODE) oom_handler" --no-default-features
    endif
	@rm src/hal/arch/riscv/linker.ld

clean:
	@cargo clean

run: build
ifeq ($(BOARD), rvqemu)
	@qemu-system-riscv64 \
  		-machine virt \
  		-nographic \
  		-bios $(BOOTLOADER) \
  		-device loader,file=$(KERNEL_BIN),addr=$(KERNEL_ENTRY_PA) \
  		-drive if=none,file=$(ROOTFS_IMG),format=raw,id=x0 \
        -device virtio-blk-device,drive=x0,bus=virtio-mmio-bus.0\
  		-m 1024 \
  		-smp threads=$(CORE_NUM)
endif

monitor:
	riscv64-unknown-elf-gdb -ex 'file target/riscv64gc-unknown-none-elf/debug/os' -ex 'set arch riscv:rv64' -ex 'target remote localhost:1234'

gdb:
	@qemu-system-riscv64 -machine virt -nographic -bios $(BOOTLOADER) -device loader,\
	file=target/riscv64gc-unknown-none-elf/debug/os,addr=0x80200000 -drive \
	file=$(ROOTFS_IMG),if=none,format=raw,id=x0 \
	-device virtio-blk-device,drive=x0,bus=virtio-mmio-bus.0 \
	-m 1024 \
	-smp threads=$(CORE_NUM) -S -s | tee qemu.log

runsimple:
	@qemu-system-riscv64 \
		-machine virt \
		-nographic \
		-bios $(BOOTLOADER) \
		-device loader,file=$(KERNEL_BIN),addr=$(KERNEL_ENTRY_PA) \
		-drive file=$(ROOTFS_IMG),if=none,format=raw,id=x0 \
		-m 1024 \
        -device virtio-blk-device,drive=x0,bus=virtio-mmio-bus.0\
		-smp threads=$(CORE_NUM)

.PHONY: user
