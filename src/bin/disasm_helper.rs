#[path = "../cdrom.rs"]
mod cdrom;

use std::path::{Path, PathBuf};
use crate::cdrom::{CdRom, CD_SECTOR_RAW_SIZE};
use opencdi::dvc::MpegSystemDemux;

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
    let game_path = find_file("giochi/Dragon's Lair (USA).chd").expect("chd not found");
    let text = std::fs::read_to_string("lair_disasm.txt")?;
    let lines: Vec<&str> = text.lines().collect();
    for (i, line) in lines.iter().enumerate() {
        if line.contains("0x27443A") || line.contains("0x2745A0") || line.contains("0x2745A6") || line.contains("0x2745AA") {
            for j in i.saturating_sub(4)..=(i + 12).min(lines.len() - 1) {
                println!("{}", lines[j]);
            }
            println!("------------------");
        }
    }
    Ok(())
}
