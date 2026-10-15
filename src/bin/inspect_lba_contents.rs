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

    println!("Scanning for submode 0x62 (Form 2 Video)...");
    let mut buf = [0u8; CD_SECTOR_RAW_SIZE];
    let mut in_range = false;
    let mut range_start = 0;
    let mut range_count = 0;
    let mut file_id = 0;
    for lba in 0..146032 {
        if cd.read_sector(lba, &mut buf) {
            let submode = buf[18];
            let is_v = (submode & 0x62) == 0x62;
            if is_v {
                if !in_range || buf[16] != file_id {
                    if in_range {
                        println!("Video range [{}..{}] (count={}) file={}", range_start, lba - 1, range_count, file_id);
                    }
                    in_range = true;
                    range_start = lba;
                    range_count = 1;
                    file_id = buf[16];
                } else {
                    range_count += 1;
                }
            } else {
                if in_range {
                    println!("Video range [{}..{}] (count={}) file={}", range_start, lba - 1, range_count, file_id);
                    in_range = false;
                }
            }
        }
    }

    Ok(())
}
