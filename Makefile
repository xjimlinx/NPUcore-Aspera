MODE ?= release
FS_MODE ?= fat32

QEMU_TAR := qemu-2k1000-static.20240526.tar.xz
QEMU_URL := https://gitlab.educg.net/wangmingjian/os-contest-2024-image/-/raw/master/$(QEMU_TAR)
QEMU_DIR := util/qemu-2k1000/tmp
QEMU_TAR_PATH := $(QEMU_DIR)/$(QEMU_TAR)

COLOR_MAIN = \033[1;34m  # 亮蓝色
COLOR_ACCENT = \033[1;37m # 亮白色
COLOR_RESET = \033[0m

all: clean
	cd os && make all

kernel:
	cd os && make kernel

run: print-logo
	cd os && make run

runsimple:
	cd os && make runsimple

change-kernel-only:
	cd os && make build && make runsimple

print-logo:
	@echo "${COLOR_ACCENT}"
	@echo "Welcome to NPUCore Project Aspera🚀"
	@echo "${COLOR_MAIN}"
	@echo "                                                                            " 
	@echo "  ________    ________    ________    _______     ________    ________      " 
	@echo " |\   __  \  |\   ____\  |\   __  \  |\  ___ \   |\   __  \  |\   __  \     " 
	@echo " \ \  \|\  \ \ \  \___|_ \ \  \|\  \ \ \   __/|  \ \  \|\  \ \ \  \|\  \    " 
	@echo "  \ \   __  \ \ \_____  \ \ \   ____\ \ \  \_|/__ \ \   _  _\ \ \   __  \   " 
	@echo "   \ \  \ \  \ \|____|\  \ \ \  \___|  \ \  \_|\ \ \ \  \\  \| \ \  \ \  \  " 
	@echo "    \ \__\ \__\  ____\_\  \ \ \__\      \ \_______\ \ \__\\ _\  \ \__\ \__\ " 
	@echo "     \|__|\|__| |\_________\ \|__|       \|_______|  \|__|\|__|  \|__|\|__| " 
	@echo "                \|_________|                                                " 
	@echo "                                                                            " 
	@echo "                                                                            " 
	@echo "${COLOR_RESET}"                                                                                                                   

.PHONY: qemu-download
qemu-download: $(QEMU_DIR)/.extracted
	chmod +x util/mkimage
	chmod +x util/qemu-2k1000/gz/runqemu2k1000
	chmod +x $(QEMU_DIR)/qemu/bin/qemu-system-loongarch64
	mkdir -p fs-img-dir
	sudo chmod 777 fs-img-dir/

$(QEMU_DIR)/.extracted: $(QEMU_TAR_PATH)
	@echo "Extracting $(QEMU_TAR)..."
	cd $(QEMU_DIR) && tar xavf $(QEMU_TAR)
	rm -rf $(QEMU_DIR)/qemu/2k1000 \
		$(QEMU_DIR)/qemu/runqemu \
		$(QEMU_DIR)/qemu/README.md \
		$(QEMU_DIR)/qemu/include \
		$(QEMU_DIR)/qemu/var
	@touch $@

$(QEMU_TAR_PATH):
	@mkdir -p $(QEMU_DIR)
	@if [ -f $@ ]; then \
		if ! tar tf $@ >/dev/null 2>&1; then \
			echo "File $@ is corrupted. Deleting and re-downloading..."; \
			rm -f $@; \
			wget -q $(QEMU_URL) -P $(QEMU_DIR); \
		fi; \
	else \
		echo "Downloading $(QEMU_TAR)..."; \
		wget -q $(QEMU_URL) -P $(QEMU_DIR); \
	fi
	@if ! tar tf $@ >/dev/null 2>&1; then \
		echo "Download failed, please check network connection"; \
		exit 1; \
	fi

clean: print-logo
	cd os && make clean
.PHONY: all kernel run clean
