use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use chd::Chd;
use log::{info, error};

pub const CD_SECTOR_RAW_SIZE: usize = 2352;
pub const CD_SUBCHANNEL_SIZE: usize = 96;
pub const CD_FRAME_SIZE: usize = CD_SECTOR_RAW_SIZE + CD_SUBCHANNEL_SIZE; // 2448

pub enum CdSource {
    Chd {
        chd: Chd<File>,
        hunk_buf: Vec<u8>,
        comp_buf: Vec<u8>,
        current_hunk: Option<u32>,
        hunk_size: usize,
        sectors_per_hunk: u32,
    },
    RawFile {
        file: File,
        sector_size: usize,
    },
}

pub struct CdRom {
    source: CdSource,
    pub total_sectors: u32,
}

impl CdRom {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let path_ref = path.as_ref();
        let ext = path_ref.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();

        let target_path = if ext == "cue" {
            // Parse CUE file to locate the actual BIN file
            if let Ok(content) = std::fs::read_to_string(path_ref) {
                let mut bin_name = None;
                for line in content.lines() {
                    let trimmed = line.trim();
                    if trimmed.starts_with("FILE ") {
                        if let Some(start) = trimmed.find('"') {
                            if let Some(end) = trimmed[start + 1..].find('"') {
                                bin_name = Some(trimmed[start + 1..start + 1 + end].to_string());
                                break;
                            }
                        }
                    }
                }
                if let Some(name) = bin_name {
                    path_ref.parent().map(|p| p.join(&name)).unwrap_or_else(|| PathBuf::from(name))
                } else {
                    path_ref.to_path_buf()
                }
            } else {
                path_ref.to_path_buf()
            }
        } else {
            path_ref.to_path_buf()
        };

        let target_ext = target_path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
        if target_ext == "chd" {
            let file = File::open(&target_path)?;
            let chd = Chd::open(file, None)?;
            let header = chd.header();

            let hunk_size = header.hunk_size() as usize;
            let sectors_per_hunk = if hunk_size % CD_FRAME_SIZE == 0 {
                (hunk_size / CD_FRAME_SIZE) as u32
            } else if hunk_size % CD_SECTOR_RAW_SIZE == 0 {
                (hunk_size / CD_SECTOR_RAW_SIZE) as u32
            } else {
                8
            };

            let total_sectors = header.hunk_count() * sectors_per_hunk;
            let hunk_buf = chd.get_hunksized_buffer();

            info!(
                "Opened CHD: {:?}, hunk_size={}, sectors_per_hunk={}, total_sectors={}",
                target_path, hunk_size, sectors_per_hunk, total_sectors
            );

            Ok(Self {
                source: CdSource::Chd {
                    chd,
                    hunk_buf,
                    comp_buf: Vec::new(),
                    current_hunk: None,
                    hunk_size,
                    sectors_per_hunk,
                },
                total_sectors,
            })
        } else {
            let file = File::open(&target_path)?;
            let file_size = file.metadata()?.len();
            let sector_size = if target_ext == "iso" { 2048 } else { CD_SECTOR_RAW_SIZE };
            let total_sectors = (file_size / sector_size as u64) as u32;

            info!(
                "Opened raw disc image: {:?}, sector_size={}, total_sectors={}",
                target_path, sector_size, total_sectors
            );

            Ok(Self {
                source: CdSource::RawFile {
                    file,
                    sector_size,
                },
                total_sectors,
            })
        }
    }

    pub fn read_sector(&mut self, lba: u32, out_sector: &mut [u8; CD_SECTOR_RAW_SIZE]) -> bool {
        if lba >= self.total_sectors {
            error!("CD-ROM: Sector {} out of bounds (max {})", lba, self.total_sectors);
            return false;
        }

        match &mut self.source {
            CdSource::Chd { chd, hunk_buf, comp_buf, current_hunk, hunk_size, sectors_per_hunk } => {
                let hunk_idx = lba / *sectors_per_hunk;
                let sector_in_hunk = (lba % *sectors_per_hunk) as usize;

                if *current_hunk != Some(hunk_idx) {
                    match chd.hunk(hunk_idx) {
                        Ok(mut hunk) => {
                            if let Err(e) = hunk.read_hunk_in(comp_buf, hunk_buf) {
                                error!("CD-ROM: Error reading hunk {}: {:?}", hunk_idx, e);
                                return false;
                            }
                            *current_hunk = Some(hunk_idx);
                        }
                        Err(e) => {
                            error!("CD-ROM: Failed to get hunk {}: {:?}", hunk_idx, e);
                            return false;
                        }
                    }
                }

                let frame_stride = if *hunk_size % CD_FRAME_SIZE == 0 {
                    CD_FRAME_SIZE
                } else {
                    CD_SECTOR_RAW_SIZE
                };

                let start = sector_in_hunk * frame_stride;
                if start + CD_SECTOR_RAW_SIZE <= hunk_buf.len() {
                    out_sector.copy_from_slice(&hunk_buf[start..start + CD_SECTOR_RAW_SIZE]);
                    true
                } else {
                    error!("CD-ROM: Sector offset out of hunk bounds");
                    false
                }
            }
            CdSource::RawFile { file, sector_size } => {
                let offset = (lba as u64) * (*sector_size as u64);
                if file.seek(SeekFrom::Start(offset)).is_err() {
                    return false;
                }
                if *sector_size == CD_SECTOR_RAW_SIZE {
                    file.read_exact(out_sector).is_ok()
                } else {
                    out_sector.fill(0);
                    let mut data = [0u8; 2048];
                    if file.read_exact(&mut data).is_err() {
                        return false;
                    }
                    out_sector[24..24 + 2048].copy_from_slice(&data);
                    true
                }
            }
        }
    }
}
