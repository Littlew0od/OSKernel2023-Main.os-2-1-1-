# Building
TARGET := riscv64gc-unknown-none-elf
MODE := release
KERNEL_ELF := target/$(TARGET)/$(MODE)/os
KERNEL_BIN := $(KERNEL_ELF).bin
DISASM_TMP := target/$(TARGET)/$(MODE)/asm
# FS_IMG := ../user/target/$(TARGET)/$(MODE)/fs.img
APPS := ../user/src/bin/*
U_FAT32_DIR := ../fs-fuse
U_FAT32 := ${U_FAT32_DIR}/fat32.img
FINAL_FAT32 := ${U_FAT32_DIR}/sdcard.img
# BOARD
BOARD ?= qemu
SBI ?= rustsbi
BOOTLOADER := ../bootloader/$(SBI)-$(BOARD).bin
K210_BOOTLOADER_SIZE := 131072

# Building mode argument
ifeq ($(MODE), release)
	MODE_ARG := --release
endif

# KERNEL ENTRY
ifeq ($(BOARD), qemu)
	KERNEL_ENTRY_PA := 0x80200000
else ifeq ($(BOARD), k210)
	KERNEL_ENTRY_PA := 0x80020000
endif

# Run K210
K210-SERIALPORT	= /dev/ttyUSB1
K210-BURNER	= ../tools/kflash.py

# Binutils
OBJDUMP := rust-objdump --arch-name=riscv64
OBJCOPY := rust-objcopy --binary-architecture=riscv64

# Disassembly
DISASM ?= -x

# Run usertests or usershell
TEST ?=

build: env apps $(KERNEL_BIN)# fat32

switch-check:
ifeq ($(BOARD), qemu)
	(which last-qemu) || (rm -f last-k210 && touch last-qemu && make clean)
else ifeq ($(BOARD), k210)
	(which last-k210) || (rm -f last-qemu && touch last-k210 && make clean)
endif

#make user apps, it should finish before fat32
apps:$(APPS)
	cd ../user && make build 

# TEST=$(TEST)

# build fat32 for kernel
fat32:
	@cd ../user && make build TEST=$(TEST)
	@cd ../src
	@./buildfs.sh "$(U_FAT32_DIR)/fat32.img"

env:
	(rustup target list | grep "riscv64gc-unknown-none-elf (installed)") || rustup target add $(TARGET)
	# cargo install cargo-binutils
	rustup component add rust-src
	rustup component add llvm-tools-preview

sdcard: fs-img
	@echo "Are you sure write to $(SDCARD) ? [y/N] " && read ans && [ $${ans:-N} = y ]
	@sudo dd if=/dev/zero of=$(SDCARD) bs=1048576 count=32
	@sudo dd if=$(FS_IMG) of=$(SDCARD)

$(KERNEL_BIN): kernel
	@$(OBJCOPY) $(KERNEL_ELF) --strip-all -O binary $@

fs-img: $(APPS)
	@cd ../user && make build TEST=$(TEST)
	@rm -f $(FS_IMG)
	@cd ../easy-fs-fuse && cargo run --release -- -s ../user/src/bin/ -t ../user/target/riscv64gc-unknown-none-elf/release/

$(APPS):

kernel:
	@echo Platform: $(BOARD)
	@cp src/linker-$(BOARD).ld src/linker.ld
	@cargo build --release --features "board_$(BOARD)"
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

run-inner: build
ifeq ($(BOARD),qemu)
	@qemu-system-riscv64 \
		-machine virt \
		-nographic \
		-m 128M \
		-bios $(BOOTLOADER) \
		-device loader,file=$(KERNEL_BIN),addr=$(KERNEL_ENTRY_PA) \
		-drive file=$(FINAL_FAT32),if=none,format=raw,id=x0 \
        -device virtio-blk-device,drive=x0,bus=virtio-mmio-bus.0 \
		# -gdb tcp::1122 -S
else
# || (cd .. && git clone https://github.com/sipeed/kflash.py.git && mv kflash.py tools)
	which $(K210-BURNER) 
	@cp $(BOOTLOADER) $(BOOTLOADER).copy
	@dd if=$(KERNEL_BIN) of=$(BOOTLOADER).copy bs=$(K210_BOOTLOADER_SIZE) seek=1
	@mv $(BOOTLOADER).copy $(KERNEL_BIN)
	@sudo chmod 777 $(K210-SERIALPORT)
	python3 $(K210-BURNER) -p $(K210-SERIALPORT) -b 1500000 $(KERNEL_BIN)
	python3 -m serial.tools.miniterm --eol LF --dtr 0 --rts 0 --filter direct $(K210-SERIALPORT) 115200
endif

debug: build
	@qemu-system-riscv64 \
			-machine virt \
			-nographic \
			-m 128M \
			-bios $(BOOTLOADER) \
			-device loader,file=$(KERNEL_BIN),addr=$(KERNEL_ENTRY_PA) \
			-drive file=$(FINAL_FAT32),if=none,format=raw,id=x0 \
 	        -device virtio-blk-device,drive=x0,bus=virtio-mmio-bus.0 \
			-gdb tcp::1122 -S
#	 @tmux new-session -d \
# 		"qemu-system-riscv64 -machine virt -nographic -bios $(BOOTLOADER) -device loader,file=$(KERNEL_BIN),addr=$(KERNEL_ENTRY_PA) -s -S" && \
# 		tmux split-window -h "riscv64-unknown-elf-gdb -ex 'file $(KERNEL_ELF)' -ex 'set arch riscv:rv64' -ex 'target remote localhost:1234'" && \
# 		tmux -2 attach-session -d


gdbserver: build
	@qemu-system-riscv64 -machine virt -nographic -bios $(BOOTLOADER) -device loader,file=$(KERNEL_BIN),addr=$(KERNEL_ENTRY_PA) -s -S

gdbclient:
	@riscv64-unknown-elf-gdb -ex 'file $(KERNEL_ELF)' -ex 'set arch riscv:rv64' -ex 'target remote localhost:1122'

.PHONY: build env kernel clean disasm disasm-vim run-inner switch-check fs-img gdbserver gdbclient
