fn main() {
    let rom = std::fs::read("../file utili/bios/cdi220b.rom").expect("read rom");
    for i in 0..rom.len() - 4 {
        if rom[i] == 0x00 && rom[i+1] == 0x31 && (rom[i+2] & 0xF0) == 0x10 {
            println!("ROM offset 0x{:06X} (Addr 0x{:06X}): ref to 0x{:02X}{:02X}{:02X}{:02X}",
                i, i + 0x400000, rom[i], rom[i+1], rom[i+2], rom[i+3]);
        }
    }
}
