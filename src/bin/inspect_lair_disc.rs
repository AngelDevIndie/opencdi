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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let game_path = find_file("giochi/Dragon's Lair (USA).chd").expect("Game not found");
    let mut cd = CdRom::open(&game_path)?;
    let mut buf = [0u8; CD_SECTOR_RAW_SIZE];

    // Read volume descriptor at LBA 16
    if cd.read_sector(16, &mut buf) {
        let data = &buf[24..24 + 2048];
        let root_dir_lba = u32::from_be_bytes([data[142], data[143], data[144], data[145]]);
        let root_dir_size = u32::from_be_bytes([data[150], data[151], data[152], data[153]]);
        println!("Root dir: LBA {}, size {}", root_dir_lba, root_dir_size);
        scan_dir(&mut cd, root_dir_lba, root_dir_size, "");
    }

    Ok(())
}

fn scan_dir(cd: &mut CdRom, start_lba: u32, size: u32, indent: &str) {
    let mut buf = [0u8; CD_SECTOR_RAW_SIZE];
    let num_sectors = (size + 2047) / 2048;

    for s in 0..num_sectors {
        let lba = start_lba + s;
        if !cd.read_sector(lba, &mut buf) { break; }
        let data = &buf[24..24 + 2048];
        let mut offset = 0;
        while offset < 2048 {
            let rec_len = data[offset] as usize;
            if rec_len == 0 { break; }
            if offset + rec_len > 2048 || offset + 32 > 2048 { break; }

            let file_lba = u32::from_be_bytes([data[offset + 2], data[offset + 3], data[offset + 4], data[offset + 5]]);
            let file_size = u32::from_be_bytes([data[offset + 10], data[offset + 11], data[offset + 12], data[offset + 13]]);
            let flags = data[offset + 25];
            let name_len = data[offset + 32] as usize;
            let name = if name_len > 0 && offset + 33 + name_len <= 2048 {
                String::from_utf8_lossy(&data[offset + 33..offset + 33 + name_len]).to_string()
            } else {
                String::new()
            };

            let is_dir = (flags & 2) != 0;
            println!("{}{} -> LBA {}, size {} (flags 0x{:02X})", indent, name, file_lba, file_size, flags);

            if is_dir && name != "." && name != ".." && !name.is_empty() && name != "\0" && name != "\x01" {
                scan_dir(cd, file_lba, file_size, &format!("{}  ", indent));
            }

            offset += rec_len;
        }
    }
}
