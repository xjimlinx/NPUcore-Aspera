# run: 清除编译结果，重新编译，运行
# all: 直接编译，并把.bin内核拷贝到根目录（适配大赛要求）
# gdb: 只运行gdb（需要先通过make run来编译）
# clean: 清除编译结果

# could also be riscv64
ARCH := loongarch64

TARGET := loongarch64-unknown-linux-gnu

MODE := debug
FS_MODE ?= ext4

# for riscv
RUSTSBI_ELF := ../rustsbi/target/riscv64gc-unknown-none-elf/release/rustsbi-k210
RUSTSBI_BIN := $(RUSTSBI_ELF).bin

KERNEL_ELF = target/$(TARGET)/$(MODE)/os
KERNEL_BIN = $(KERNEL_ELF).bin
KERNEL_UIMG = $(KERNEL_ELF).ui

BOARD ?= laqemu
LDBOARD = la2k1000

# 大写K转小写
ifeq ($(BOARD), 2K1000)
	BOARD = 2k1000
else ifeq ($(BOARD), K210)
	BOARD = k210
endif

# 块设备类型
BLOCK ?= mem

# Binutils
OBJCOPY := loongarch64-linux-gnu-objcopy
OBJDUMP := loongarch64-linux-gnu-objdump
READELF := loongarch64-linux-gnu-readelf

ifndef LOG
	LOG_OPTION := "log_off"
endif

ifeq ($(MODE), debug)
	LA_2k1000_DISABLE_EH_FRAME := -D EH_ENABLED
endif

IMG_DIR := ../fs-img-dir
IMG_NAME = rootfs-ubifs-ze.img
IMG := ${IMG_DIR}/$(IMG_NAME)
IMG_LN = $(shell readlink -f $(IMG_DIR))/$(IMG_NAME)

QEMU_2k1000_DIR=../util/qemu-2k1000/gz
QEMU_2k1000=$(QEMU_2k1000_DIR)/runqemu2k1000
U_IMG=$(IMG_DIR)/uImage

LA_DEBUGGER_SERIAL_PORT = $$(python3 -m serial.tools.list_ports 1A86:7523 -q | head -n 1)
LA_DEBUGGER_PORT_FREQ = $(LA_DEBUGGER_SERIAL_PORT) 115200
LA_2k1000_SERIAL_PORT = $$(python3 -m serial.tools.list_ports 067B:2303 -q | head -n 1)
LA_2k1000_PORT_FREQ = $(LA_2k1000_SERIAL_PORT) 115200
MINITERM_START_CMD=python3 -m serial.tools.miniterm --dtr 0 --rts 0 --filter direct

LA_ENTRY_POINT = 0x9000000090000000
LA_LOAD_ADDR = 0x9000000090000000

run: env update-usr run-inner

# 更新用户态程序
update-usr: fs-img

# 编译用户态程序
user: env
	@cd ../user && make rust-user BOARD=$(BOARD) MODE=$(MODE)

# 生成根文件系统镜像
fs-img: user
ifeq ($(BOARD),laqemu)
	@sudo rm -rf $(IMG)
	./buildfs.sh "$(IMG)" "laqemu" $(MODE) $(FS_MODE)
else
	./buildfs.sh "$(IMG)" 2k1000 $(MODE) $(FS_MODE)
endif

# 仅更新内核并运行
run-inner: uimage do-run

# 将内核转换为二进制文件
$(KERNEL_BIN): kernel
	@$(OBJCOPY) $(KERNEL_ELF) $@ --strip-all -O binary &
	@$(OBJDUMP) $(KERNEL_ELF) -SC > target/$(TARGET)/$(MODE)/asm_all.txt
	@$(READELF) -ash $(KERNEL_ELF) > target/$(TARGET)/$(MODE)/sec.txt &

# 编译内核
kernel:
	@echo Platform: $(BOARD)
    ifeq ($(MODE), debug)
		@cargo build --no-default-features --features "comp board_$(BOARD) block_$(BLOCK) $(LOG_OPTION)" --target $(TARGET)
    else
		@cargo build --no-default-features --release --features "comp board_$(BOARD) block_$(BLOCK) $(LOG_OPTION)"  --target $(TARGET)
    endif

# 更新内核
uimage: env $(KERNEL_BIN)
	../util/mkimage -A loongarch -O linux -T kernel -C none -a $(LA_LOAD_ADDR) -e $(LA_ENTRY_POINT) -n NPUcore+ -d $(KERNEL_BIN) $(KERNEL_UIMG)
	-@sudo rm $(U_IMG)
	@sudo cp -f $$(pwd)/target/$(TARGET)/$(MODE)/os.ui $(U_IMG)

do-run:
ifeq ($(BOARD), laqemu)
# 将镜像链接到指定目录
	-ln -sf $(IMG_LN) $(QEMU_2k1000_DIR)/$(IMG_NAME)
	@echo "========WARNING!========"
	@echo "下一个命令是修改后的runqemu2k1000脚本，其中任何潜在的和隐式的“当前工作目录”已被生成的脚本存储路径所替换。"
	@./run_script $(QEMU_2k1000)
else ifeq ($(BOARD), 2k1000)
	@./run_script $(MINITERM_START_CMD) $(LA_2k1000_PORT_FREQ)
endif

# 生成根文件系统镜像并编译内核
all: fs-img uimage mv
mv:
	mv $(KERNEL_BIN) ../kernel.bin

gdb:
ifeq ($(BOARD),laqemu)
	./run_script $(QEMU_2k1000) "-S"
else ifeq ($(BOARD), 2k1000)
	@./la_gdbserver minicom -D $(LA_DEBUGGER_PORT_FREQ)
endif

env: # 切换工具链
	-(rustup target list | grep "$(TARGET) (installed)") || rustup target add $(TARGET)
	if ! pacman -Q expect > /dev/null 2>&1; then sudo pacman -S --noconfirm expect; fi

clean:
	@cargo clean
	@cd ../user && make clean

.PHONY: user update gdb new-gdb monitor .FORCE
