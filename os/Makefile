TARGET := loongarch64-unknown-linux-gnu
MODE ?= release
CORE_NUM := 1

KERNEL_ELF = target/$(TARGET)/$(MODE)/os
KERNEL_BIN = $(KERNEL_ELF).bin
KERNEL_UIMG = $(KERNEL_ELF).ui

ARCH=${shell echo ${TARGET}|cut -d- -f1 | grep -o "^[a-zA-Z]\+[[:digit:]]*"}
# BOARD
BOARD ?= laqemu
LDBOARD = la2k500

SDDEV := $(shell lsblk -do NAME | grep sd | tail -1)
SDCARD := /dev/$(SDDEV)

# Disassembly
DISASM ?= -x

ifeq ($(MODE), debug)
ifeq ($(BOARD), 2k500)
	LA_2K500_DISABLE_EH_FRAME := -D EH_ENABLED
endif
endif

ifeq ($(BOARD), 2K500)
	BOARD = 2k500
endif

# Binutils
OBJCOPY := loongarch64-unknown-linux-gnu-objcopy
OBJDUMP := loongarch64-unknown-linux-gnu-objdump
READELF := loongarch64-unknown-linux-gnu-readelf

ifndef LOG
	LOG_OPTION := "log_off"
endif

# Filesystem
IMG_DIR := ../fs-img-dir
FS_MODE ?= fat32

FAT32_IMG_LA = rootfs-ubifs-ze.img
FAT32_IMG_NAME ?= $(FAT32_IMG_LA)
U_FAT32_DIR := $(IMG_DIR)
U_FAT32 := ${U_FAT32_DIR}/$(FAT32_IMG_NAME)
FAT32_2K500_LN = $(shell readlink -f $(U_FAT32_DIR))/$(FAT32_IMG_LA)

EXT4_IMG_LA = rootfs-ubifs-ze.img
EXT4_IMG_NAME ?= $(EXT4_IMG_LA)
U_EXT4_DIR := $(IMG_DIR)
U_EXT4 := ${U_EXT4_DIR}/$(EXT4_IMG_NAME)
EXT4_2K500_LN = $(shell readlink -f $(U_EXT4_DIR))/$(EXT4_IMG_LA)

QEMU_2K500_DIR=../util/qemu-2k500/gz
QEMU_2K500=$(QEMU_2K500_DIR)/runqemu2k500
IMG_DIR=../fs-img-dir
U_IMG=$(IMG_DIR)/uImage

LA_DEBUGGER_SERIAL_PORT = $$(python3 -m serial.tools.list_ports 1A86:7523 -q | head -n 1)
LA_DEBUGGER_PORT_FREQ = $(LA_DEBUGGER_SERIAL_PORT) 115200
LA_2K500_SERIAL_PORT = $$(python3 -m serial.tools.list_ports 067B:2303 -q | head -n 1)
LA_2K500_PORT_FREQ = $(LA_2K500_SERIAL_PORT) 115200
MINITERM_START_CMD=python3 -m serial.tools.miniterm --dtr 0 --rts 0 --filter direct

LA_ENTRY_POINT = 0x9000000000000000
LA_LOAD_ADDR = 0x9000000000000000

RUST_LA_TOOLCHAIN = $(shell rustup show | grep "\(la\|loongarch\)\(64\)\?" |cut -d' ' -f1|head -n 1)


all: clean remake-qemu-flash-img run
build: env $(KERNEL_BIN)
env: # switch-toolchain
	-(rustup target list | grep "$(TARGET) (installed)") || rustup target add $(TARGET)
	if [ "$(dpkg --list| grep "ii[[:space:]]*expect")"="" ];then true;else sudo pacman install expect;fi
ifneq ($(ARCH), loongarch64)
#	-cargo install cargo-binutils --vers ~0.2
endif
#	-rustup component add rust-src
#	-rustup component add llvm-tools-preview

# 构建用户态程序
user: c-user rust-user
c-user:
	@cd ../user && make c-user BOARD=$(BOARD) MODE=$(MODE)
rust-user: env
	@cd ../user && make rust-user BOARD=$(BOARD) MODE=$(MODE)

# dev/zero永远输出0
sdcard:
ifeq ($(findstring WSL, $(shell uname -a)), )
		@echo "Are you sure write to $(SDCARD) ? \nYou may also specify another drive with the option \`SDCARD=/dev/<drive>\` [y/N] " && read ans && [ $${ans:-N} = y ]
		./buildfs.sh "$(SDCARD)" "qemu" $(MODE)
else
		@echo "Are you sure write to E:/ ? [y/N] " && read ans && [ $${ans:-N} = y ]
		@sudo mount $(U_FAT32_DIR)/$(FAT32_IMG_NAME) $(U_FAT32_DIR)/fs
		@powershell.exe 'cd $(U_FAT32_DIR); copy -Recurse -path "$$(pwd)/fs/*" -destination "E:/" -force'
		@sudo umount $(U_FAT32_DIR)/fs
endif

# 此步会将kernel链接到uImage
$(KERNEL_BIN): kernel
	@$(OBJCOPY) $(KERNEL_ELF) $@ --strip-all -O binary &
	@$(OBJDUMP) $(KERNEL_ELF) -SC > target/$(TARGET)/$(MODE)/asm_all.txt
	@$(READELF) -ash $(KERNEL_ELF) > target/$(TARGET)/$(MODE)/sec.txt &
ifeq ($(ARCH), loongarch64)
	../util/mkimage -A loongarch -O linux -T kernel -C none -a $(LA_LOAD_ADDR) -e $(LA_ENTRY_POINT) -n NPUcore+ -d $(KERNEL_BIN) $(KERNEL_UIMG)
	-@sudo rm $(U_IMG)
	@cp -f $$(pwd)/target/$(TARGET)/$(MODE)/os.ui $(U_IMG)
endif
	-@rm src/linker.ld

fs-img:
ifeq ($(BOARD), 2k500)
ifeq ($(FS_MODE), fat32)
	@./buildfs.sh "$(U_FAT32_DIR)/$(FAT32_IMG_NAME)" "la2k500" $(MODE) fat32
else ifeq ($(FS_MODE), ext4)
	@./buildfs.sh "$(U_EXT4_DIR)/$(EXT4_IMG_NAME)" "la2k500" $(MODE) ext4
else
	@echo "Invalid FS_MODE"
	exit 1
endif
else ifeq ($(BOARD), laqemu)
ifeq ($(FS_MODE), fat32)
	@./buildfs.sh "$(U_FAT32_DIR)/$(FAT32_IMG_NAME)" "laqemu" $(MODE) fat32
else ifeq ($(FS_MODE), ext4)
	@./buildfs.sh "$(U_EXT4_DIR)/$(EXT4_IMG_NAME)" "laqemu" $(MODE) ext4
else
	@echo "Invalid FS_MODE"
	exit 1
endif
endif

remake-qemu-flash-img: user fs-img .WAIT qemu-flash-rootfs-img

remake-qemu-flash-img-without-user: fs-img qemu-flash-rootfs-img

qemu-flash-rootfs-img:
ifeq ($(BOARD), 2k500)
	@./la_fs $(MINITERM_START_CMD) $(LA_2K500_PORT_FREQ)
else ifeq ($(BOARD), laqemu)
	@./la_fs $(QEMU_2K500)
	@echo QEMU_2K500
endif

src-linker.ld: .FORCE
	-@rm src/linker.ld
	@cpp -P $(LA_2K500_DISABLE_EH_FRAME) -D $(LDBOARD) src/linker.in.ld -o src/linker.ld

kernel: src-linker.ld
	@echo Platform: $(BOARD)
ifeq ($(MODE), debug)
	@cargo build --no-default-features --features "board_$(BOARD) $(LOG_OPTION)" --target $(TARGET)
else
	@cargo build --no-default-features --release --features "board_$(BOARD) $(LOG_OPTION)"  --target $(TARGET)
endif

clean:
	@cargo clean
	@sudo rm -rf ../fs-img-dir
	@sudo rm -f src/linker.ld
	-@cd ../user && make clean

run: env run-inner

run-inner: build do-run

runsimple: do-run

do-run:
# 此步会将rootfs文件夹链接到qemu-2k500/gz下
ifeq ($(BOARD), laqemu)
ifeq ($(FS_MODE), fat32)
	ln -sf $(FAT32_2K500_LN) $(QEMU_2K500_DIR)/$(FAT32_IMG_LA)
else ifeq ($(FS_MODE), ext4)
	ln -sf $(EXT4_2K500_LN) $(QEMU_2K500_DIR)/$(EXT4_IMG_LA)
else
	@echo "Invalid FS_MODE"
	exit 1
endif
	@echo "========WARNING!========"
	@echo "The next command is expecting a modified runqemu2k500 script where any potential and implicit \"current working directory\" has been replaced by a generated script storage path."
	@./run_script $(QEMU_2K500)
else ifeq ($(BOARD), 2k500)
	@./run_script $(MINITERM_START_CMD) $(LA_2K500_PORT_FREQ)
endif

new-gdb: build gdb
gdb:
ifeq ($(BOARD),laqemu)
	./run_script $(QEMU_2K500) "-S"
else ifeq ($(BOARD), 2k500)
	@./la_gdbserver minicom -D $(LA_DEBUGGER_PORT_FREQ)
endif

ch-default:
	@DEFAULT_STR=$$(grep -o "default[[:space:]]*=[[:space:]]\[[[:space:]]*\"board_.\+\"[[:space:]]*\]" Cargo.toml);
	@if [ $$(echo "$$DEFAULT_STR"| grep "qemu") ]; then echo "found"; else echo "not found"; fi;

# test_addr:
# 	@echo "ENT" $(LA_ENTRY_POINT) "\nld " $(LA_LOAD_ADDR)

.PHONY: user update gdb new-gdb monitor .FORCE
