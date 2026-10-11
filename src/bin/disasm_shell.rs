use std::fs;
use std::path::Path;

fn main() {
    let candidates = [
        "file utili/bios/cdi220b.rom",
        "../file utili/bios/cdi220b.rom",
        "bios/cdi220b.rom",
        "../bios/cdi220b.rom",
    ];
    let mut bios_data = None;
    for c in &candidates {
        if let Ok(d) = fs::read(c) {
            bios_data = Some(d);
            break;
        }
    }
    let bios = bios_data.expect("bios not found");
    let base_pc = 0x400000;
    let start = (0x4295C0 - base_pc) as usize;
    let end = (0x42DCD6 - base_pc) as usize;
    let target_pc = 0x42D3D2;
    println!("Searching for references to 0x00D4 in ROM (0x431800..0x433000):");
    let p_start = (0x431800 - base_pc) as usize;
    let p_end = (0x433000 - base_pc) as usize;
    for i in p_start..p_end - 4 {
        if bios[i] == 0x00 && bios[i + 1] == 0xD4 {
            let pc = base_pc + i as u32;
            let mut sub_pc = pc.saturating_sub(6);
            while sub_pc < pc + 8 {
                let off = (sub_pc - base_pc) as usize;
                if off + 2 > bios.len() { break; }
                let op = ((bios[off] as u16) << 8) | (bios[off + 1] as u16);
                let (dis, len) = m68k::dasm::disassemble(sub_pc, op, m68k::CpuType::SCC68070);
                println!("  0x{:06X}: {}", sub_pc, dis);
                sub_pc += len as u32;
            }
        }
    }
}
