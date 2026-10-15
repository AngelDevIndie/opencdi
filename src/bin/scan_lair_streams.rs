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
    println!("Scanning sectors for video/audio streams on Dragon's Lair...");

    let mut buf = [0u8; CD_SECTOR_RAW_SIZE];
    let mut in_stream = false;
    let mut stream_start = 0;
    let mut stream_type = 0;
    let mut stream_count = 0;

    for lba in 0..146032 {
        if cd.read_sector(lba, &mut buf) {
            let submode = buf[18];
            let is_stream = (submode & 0x40) != 0; // Real-time sector
            let is_video = (submode & 0x02) != 0;
            let is_audio = (submode & 0x04) != 0;
            let is_data = (submode & 0x08) != 0;

            let stype = (submode & 0x0E);
            if is_stream {
                if !in_stream || stream_type != stype {
                    if in_stream {
                        println!("Stream range [{}..{}] (count={}): type=0x{:02X} (vid={}, aud={}, dat={})",
                            stream_start, lba - 1, stream_count, stream_type,
                            (stream_type & 2) != 0, (stream_type & 4) != 0, (stream_type & 8) != 0);
                    }
                    in_stream = true;
                    stream_start = lba;
                    stream_type = stype;
                    stream_count = 1;
                } else {
                    stream_count += 1;
                }
            } else {
                if in_stream {
                    println!("Stream range [{}..{}] (count={}): type=0x{:02X} (vid={}, aud={}, dat={})",
                        stream_start, lba - 1, stream_count, stream_type,
                        (stream_type & 2) != 0, (stream_type & 4) != 0, (stream_type & 8) != 0);
                    in_stream = false;
                }
            }
        }
    }

    Ok(())
}
