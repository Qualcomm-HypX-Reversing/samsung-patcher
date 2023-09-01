all:
	aarch64-linux-gnu-as -g patch.S -c -o patch.o

disassemble:
	llvm-objdump --arch=aarch64 --disassemble patch.o
