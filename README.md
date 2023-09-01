# Samsung Patcher

This piece of software allows you to patch a samsung kernel binary WITHOUT SOURCES. 

The binary takes in two files a kernel ELF (This should be produced via the `vmlinux-to-elf` tool from the bootimg) as well as a patch object file. 

The patch object file used in this case was `kernel_patch/patch.o`. This file can be generated via running `make all` in `kernel_patch`. Note the `aarch64-linux-gnu-*` toolchain is required. 

This patch (Found in `patch.S`) modifies `el0_svc` to provide full kernel read, write, and execute. 

To choose what function the patch is applied to modify `FUNCTION_TO_APPLY_PATCH` in the `main.rs`.


## Using this tool

The kernel ELF can be obtained by obtaining the `boot.img` file (Usually included in firmware upgrades) and running `unpack_bootimg.py` (Found here: https://android.googlesource.com/platform/system/tools/mkbootimg/+/refs/heads/main). Make sure to run `unpack_bootimg.py` with `--format mkbootimg` flag as that will output the arguments needed to roll back the kernel into a new `boot.img` file.

Once you have run `unpack_bootimg.py` the output folder should have a file called `kernel`. That is your kernel image.

From here, download `vmlinux-to-elf` (Found here: https://github.com/marin-m/vmlinux-to-elf) and run `vmlinux-to-elf` on the kernel binary. This should output a ELF file. 

From here run `cargo run [elf file] [patch file]` to obtain a new kernel image. There will be two output files. The first is `patched_vmlinux` and the second will be `patched_kernel`. The `patched_vmlinux` can be used for debugging purposes to see if your patch actually applied properly and `patched_kernel` can be used to roll back into a boot image.

To roll it back into a boot image run `mkbootimg.py` along with the arguments you got from `--format mkbootimg` (Make sure to replace the kernel flag with `patched_kernel` as well as supply an output file). 


## Internals

The core of this tool is that `vmlinux-to-elf` simply prepends a ELF header to the file as well as appends symbol information. So, we can simply take our ELF, apply the patches, and chop off the ELF header to get our boot image back. This resulting image is perfectly bootable (The phone does not care about the symbol info at the end).