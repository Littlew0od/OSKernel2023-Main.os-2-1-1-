DOCKER_NAME ?= rcore-tutorial-v3
.PHONY: docker build_docker
	
docker:
	docker run --rm -it -v ${PWD}:/mnt -w /mnt ${DOCKER_NAME} bash

build_docker: 
	docker build -t ${DOCKER_NAME} .

fmt:
	cd easy-fs; cargo fmt; cd ../easy-fs-fuse cargo fmt; cd ../os ; cargo fmt; cd ../user; cargo fmt; cd ..

all:
	cd ./os && make build
	cp -f ./os/target/riscv64gc-unknown-none-elf/release/os.bin ./kernel-qemu
	cp -f ./bootloader/rustsbi-qemu.bin ./sbi-qemu