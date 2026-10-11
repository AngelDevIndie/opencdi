use std::fs;

fn inspect_descriptor(rom: &[u8], offset: usize, desc_name: &str) {
    println!("\n=== Device Descriptor: {} at offset 0x{:06X} (Addr 0x{:06X}) ===", desc_name, offset, 0x400000 + offset);
    let name_off = u32::from_be_bytes([rom[offset+12], rom[offset+13], rom[offset+14], rom[offset+15]]) as usize;
    let name = rom[offset + name_off..].iter().take_while(|&&b| b != 0).map(|&b| b as char).collect::<String>();
    println!("  Module Name: '{}'", name);
    println!("  Header bytes: {:02X?}", &rom[offset..offset + 32]);
    println!("  Body bytes (32..96): {:02X?}", &rom[offset+32..offset + 96]);
}
fn main() {
    let rom = fs::read("../file utili/bios/cdi220b.rom").expect("Failed to read ROM");
    let rom_base = 0x400000;
    let dvc_rom = fs::read("../file utili/dvc/vmpega.rom").expect("Failed to read DVC ROM");
    let mut p = 0x011BD0;
    println!("=== Disassembly of fmvdrv around 0x011BDA ===");
    while p < 0x011C50 {
        let opcode = ((dvc_rom[p] as u16) << 8) | (dvc_rom[p + 1] as u16);
        let (text, len) = m68k::dasm::disassemble(p as u32, opcode, m68k::CpuType::SCC68070);
        let l = len.max(2);
        print!("  0x{:06X}: ", p);
        for i in 0..(l as usize / 2) {
            print!("{:02X}{:02X} ", dvc_rom[p + i * 2], dvc_rom[p + i * 2 + 1]);
        }
        println!("  {}", text);
        p += l as usize;
    }
    return;
}
