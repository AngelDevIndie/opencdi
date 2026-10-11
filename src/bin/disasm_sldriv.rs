fn main() {
    let rom = std::fs::read("../file utili/bios/cdi220b.rom").expect("read rom");
    let mut pc = 0x031AA0u32;
    let end = pc + 826;
    while pc < end {
        let op = u16::from_be_bytes([rom[pc as usize], rom[(pc+1) as usize]]);
        let (dis, len) = m68k::dasm::disassemble(pc + 0x400000, op, m68k::CpuType::SCC68070);
        let bytes: Vec<String> = (0..len).map(|k| format!("{:02X}", rom[(pc + k as u32) as usize])).collect();
        println!("0x{:06X}: {:<16} {:<24}", pc + 0x400000, bytes.join(" "), dis);
        pc += len as u32;
    }
}
