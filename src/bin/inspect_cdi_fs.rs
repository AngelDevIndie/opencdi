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

fn parse_rtos_dir(cd: &mut CdRom, start_lba: u32, total_bytes: u32, path_prefix: &str) {
    let mut sector_buf = [0u8; CD_SECTOR_RAW_SIZE];
    let num_sectors = (total_bytes.max(2048) + 2047) / 2048;

    for s in 0..num_sectors {
        let lba = start_lba + s;
        if !cd.read_sector(lba, &mut sector_buf) { break; }
        let mut offset = 24; // Form 1 data starts at 24
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
            print!("{:<15}: ", name);
            for b in &sector_buf[offset..offset + 24] { print!("{:02X} ", b); }
            println!();

            offset += record_len;
        }
    }
}



fn main() -> Result<(), Box<dyn std::error::Error>> {
    let game_path = find_file("giochi/7th Guest, The (USA).chd").expect("Game not found");
    let mut cd = CdRom::open(&game_path)?;
    println!("=== The 7th Guest Root Directory ===");
    // CD-i root directory is usually at sector 150 (LBA 150 or 166)
    // Let's check sectors 150 to 180
    for lba in 150..180 {
        let mut buf = [0u8; CD_SECTOR_RAW_SIZE];
        if cd.read_sector(lba, &mut buf) {
            let form = buf[18] & 0x20;
            let submode = buf[18];
            if (submode & 0x0E) == 0 { // data
                parse_rtos_dir(&mut cd, lba, 2048, "");
            }
        }
    }
    return Ok(());
    let mut buf = [0u8; CD_SECTOR_RAW_SIZE];
    cd.read_sector(8661, &mut buf);
    for i in (0..64).step_by(16) {
        print!("{:04X}: ", i);
        for j in 0..16 {
            print!("{:02X} ", buf[i + j]);
        }
        println!();
    }








    Ok(())
}


