use std::fs;

fn main() {
    let rom_path = std::env::args().nth(1).unwrap_or_else(|| "../file utili/bios/cdi220b.rom".to_string());
    let rom = fs::read(&rom_path).expect("Failed to read ROM");
    println!("=== OS-9 Modules in {} ===", rom_path);

    let mut i = 0;
    while i + 16 < rom.len() {
        if rom[i] == 0x4A && rom[i + 1] == 0xFC {
            let size = u32::from_be_bytes([rom[i + 4], rom[i + 5], rom[i + 6], rom[i + 7]]);
            let name_offset = u32::from_be_bytes([rom[i + 12], rom[i + 13], rom[i + 14], rom[i + 15]]);
            let mut name = String::new();
            if (i as u32 + name_offset) < rom.len() as u32 {
                let mut n_idx = (i as u32 + name_offset) as usize;
                while n_idx < rom.len() && rom[n_idx] != 0 && rom[n_idx] >= 32 && rom[n_idx] < 127 {
                    name.push(rom[n_idx] as char);
                    n_idx += 1;
                }
            }
            if !name.is_empty() && size < 0x80000 {
                println!("Offset 0x{:06X} (Addr 0x{:06X}): Module '{:16}' (size {} bytes)", i, 0x400000 + i, name, size);
                i += 2;
                continue;
            }
        }
        i += 2;
    }
}
