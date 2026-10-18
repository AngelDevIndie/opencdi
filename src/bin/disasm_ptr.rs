use m68k::dasm::disassemble;
use m68k::CpuType;

fn main() {
    let rom = std::fs::read("../file utili/bios/cdi220b.rom").expect("bios");
    let ptr_base = 0x31AA0usize; // 0x431AA0 in memory
    
    // Module header
    let exec_off = u16::from_be_bytes([rom[ptr_base + 0x14], rom[ptr_base + 0x15]]) as usize;
    println!("Module exec offset: +0x{:04X} -> 0x{:06X}", exec_off, 0x431AA0 + exec_off);
    let table_base = ptr_base + exec_off;
    let names = ["Init", "Read", "Write", "GetStat", "SetStat", "Term"];
    for i in 0..6 {
        let entry_off = u16::from_be_bytes([rom[table_base + i * 2], rom[table_base + i * 2 + 1]]) as usize;
        let entry_addr = 0x431AA0 + entry_off;
        println!("Entry {}: +0x{:04X} (0x{:06X})", names[i], entry_off, entry_addr);
    }

    println!("\n=== DISASSEMBLING 0x431F50..0x432020 (Pointer Type & Init) ===");
    let mut pc = 0x431F50usize;
    while pc < 0x432020 {
        let off = pc - 0x400000;
        let w = u16::from_be_bytes([rom[off], rom[off + 1]]);
        let (s, sz) = disassemble(pc as u32, w, CpuType::SCC68070);
        let len = sz.max(2) as usize;
        let b: Vec<String> = rom[off..off + len].iter().map(|x| format!("{:02X}", x)).collect();
        println!("  0x{:06X}: {:<16} {}", pc, b.join(" "), s);
        pc += 2;
    }
}

