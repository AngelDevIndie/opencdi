use std::fs;
use m68k::dasm::disassemble;
use m68k::CpuType;

fn main() {
    let rom = fs::read("../file utili/bios/cdi220b.rom").expect("bios");
    let base = 0x31E5A;
    let targets = [
        ("0xF0", 0x05DA + 2 + 0x32),
        ("0xF1", 0x0642),
        ("0xF2", 0x05EA + 2 + 0x0092),
        ("0xF3", 0x05F4 + 2 + 0x00B0),
        ("0xF4", 0x05FE + 2 + 0x00CE),
        ("0xF6", 0x0602 + 2 + 0x00EC),
        ("0xF5", 0x060C + 2 + 0x010A),
    ];
    // Driver table at 0x431E5A + 0x40
    println!("=== SLDRIV DRIVER TABLE ===");
    for i in 0..6 {
        let off = u16::from_be_bytes([rom[base + 0x40 + i * 2], rom[base + 0x40 + i * 2 + 1]]);
        let names = ["Init", "Read", "Write", "GetStat", "SetStat", "Term"];
        println!("  {}: +0x{:04X} -> 0x{:06X}", names[i], off, 0x431E5A + off as usize);
    }

    println!("\n=== SLDRIV INTERRUPT/POINTER HANDLER at 0x432480 ===");
    let mut pc = 0x32480;
    while pc < 0x32580 {
        let w = u16::from_be_bytes([rom[pc], rom[pc + 1]]);
        let (s, sz) = disassemble((0x00400000 + pc) as u32, w, CpuType::SCC68070);
        let b: Vec<String> = rom[pc..pc + sz.max(2) as usize].iter().map(|x| format!("{:02X}", x)).collect();
        println!("  {:06X}: {:<16} {}", 0x00400000 + pc, b.join(" "), s);
        pc += 2;
    }

    println!("\n=== VIDEO from 0x41EB40 to 0x41EC00 ===");
    let mut pc = 0x41EB40 - 0x400000;
    while pc < 0x41EC00 - 0x400000 {
        let w = u16::from_be_bytes([rom[pc], rom[pc + 1]]);
        let (s, sz) = disassemble((0x00400000 + pc) as u32, w, CpuType::SCC68070);
        let b: Vec<String> = rom[pc..pc + sz.max(2) as usize].iter().map(|x| format!("{:02X}", x)).collect();
        println!("  {:06X}: {:<16} {}", 0x00400000 + pc, b.join(" "), s);
        pc += sz.max(2) as usize;
    }

    println!("\n=== VIDEO at 0x41EBE0 ===");
    let mut pc = 0x41EBE0 - 0x400000;
    while pc < 0x41EC30 - 0x400000 {
        let w = u16::from_be_bytes([rom[pc], rom[pc + 1]]);
        let (s, sz) = disassemble((0x00400000 + pc) as u32, w, CpuType::SCC68070);
        let b: Vec<String> = rom[pc..pc + sz.max(2) as usize].iter().map(|x| format!("{:02X}", x)).collect();
        println!("  {:06X}: {:<16} {}", 0x00400000 + pc, b.join(" "), s);
        pc += sz.max(2) as usize;
    }
}
