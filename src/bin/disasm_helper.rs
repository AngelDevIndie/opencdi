use std::fs;

fn main() {
    let rom = fs::read("file utili/bios/cdi220b.rom")
        .or_else(|_| fs::read("../file utili/bios/cdi220b.rom"))
        .expect("Cannot read ROM");

    let args: Vec<String> = std::env::args().collect();
    let start = args.get(1).and_then(|s| u32::from_str_radix(s.trim_start_matches("0x"), 16).ok()).unwrap_or(0x400F30);
    let end = args.get(2).and_then(|s| u32::from_str_radix(s.trim_start_matches("0x"), 16).ok()).unwrap_or(start + 0x40);

    let mut pc = start;
    while pc < end {
        let offset = (pc - 0x400000) as usize;
        let op = u16::from_be_bytes([rom[offset], rom[offset + 1]]);
        let (dis, len) = m68k::dasm::disassemble(pc, op, m68k::CpuType::SCC68070);
        let bytes: Vec<String> = (0..len).map(|i| format!("{:02X}", rom[offset + i as usize])).collect();
        println!("0x{:06X}: {:<16} {:<32}", pc, bytes.join(" "), dis);
        pc += len as u32;
    }
}
