MODE ?= release
all:
	cd os && make all
kernel:
	cd os && make kernel MODE=release

