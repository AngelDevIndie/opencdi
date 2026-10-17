use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use chd::Chd;
use log::{info, error};

pub const CD_SECTOR_RAW_SIZE: usize = 2352;
pub const CD_SUBCHANNEL_SIZE: usize = 96;
pub const CD_FRAME_SIZE: usize = CD_SECTOR_RAW_SIZE + CD_SUBCHANNEL_SIZE; // 2448

#[derive(Clone, Debug)]
pub struct CdTrack {
    pub number: u8,
    pub control: u8,      // 0x41 for Data, 0x01 for Audio
    pub start_lba: u32,
    pub length_sectors: u32,
    pub sector_size: usize,
    pub file_index: usize,
    pub file_offset: u64,
}

pub enum CdSource {
    Chd {
        chd: Chd<File>,
        hunk_buf: Vec<u8>,
        comp_buf: Vec<u8>,
        current_hunk: Option<u32>,
        hunk_size: usize,
        sectors_per_hunk: u32,
    },
    RawFiles {
        files: Vec<File>,
    },
}

pub struct CdRom {
    source: CdSource,
    pub total_sectors: u32,
    pub tracks: Vec<CdTrack>,
    pub disc_type: u8, // 0x10 = CD-i, 0x20 = CD-ROM XA / CD-i Bridge / Video CD, 0x00 = CD-DA
}

fn parse_msf(s: &str) -> Option<u32> {
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() != 3 {
        return None;
    }
    let m: u32 = parts[0].trim().parse().ok()?;
    let s: u32 = parts[1].trim().parse().ok()?;
    let f: u32 = parts[2].trim().parse().ok()?;
    Some(m * 60 * 75 + s * 75 + f)
}

impl CdRom {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let path_ref = path.as_ref();
        let ext = path_ref.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();

        if ext == "cue" {
            let content = std::fs::read_to_string(path_ref)?;
            let mut files: Vec<File> = Vec::new();
            let mut file_sizes: Vec<u64> = Vec::new();

            struct ParsedTrack {
                num: u8,
                control: u8,
                sector_size: usize,
                file_index: usize,
                idx01: u32,
            }
            let mut parsed_tracks: Vec<ParsedTrack> = Vec::new();
            let mut current_file_idx: Option<usize> = None;

            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.is_empty() || trimmed.starts_with('#') {
                    continue;
                }
                let parts: Vec<&str> = trimmed.split_whitespace().collect();
                if parts.is_empty() {
                    continue;
                }

                if parts[0].eq_ignore_ascii_case("FILE") {
                    if let Some(start) = trimmed.find('"') {
                        if let Some(end) = trimmed.rfind('"') {
                            if end > start {
                                let fname = &trimmed[start + 1..end];
                                let fpath = path_ref.parent().map(|p| p.join(fname)).unwrap_or_else(|| PathBuf::from(fname));
                                let file = File::open(&fpath)?;
                                let fsize = file.metadata()?.len();
                                files.push(file);
                                file_sizes.push(fsize);
                                current_file_idx = Some(files.len() - 1);
                            }
                        }
                    }
                } else if parts[0].eq_ignore_ascii_case("TRACK") {
                    if parts.len() >= 3 {
                        let tnum: u8 = parts[1].parse().unwrap_or(1);
                        let mode = parts[2].to_uppercase();
                        let control = if mode.starts_with("AUDIO") { 0x01 } else { 0x41 };
                        let sector_size = if mode.contains("2048") { 2048 } else { CD_SECTOR_RAW_SIZE };
                        let fidx = current_file_idx.unwrap_or(0);
                        parsed_tracks.push(ParsedTrack {
                            num: tnum,
                            control,
                            sector_size,
                            file_index: fidx,
                            idx01: 0,
                        });
                    }
                } else if parts[0].eq_ignore_ascii_case("INDEX") {
                    if parts.len() >= 3 && (parts[1] == "01" || parts[1] == "1") {
                        if let Some(t) = parsed_tracks.last_mut() {
                            if let Some(sectors) = parse_msf(parts[2]) {
                                t.idx01 = sectors;
                            }
                        }
                    }
                }
            }

            if files.is_empty() {
                return Err(format!("No files found in CUE sheet {:?}", path_ref).into());
            }

            let mut tracks: Vec<CdTrack> = Vec::new();
            let total_sectors: u32;

            if files.len() == 1 {
                let fsize = file_sizes[0];
                let sec_size = parsed_tracks.first().map(|t| t.sector_size).unwrap_or(CD_SECTOR_RAW_SIZE);
                let file_sectors = (fsize / sec_size as u64) as u32;

                for i in 0..parsed_tracks.len() {
                    let pt = &parsed_tracks[i];
                    let start_lba = pt.idx01;
                    let file_offset = (start_lba as u64) * (pt.sector_size as u64);
                    let length_sectors = if i + 1 < parsed_tracks.len() {
                        parsed_tracks[i + 1].idx01.saturating_sub(start_lba)
                    } else {
                        file_sectors.saturating_sub(start_lba)
                    };

                    tracks.push(CdTrack {
                        number: pt.num,
                        control: pt.control,
                        start_lba,
                        length_sectors,
                        sector_size: pt.sector_size,
                        file_index: 0,
                        file_offset,
                    });
                }
                total_sectors = file_sectors;
            } else {
                let mut current_disc_lba = 0u32;
                for pt in parsed_tracks {
                    let fsize = file_sizes.get(pt.file_index).copied().unwrap_or(0);
                    let length_sectors = (fsize / pt.sector_size as u64) as u32;
                    tracks.push(CdTrack {
                        number: pt.num,
                        control: pt.control,
                        start_lba: current_disc_lba,
                        length_sectors,
                        sector_size: pt.sector_size,
                        file_index: pt.file_index,
                        file_offset: 0,
                    });
                    current_disc_lba += length_sectors;
                }
                total_sectors = current_disc_lba;
            }

            if tracks.is_empty() {
                tracks.push(CdTrack {
                    number: 1,
                    control: 0x41,
                    start_lba: 0,
                    length_sectors: total_sectors,
                    sector_size: CD_SECTOR_RAW_SIZE,
                    file_index: 0,
                    file_offset: 0,
                });
            }

            info!(
                "Opened CUE {:?}: {} file(s), {} track(s), total_sectors={}",
                path_ref,
                files.len(),
                tracks.len(),
                total_sectors
            );
            for t in &tracks {
                info!("  Track {:02}: LBA {}..{} ({} sectors) ctrl=0x{:02X} file_idx={}", t.number, t.start_lba, t.start_lba + t.length_sectors.saturating_sub(1), t.length_sectors, t.control, t.file_index);
            }

            let mut cdrom = Self {
                source: CdSource::RawFiles { files },
                total_sectors,
                tracks,
                disc_type: 0x10,
            };
            cdrom.detect_disc_type();
            return Ok(cdrom);
        }

        if ext == "chd" {
            let file = File::open(path_ref)?;
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
                path_ref, hunk_size, sectors_per_hunk, total_sectors
            );

            let tracks = vec![CdTrack {
                number: 1,
                control: 0x41,
                start_lba: 0,
                length_sectors: total_sectors,
                sector_size: CD_SECTOR_RAW_SIZE,
                file_index: 0,
                file_offset: 0,
            }];

            let mut cdrom = Self {
                source: CdSource::Chd {
                    chd,
                    hunk_buf,
                    comp_buf: Vec::new(),
                    current_hunk: None,
                    hunk_size,
                    sectors_per_hunk,
                },
                total_sectors,
                tracks,
                disc_type: 0x10,
            };
            cdrom.detect_disc_type();
            Ok(cdrom)
        } else {
            let file = File::open(path_ref)?;
            let file_size = file.metadata()?.len();
            let sector_size = if ext == "iso" { 2048 } else { CD_SECTOR_RAW_SIZE };
            let total_sectors = (file_size / sector_size as u64) as u32;

            info!(
                "Opened raw disc image: {:?}, sector_size={}, total_sectors={}",
                path_ref, sector_size, total_sectors
            );

            let tracks = vec![CdTrack {
                number: 1,
                control: 0x41,
                start_lba: 0,
                length_sectors: total_sectors,
                sector_size,
                file_index: 0,
                file_offset: 0,
            }];

            let mut cdrom = Self {
                source: CdSource::RawFiles { files: vec![file] },
                total_sectors,
                tracks,
                disc_type: 0x10,
            };
            cdrom.detect_disc_type();
            Ok(cdrom)
        }
    }

    fn detect_disc_type(&mut self) {
        let mut s16 = [0u8; CD_SECTOR_RAW_SIZE];
        if self.read_sector(16, &mut s16) {
            let sys_id = &s16[24 + 8..24 + 40];
            if sys_id.starts_with(b"CD-RTOS CD-BRIDGE") {
                info!("CD-ROM: Riconosciuto disco CD-RTOS CD-BRIDGE (Video CD / White Book), disc_type = 0x20");
                self.disc_type = 0x20;
            } else if sys_id.starts_with(b"CD-RTOS") || sys_id.starts_with(b"CD-I") {
                info!("CD-ROM: Riconosciuto disco CD-i standard, disc_type = 0x10");
                self.disc_type = 0x10;
            } else {
                info!("CD-ROM: Disc system ID: {:?}, disc_type = 0x10", String::from_utf8_lossy(sys_id).trim());
                self.disc_type = 0x10;
            }
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
            CdSource::RawFiles { files } => {
                if let Some(track) = self.tracks.iter().find(|t| lba >= t.start_lba && lba < t.start_lba + t.length_sectors) {
                    if track.file_index >= files.len() {
                        return false;
                    }
                    let file = &mut files[track.file_index];
                    let rel_lba = lba - track.start_lba;
                    let offset = track.file_offset + (rel_lba as u64) * (track.sector_size as u64);

                    if file.seek(SeekFrom::Start(offset)).is_err() {
                        return false;
                    }
                    if track.sector_size == CD_SECTOR_RAW_SIZE {
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
                } else {
                    error!("CD-ROM: No track matches LBA {}", lba);
                    false
                }
            }
        }
    }
}
