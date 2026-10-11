use std::fs;

fn main() {
    let rom = fs::read("../file utili/bios/cdi220b.rom")
        .or_else(|_| fs::read("bios/cdi220b.rom"))
        .expect("ROM not found");

    let base = 0x295C0; // 0x4295C0 in 68k space
    println!("=== cdapdriv Header at 0x{:06X} ===", 0x400000 + base);
    for row in 0..8 {
        let o = base + row * 16;
        let bytes: Vec<String> = (0..16).map(|i| format!("{:02X}", rom[o + i])).collect();
        println!("+0x{:02X} (0x{:06X}): {}", row * 16, 0x400000 + o, bytes.join(" "));
    }

    let dvc_rom = fs::read("../file utili/dvc/vmpega.rom")
        .or_else(|_| fs::read("dvc/vmpega.rom"))
        .expect("DVC ROM not found");
    println!("DVC ROM size: {} bytes", dvc_rom.len());

    let mut i = 0;
    while i + 48 < dvc_rom.len() {
        if dvc_rom[i] == 0x4A && dvc_rom[i + 1] == 0xFC {
            let size = u32::from_be_bytes([dvc_rom[i + 4], dvc_rom[i + 5], dvc_rom[i + 6], dvc_rom[i + 7]]) as usize;
            let name_off = u32::from_be_bytes([dvc_rom[i + 12], dvc_rom[i + 13], dvc_rom[i + 14], dvc_rom[i + 15]]) as usize;
            let type_lang = u16::from_be_bytes([dvc_rom[i + 18], dvc_rom[i + 19]]);
            let exec_off = u32::from_be_bytes([dvc_rom[i + 24], dvc_rom[i + 25], dvc_rom[i + 26], dvc_rom[i + 27]]) as usize;
            
            if size >= 48 && size <= dvc_rom.len() - i && name_off < size && name_off >= 28 {
                let mut name = String::new();
                let mut no = i + name_off;
                while no < i + size && dvc_rom[no] != 0 && dvc_rom[no] >= 0x20 && dvc_rom[no] < 0x7F {
                    name.push(dvc_rom[no] as char);
                    no += 1;
                }
                if no < i + size && (dvc_rom[no] & 0x7F) >= 0x20 && (dvc_rom[no] & 0x7F) < 0x7F && (dvc_rom[no] & 0x80) != 0 {
                    name.push((dvc_rom[no] & 0x7F) as char);
                }
                if !name.is_empty() {
                    let type_byte = (type_lang >> 8) as u8;
                    let lang_byte = (type_lang & 0xFF) as u8;
                    println!("Module at 0x{:06X} (size 0x{:05X}): Type=0x{:02X}, Lang=0x{:02X}, Exec=+0x{:04X}, Name='{}'",
                        0xE40000 + i, size, type_byte, lang_byte, exec_off, name);
                    i += (size + 3) & !3;
                    continue;
                }
            }
        }
        i += 2;
    }

    println!("\n=== Inspecting Device Driver 'fmvdrv' at 0xE51A8C ===");
    let drv_off = 0x11A8Cusize;
    let init_off = u16::from_be_bytes([dvc_rom[drv_off + 0x20], dvc_rom[drv_off + 0x21]]) as usize;
    let read_off = u16::from_be_bytes([dvc_rom[drv_off + 0x22], dvc_rom[drv_off + 0x23]]) as usize;
    let write_off = u16::from_be_bytes([dvc_rom[drv_off + 0x24], dvc_rom[drv_off + 0x25]]) as usize;
    let getstat_off = u16::from_be_bytes([dvc_rom[drv_off + 0x26], dvc_rom[drv_off + 0x27]]) as usize;
    let setstat_off = u16::from_be_bytes([dvc_rom[drv_off + 0x28], dvc_rom[drv_off + 0x29]]) as usize;
    let term_off = u16::from_be_bytes([dvc_rom[drv_off + 0x2A], dvc_rom[drv_off + 0x2B]]) as usize;
    println!("fmvdrv: Init=+0x{:04X} (0x{:06X})", init_off, 0xE40000 + drv_off + init_off);
    println!("fmvdrv: Read=+0x{:04X} (0x{:06X})", read_off, 0xE40000 + drv_off + read_off);
    println!("fmvdrv: Write=+0x{:04X} (0x{:06X})", write_off, 0xE40000 + drv_off + write_off);
    println!("fmvdrv: GetStat=+0x{:04X} (0x{:06X})", getstat_off, 0xE40000 + drv_off + getstat_off);
    println!("fmvdrv: SetStat=+0x{:04X} (0x{:06X})", setstat_off, 0xE40000 + drv_off + setstat_off);
    println!("fmvdrv: Term=+0x{:04X} (0x{:06X})", term_off, 0xE40000 + drv_off + term_off);

    println!("\n=== Searching for signals 0x1018, 0x1020 etc in DVC ROM ===");
    for p in 0..dvc_rom.len() - 4 {
        let w = u16::from_be_bytes([dvc_rom[p], dvc_rom[p + 1]]);
        if w == 0x1018 || w == 0x1020 || w == 0x1019 || w == 0x1021 || w == 0x1017 {
            let pc = 0xE40000 + p as u32;
            let prev = if p >= 2 { u16::from_be_bytes([dvc_rom[p - 2], dvc_rom[p - 1]]) } else { 0 };
            let (text, _) = m68k::dasm::disassemble(pc - 2, prev, m68k::CpuType::SCC68070);
            println!("  0x{:06X}: prev={:04X} sig={:04X} | {}", pc - 2, prev, w, text);
        }
    }
















}
