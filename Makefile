MODE ?= release
all:
	cd os && make all
kernel:
	cd os && make kernel
run:
	cd os && make run
clean:
	cd os && make clean

.PHONY: all kernel run clean
