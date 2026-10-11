#[path = "../cdrom.rs"]
mod cdrom;

use std::path::{Path, PathBuf};
use crate::cdrom::{CdRom, CD_SECTOR_RAW_SIZE};

fn find_file(relative_subpath: &str) -> Option<PathBuf> {
    let candidates = [
        Path::new(relative_subpath).to_path_buf(),
        Path::new("..").join(relative_subpath),
        Path::new("file utili").join(relative_subpath),
        Path::new("..").join("file utili").join(relative_subpath),
    ];
    for c in &candidates {
        if c.exists() {
            return Some(c.clone());
        }
    }
    None
}

fn generate_sector_scramble() -> [u8; 2448] {
    let mut table = [0u8; 2448];
    let mut reg: u16 = 0x0001;
    for i in 12..2448 {
        let mut byte_val = 0u8;
        for bit in 0..8 {
            let out_bit = (reg & 1) as u8;
            byte_val |= out_bit << bit;
            let feedback = (reg ^ (reg >> 1)) & 1;
            reg = (reg >> 1) | (feedback << 14);
        }
        table[i] = byte_val;
    }
    table
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let game_path = find_file("giochi/Dragon's Lair (USA).chd").expect("Dragon's Lair CHD not found");
    let mut cd = CdRom::open(&game_path)?;
    let mut sector_buf = [0u8; CD_SECTOR_RAW_SIZE];

    for lba in 6420..=6430 {
        if cd.read_sector(lba, &mut sector_buf) {
            let is_sync = sector_buf[0] == 0x00 && sector_buf[1..11].iter().all(|&b| b == 0xFF) && sector_buf[11] == 0x00;
            let is_swapped = sector_buf[0] == 0xFF && sector_buf[1] == 0x00;
            println!("LBA {}: sync={}, swapped={}, bytes 12..24:", lba, is_sync, is_swapped);
            for b in &sector_buf[12..24] { print!("{:02X} ", b); }
            println!();
            if lba == 6422 {
                println!("LBA 6422 hex 24..64:");
                for b in &sector_buf[24..64] { print!("{:02X} ", b); }
                println!();
            }
            let data = &sector_buf[24..24+128];
            let text: String = data.iter().map(|&c| if c >= 32 && c <= 126 { c as char } else { '.' }).collect();
            println!("   data: {}", text);
        }
    }
    Ok(())
}
