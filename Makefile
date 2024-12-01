MODE ?= release
all:
	cd os && make all
kernel:
	cd os && make kernel
run:
	cd os && make run
