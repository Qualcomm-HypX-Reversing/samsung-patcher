#![feature(cursor_remaining)]


use elf::{
    ElfBytes,
    endian::{ AnyEndian, EndianParse },
    abi::{ R_AARCH64_CALL26, R_AARCH64_JUMP26 },
    
};


use std::io::Cursor;
use std::io::Write;
use std::fs::OpenOptions;
use byteorder::{ReadBytesExt, LittleEndian, WriteBytesExt};


const FUNCTION_TO_APPLY_PATCH: &str = "el0_svc"; //this is the function we will be patching
const ARMD_MAGIC: u32 = 0x644d5241;

fn get_offset_of_vaddr<E: EndianParse>(elf: &ElfBytes<E>, vaddr: u64) -> u64 {
    for phdr in elf.segments().expect("No phdrs") {
        if phdr.p_vaddr <= vaddr && vaddr <= phdr.p_vaddr + phdr.p_filesz {
            return vaddr - phdr.p_vaddr + phdr.p_offset;
        }
    }

    return 0;
}

fn get_offset_of_symbol<E: EndianParse>(elf: &ElfBytes<E>, symbol: &str) -> u64 {
    let symtab = elf
        .symbol_table()
        .expect("Failed to parse symbol table")
        .expect("No symbol table"); //ok we have the symbol table

    let mut ret_addr: u64 = 0;

    for sym in symtab.0 {
        if symtab.1.get(sym.st_name as usize).expect("Could not get symbol name") == symbol {
            ret_addr = sym.st_value;
            break;
        }
    }

    return get_offset_of_vaddr(elf, ret_addr);
}

fn get_vaddr_of_symbol<E: EndianParse>(elf: &ElfBytes<E>, symbol: &str)-> u64{

    let symtab = elf
        .symbol_table()
        .expect("Failed to parse symbol table")
        .expect("No symbol table"); //ok we have the symbol table

    let mut ret_addr: u64 = 0;

    for sym in symtab.0 {
        if symtab.1.get(sym.st_name as usize).expect("Could not get symbol name") == symbol {
            ret_addr = sym.st_value;
            break;
        }
    }

    return ret_addr;
}

fn read_insn(text_data: &[u8], off: u64) -> u32 {
    let mut cursor = Cursor::new(text_data);
    cursor.set_position(off);
   return cursor.read_u32::<LittleEndian>().expect("Failed to read insn"); 
}


fn write_insn(text_data: &mut [u8], off: u64, insn: u32){
    let mut cursor = Cursor::new(text_data);
    cursor.set_position(off);
    cursor.write_u32::<LittleEndian>(insn).expect("Failed to write insn"); 

}

/*
The kernel header is described here: https://www.kernel.org/doc/Documentation/arm64/booting.txt

We look for "ARMd" and we subtract by 56 as the first byte is at an offset of 56
*/
fn write_bootable_kernel(text_data: &[u8]) {
    let mut cursor = Cursor::new(text_data);
    let mut val_u32 = cursor.read_u32::<LittleEndian>().expect("Failed to read insn"); 

    while val_u32 != ARMD_MAGIC {
        val_u32 = cursor.read_u32::<LittleEndian>().expect("Failed to read insn"); 
    }

    cursor.set_position(cursor.position()-60);

    let mut file = OpenOptions::new()
    .create(true) // To create a new file
    .write(true)
    .truncate(true)
    .open("patched_kernel").expect("Failed to open patched_kernel");

    file.write_all(cursor.remaining_slice()).expect("Failed to write patched_kernel");
    
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 3 {
        println!("Usage: ./samsung_patcher [vmlinux_file] [patch_file]");
        return;
    }

    let vmlinux_file_name = &args[1];

    let patch_file = &args[2];

    let mut vmlinux_file_data = std::fs
        ::read(std::path::PathBuf::from(vmlinux_file_name))
        .expect("Could not read vmlinux");

    let vmlinux = ElfBytes::<AnyEndian>
        ::minimal_parse(vmlinux_file_data.as_slice())
        .expect("Failed to parse ELF");

    let patch_file_data = std::fs
        ::read(std::path::PathBuf::from(patch_file))
        .expect("Could not read patch file");

    let patch_file = ElfBytes::<AnyEndian>
        ::minimal_parse(patch_file_data.as_slice())
        .expect("Failed to parse ELF");

    let mut patch_off = get_offset_of_symbol(&vmlinux, FUNCTION_TO_APPLY_PATCH); //get the file offset of where the symbol is
    let patch_vaddr = get_vaddr_of_symbol(&vmlinux, FUNCTION_TO_APPLY_PATCH);
    println!("func: {}, addr: {:#x}", FUNCTION_TO_APPLY_PATCH, patch_off);

    let patch_file_text_header = patch_file
        .section_header_by_name(".text")
        .expect("Failed to parse patch file")
        .expect("Failed to find the text section");

    let mut patch_file_text_data_vec =  Vec::new();
    patch_file_text_data_vec.extend_from_slice(patch_file.section_data(&patch_file_text_header).expect("Failed to get text data").0);

    let patch_file_text_data = patch_file_text_data_vec.as_mut_slice();
    
    

    let patch_file_relas_header = patch_file
        .section_header_by_name(".rela.text")
        .expect("Failed to parse patch file")
        .expect("Failed to find the text section");

    let patch_file_relas = patch_file
        .section_data_as_relas(&patch_file_relas_header)
        .expect("Section is not a rela section");

    let patch_file_symbols = patch_file
        .symbol_table()
        .expect("Failed parse patch file")
        .expect("Failed to parse symbol/string table of patch file");


    //apply relocations here
    for rela in patch_file_relas {
        if rela.r_type != R_AARCH64_CALL26 && rela.r_type != R_AARCH64_JUMP26 { //we only handle 26 bit jumps
            panic!("Rela that is not a call or a jump");
        }
        let symbol = patch_file_symbols.0.get(rela.r_sym as usize).expect("Could not find symbol");

        let symbol_name = patch_file_symbols.1
            .get(symbol.st_name as usize)
            .expect("Could not find symbol name");

        let sym_vaddr = get_vaddr_of_symbol(&vmlinux, symbol_name);

        if sym_vaddr == 0 {
            panic!("Could not find {} in kernel", symbol_name);
        }

        //actually patch the instruction 
        //this can underflow and that's ok as we need to find the difference.
        // We do the >> 2 because the immediate is offset/4 (https://developer.arm.com/documentation/ddi0596/2021-12/Base-Instructions/BL--Branch-with-Link-?lang=en)
        let addend = (sym_vaddr.overflowing_sub(patch_vaddr + rela.r_offset).0 >> 2) as u32; 
        
        let mut insn = read_insn(patch_file_text_data, rela.r_offset); //offset is an offset into the loaded executable segments
        println!("old insn: {:#x}, off: {:#x}, addend: {:#x}", insn, rela.r_offset, addend);

        insn &= !((1<<26)-1); 
        insn |= addend & ((1<<26)-1);


        println!("new insn: {:#x}, off: {:#x}", insn, rela.r_offset);

        write_insn(patch_file_text_data, rela.r_offset, insn);
        
    }

    //the patch_file_text should be ready to write to the vmlinux

    for byte in patch_file_text_data {
        vmlinux_file_data[patch_off as usize] = *byte;
        patch_off += 1;
    }

    let mut file = OpenOptions::new()
    .create(true) // To create a new file
    .write(true)
    .truncate(true)
    .open("patched_vmlinux").expect("Failed to open patched_vmlinux");

    file.write_all(vmlinux_file_data.as_slice()).expect("Failed to write patched_vmlinux");

    write_bootable_kernel(vmlinux_file_data.as_slice());


}
