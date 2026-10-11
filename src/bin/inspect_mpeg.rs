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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let game_path = find_file("giochi/Dragon's Lair (USA).chd").expect("Dragon's Lair CHD not found");
    let mut cd = CdRom::open(&game_path)?;
    let mut sector_buf = [0u8; CD_SECTOR_RAW_SIZE];

    let mut stream: Vec<u8> = Vec::new();

    // Read sectors 6422 to 6460 (video stream)
    for lba in 6422..=6460 {
        if cd.read_sector(lba, &mut sector_buf) {
            let submode = sector_buf[18];
            // Video or Audio sector
            if (submode & 0x62) == 0x62 { // video
                stream.extend_from_slice(&sector_buf[24..24 + 2324]);
            }
        }
    }

    println!("Total video stream bytes extracted: {}", stream.len());

    // Scan for start codes (00 00 01 xx)
    let mut i = 0;
    while i + 4 <= stream.len() {
        if stream[i] == 0 && stream[i+1] == 0 && stream[i+2] == 1 {
            let code = stream[i+3];
            match code {
                0x00 => {
                    let temp_ref = ((stream[i+4] as u16) << 2) | ((stream[i+5] >> 6) as u16);
                    let pic_type = (stream[i+5] >> 3) & 7;
                    let type_name = match pic_type {
                        1 => "I-FRAME",
                        2 => "P-FRAME",
                        3 => "B-FRAME",
                        _ => "UNKNOWN",
                    };
                    println!("Offset {:6}: PICTURE_START (0x00) -> {} (temp_ref={})", i, type_name, temp_ref);
                }
                0xB3 => {
                    let w = ((stream[i+4] as usize) << 4) | ((stream[i+5] >> 4) as usize);
                    let h = (((stream[i+5] & 0x0F) as usize) << 8) | (stream[i+6] as usize);
                    let fps_code = stream[i+7] & 0x0F;
                    println!("Offset {:6}: SEQUENCE_HEADER (0xB3) -> {}x{}, fps_code={}", i, w, h, fps_code);
                }
                0xB8 => {
                    println!("Offset {:6}: GOP_START (0xB8)", i);
                }
                0xBA => {
                    // Pack header
                    // println!("Offset {:6}: PACK_HEADER (0xBA)", i);
                }
                0xBB => {
                    // System header
                    // println!("Offset {:6}: SYSTEM_HEADER (0xBB)", i);
                }
                0xBE => {
                    // Padding
                }
                0xE0 => {
                    let len = ((stream[i+4] as usize) << 8) | (stream[i+5] as usize);
                    println!("Offset {:6}: VIDEO_PACKET (0xE0) len={}", i, len);
                }
                0x01..=0xAF => {
                    // Slice start
                    // println!("Offset {:6}: SLICE_START 0x{:02X}", i, code);
                }
                _ => {}
            }
        }
        i += 1;
    }

    Ok(())
}
