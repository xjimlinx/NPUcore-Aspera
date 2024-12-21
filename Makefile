MODE ?= release
FS_MODE ?= fat32

all: clean
	cd os && make all

kernel:
	cd os && make kernel

run:
	cd os && make run

runsimple:
	cd os && make runsimple

change-kernel-only:
	cd os && make build && make runsimple

change-rootfs-only:
	cd os && make remake-qemu-flash-img && make runsimple

clean:
	cd os && make clean
.PHONY: all kernel run clean
