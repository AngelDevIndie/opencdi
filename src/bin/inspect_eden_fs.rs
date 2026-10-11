#[path = "../cdrom.rs"]
mod cdrom;

use std::path::{Path, PathBuf};
use crate::cdrom::{CdRom, CD_SECTOR_RAW_SIZE};

fn find_file(rel: &str) -> Option<PathBuf> {
    for c in [
        Path::new(rel).to_path_buf(),
        Path::new("..").join(rel),
        Path::new("file utili").join(rel),
        Path::new("..").join("file utili").join(rel),
    ] {
        if c.exists() { return Some(c); }
    }
    None
}

fn parse_dir(cd: &mut CdRom, start_lba: u32, total_bytes: u32, prefix: &str) {
    let mut sector_buf = [0u8; CD_SECTOR_RAW_SIZE];
    let num_sectors = (total_bytes.max(2048) + 2047) / 2048;

    for s in 0..num_sectors {
        let lba = start_lba + s;
        if !cd.read_sector(lba, &mut sector_buf) { break; }
        let mut offset = 24;
        while offset < 2048 + 24 {
            let record_len = sector_buf[offset] as usize;
            if record_len == 0 { break; }
            if offset + record_len > sector_buf.len() { break; }

            let name_len = sector_buf[offset + 32] as usize;
            let name = if name_len > 0 && offset + 33 + name_len <= sector_buf.len() {
                String::from_utf8_lossy(&sector_buf[offset + 33..offset + 33 + name_len]).to_string()
            } else {
                String::new()
            };
            let lbn = ((sector_buf[offset + 2] as u32) << 24)
                | ((sector_buf[offset + 3] as u32) << 16)
                | ((sector_buf[offset + 4] as u32) << 8)
                | (sector_buf[offset + 5] as u32);
            let size = ((sector_buf[offset + 10] as u32) << 24)
                | ((sector_buf[offset + 11] as u32) << 16)
                | ((sector_buf[offset + 12] as u32) << 8)
                | (sector_buf[offset + 13] as u32);
            let attr = ((sector_buf[offset + 18] as u16) << 8) | (sector_buf[offset + 19] as u16);
            let is_dir = (attr & 0x0080) != 0 || (attr & 0x8000) != 0;
            println!("{}/{:<20} | LBN={:6} | Size={:8} | Attr=0x{:04X} {}", prefix, name, lbn, size, attr, if is_dir { "[DIR]" } else { "" });

            offset += record_len;
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let game_path = find_file("giochi/Lost Eden (Europe).chd").expect("Game not found");
    let mut cd = CdRom::open(&game_path)?;
    println!("=== Lost Eden Root Directory (LBA 2269) ===");
    parse_dir(&mut cd, 2269, 2048, "");
    Ok(())
}
