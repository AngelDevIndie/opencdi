use m68k::dasm::disassemble;
use m68k::CpuType;

fn main() {
    let rom = std::fs::read("../file utili/bios/cdi220b.rom").expect("bios");
    let base = 0x31AA0;
    // Driver table at 0x431AA0 + 0x40
    println!("=== POINTER DRIVER TABLE ===");
    for i in 0..6 {
        let off = u16::from_be_bytes([rom[base + 0x40 + i * 2], rom[base + 0x40 + i * 2 + 1]]);
        let names = ["Init", "Read", "Write", "GetStat", "SetStat", "Term"];
        println!("  {}: +0x{:04X} -> 0x{:06X}", names[i], off, 0x431AA0 + off as usize);
    }

    println!("\n=== POINTER GETSTAT at 0x431CFA ===");
    let mut pc = 0x31CFA;
    while pc < 0x31D40 {
        let w = u16::from_be_bytes([rom[pc], rom[pc + 1]]);
        let (s, sz) = disassemble((0x00400000 + pc) as u32, w, CpuType::SCC68070);
        let b: Vec<String> = rom[pc..pc + sz.max(2) as usize].iter().map(|x| format!("{:02X}", x)).collect();
        println!("  {:06X}: {:<16} {}", 0x00400000 + pc, b.join(" "), s);
        pc += 2;
    }
}
