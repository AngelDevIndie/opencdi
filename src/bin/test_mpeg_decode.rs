#[path = "../cdrom.rs"]
mod cdrom;
#[path = "../mpeg/mod.rs"]
mod mpeg;

use std::path::{Path, PathBuf};
use crate::cdrom::{CdRom, CD_SECTOR_RAW_SIZE};
use crate::mpeg::{demux_all, Video};

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

fn save_bmp(buffer: &[u32], width: usize, height: usize, path: &str) -> std::io::Result<()> {
    let w = width as i32;
    let h = height as i32;
    let file_size = 54 + (width * height * 4) as u32;

    let mut bmp = Vec::with_capacity(file_size as usize);
    bmp.extend_from_slice(b"BM");
    bmp.extend_from_slice(&file_size.to_le_bytes());
    bmp.extend_from_slice(&[0, 0, 0, 0]);
    bmp.extend_from_slice(&54u32.to_le_bytes());

    bmp.extend_from_slice(&40u32.to_le_bytes());
    bmp.extend_from_slice(&w.to_le_bytes());
    bmp.extend_from_slice(&(-h).to_le_bytes()); // top-down
    bmp.extend_from_slice(&1u16.to_le_bytes());
    bmp.extend_from_slice(&32u16.to_le_bytes());
    bmp.extend_from_slice(&0u32.to_le_bytes());
    bmp.extend_from_slice(&((width * height * 4) as u32).to_le_bytes());
    bmp.extend_from_slice(&2835u32.to_le_bytes());
    bmp.extend_from_slice(&2835u32.to_le_bytes());
    bmp.extend_from_slice(&0u32.to_le_bytes());
    bmp.extend_from_slice(&0u32.to_le_bytes());

    for &px in buffer {
        let b = (px & 0xFF) as u8;
        let g = ((px >> 8) & 0xFF) as u8;
        let r = ((px >> 16) & 0xFF) as u8;
        let a = 0xFF;
        bmp.extend_from_slice(&[b, g, r, a]);
    }

    std::fs::write(path, bmp)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let game_path = find_file("giochi/Dragon's Lair (USA).chd").expect("Dragon's Lair CHD not found");
    let mut cd = CdRom::open(&game_path)?;
    let mut sector_buf = [0u8; CD_SECTOR_RAW_SIZE];

    let mut stream: Vec<u8> = Vec::new();

    let mut demux = crate::mpeg::Demux::new(Vec::new());
    let mut video: Option<Video> = None;
    let mut frame_count = 0;

    // Read sectors 6422 to 6600 (video stream)
    for lba in 6422..=6600 {
        if cd.read_sector(lba, &mut sector_buf) {
            let submode = sector_buf[18];
            if (submode & 0x62) == 0x62 {
                demux.append(&sector_buf[24..24 + 2324]);
                while let Some(packet) = demux.decode() {
                    if packet.ty == crate::mpeg::PACKET_VIDEO_1 {
                        if video.is_none() {
                            video = Some(Video::new(packet.data));
                        } else {
                            video.as_mut().unwrap().append(&packet.data);
                        }
                    }
                }
                if let Some(ref mut v) = video {
                    while let Some(frame_idx) = v.decode() {
                        frame_count += 1;
                        println!("Streaming decoded frame #{} (idx={}) at LBA {}", frame_count, frame_idx, lba);

                        let mut rgb = vec![0u32; v.width() * v.height()];
                        v.frame(frame_idx).write_rgb_scaled(&mut rgb, v.width(), v.height());

                        let out_path = format!("C:/Users/Gian/.gemini/antigravity/brain/47421117-6e2a-4536-8928-519a0c9b76ed/dvc_stream_frame_{}.bmp", frame_count);
                        save_bmp(&rgb, v.width(), v.height(), &out_path)?;
                    }
                }
            }
        }
    }

    println!("Total streaming frames decoded: {}", frame_count);

    Ok(())
}
