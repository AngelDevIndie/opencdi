use std::path::{Path, PathBuf};
use std::fs;
use opencdi::cdrom::{CdRom, CD_SECTOR_RAW_SIZE};
use opencdi::dvc::MpegSystemDemux;
use opencdi::dvc::mpeg::Video;

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

fn save_bmp(path: &str, buffer: &[u32; 352 * 288]) {
    let mut data = Vec::with_capacity(352 * 288 * 4 + 54);
    let width = 352u32;
    let height = 288u32;
    let file_size = 54 + width * height * 4;
    data.extend_from_slice(b"BM");
    data.extend_from_slice(&file_size.to_le_bytes());
    data.extend_from_slice(&[0, 0, 0, 0]);
    data.extend_from_slice(&54u32.to_le_bytes());
    data.extend_from_slice(&40u32.to_le_bytes());
    data.extend_from_slice(&(width as i32).to_le_bytes());
    data.extend_from_slice(&(-(height as i32)).to_le_bytes());
    data.extend_from_slice(&1u16.to_le_bytes());
    data.extend_from_slice(&32u16.to_le_bytes());
    data.extend_from_slice(&0u32.to_le_bytes());
    data.extend_from_slice(&(width * height * 4).to_le_bytes());
    data.extend_from_slice(&0u32.to_le_bytes());
    data.extend_from_slice(&0u32.to_le_bytes());
    data.extend_from_slice(&0u32.to_le_bytes());
    data.extend_from_slice(&0u32.to_le_bytes());

    for &pix in buffer.iter() {
        data.extend_from_slice(&pix.to_le_bytes());
    }
    fs::write(path, data).unwrap();
}

fn test_stream(cd: &mut CdRom, start_lba: u32, end_lba: u32, name: &str) {
    let mut demux = MpegSystemDemux::default();
    let mut buf = [0u8; CD_SECTOR_RAW_SIZE];

    for lba in start_lba..=end_lba {
        if cd.read_sector(lba, &mut buf) {
            let data = &buf[24..24 + 2324];
            demux.feed(data);
        }
    }

    let mut decoder = Video::new(demux.video.clone());
    let mut fb = Box::new([0xFF00_0000u32; 352 * 288]);
    let mut count = 0;
    while let Some(idx) = decoder.decode() {
        count += 1;
        let w = (decoder.width() as usize).min(352);
        let h = (decoder.height() as usize).min(288);
        decoder.frame(idx).write_rgb_scaled(&mut *fb, w, h);
    }
    println!("Stream '{}' (LBA {}..{}): demuxed {} video bytes, decoded {} frames",
        name, start_lba, end_lba, demux.video.len(), count);
    if count > 0 {
        save_bmp(&format!("{}.bmp", name), &fb);
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let game_path = find_file("giochi/Dragon's Lair (USA).chd").expect("Game not found");
    let mut cd = CdRom::open(&game_path)?;

    test_stream(&mut cd, 6440, 6600, "stream_6440");
    test_stream(&mut cd, 9779, 9840, "stream_9779");

    Ok(())
}
