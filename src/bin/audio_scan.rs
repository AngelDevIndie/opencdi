#[path = "../cdrom.rs"]
mod cdrom;

use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use crate::cdrom::{CdRom, CD_SECTOR_RAW_SIZE};

fn find_file(rel: &str) -> Option<PathBuf> {
    for c in [
        Path::new(rel).to_path_buf(),
        Path::new("..").join(rel),
        Path::new("file utili").join(rel),
        Path::new("..").join("file utili").join(rel),
    ] {
        if c.exists() {
            return Some(c);
        }
    }
    None
}

// CD-i ADPCM Predictor Coefficients (K0, K1)
const K0: [f64; 4] = [0.0, 0.9375, 1.796875, 1.53125];
const K1: [f64; 4] = [0.0, 0.0, -0.8125, -0.859375];

#[derive(Default)]
struct ChannelState {
    hist1: f64,
    hist2: f64,
}

impl ChannelState {
    fn decode_nibble(&mut self, nibble: u8, filter: usize, shift: usize) -> i16 {
        // Sign extend 4-bit nibble (-8..7)
        let val = if nibble >= 8 {
            (nibble as i32) - 16
        } else {
            nibble as i32
        };

        // Scale by shift: 12 - shift
        let scale = (12 - shift).min(12);
        let sample_val = (val << scale) as f64;

        let filt = filter.min(3);
        let pred = K0[filt] * self.hist1 + K1[filt] * self.hist2;
        let mut sample = sample_val + pred;

        // Clamp to 16-bit signed PCM
        if sample > 32767.0 { sample = 32767.0; }
        if sample < -32768.0 { sample = -32768.0; }

        self.hist2 = self.hist1;
        self.hist1 = sample;

        sample as i16
    }
}

fn decode_audio_sector(
    payload: &[u8], // 2304 bytes (18 sound groups of 128 bytes)
    is_stereo: bool,
    left_state: &mut ChannelState,
    right_state: &mut ChannelState,
    out_pcm: &mut Vec<i16>,
) {
    if payload.len() < 18 * 128 {
        return;
    }

    for g in 0..18 {
        let group = &payload[g * 128..(g + 1) * 128];
        let params = &group[0..16];
        let data = &group[16..128]; // 112 bytes = 28 lines of 4 bytes

        // Sound parameters for the 8 sound units (subframes)
        let mut filters = [0usize; 8];
        let mut shifts = [0usize; 8];

        for u in 0..8 {
            // In Level B/C: units 0..3 are at params[4..8], units 4..7 at params[12..16]
            let p_idx = if u < 4 { 4 + u } else { 12 + (u - 4) };
            let p = params[p_idx];
            filters[u] = ((p >> 4) & 0x03) as usize;
            shifts[u] = (p & 0x0F) as usize;
            if shifts[u] > 12 { shifts[u] = 12; }
        }

        // Each line has 4 bytes, covering 8 subframes (4 low nibbles, 4 high nibbles)
        // There are 28 samples per subframe.
        for line in 0..28 {
            let line_offset = line * 4;
            let b0 = data[line_offset + 0];
            let b1 = data[line_offset + 1];
            let b2 = data[line_offset + 2];
            let b3 = data[line_offset + 3];

            let nibbles = [
                b0 & 0x0F, b1 & 0x0F, b2 & 0x0F, b3 & 0x0F,
                (b0 >> 4) & 0x0F, (b1 >> 4) & 0x0F, (b2 >> 4) & 0x0F, (b3 >> 4) & 0x0F,
            ];

            if is_stereo {
                // Stereo: units 0, 2, 4, 6 are Left; units 1, 3, 5, 7 are Right
                for pair in 0..4 {
                    let u_left = pair * 2;
                    let u_right = pair * 2 + 1;

                    let l = left_state.decode_nibble(nibbles[u_left], filters[u_left], shifts[u_left]);
                    let r = right_state.decode_nibble(nibbles[u_right], filters[u_right], shifts[u_right]);

                    out_pcm.push(l);
                    out_pcm.push(r);
                }
            } else {
                // Mono: units 0..7 are sequential samples
                for u in 0..8 {
                    let s = left_state.decode_nibble(nibbles[u], filters[u], shifts[u]);
                    out_pcm.push(s);
                    out_pcm.push(s); // Duplicate to stereo for WAV playback
                }
            }
        }
    }
}

fn write_wav(path: &str, pcm: &[i16], sample_rate: u32, channels: u16) -> std::io::Result<()> {
    let mut f = File::create(path)?;
    let byte_rate = sample_rate * (channels as u32) * 2;
    let block_align = channels * 2;
    let data_len = (pcm.len() * 2) as u32;

    f.write_all(b"RIFF")?;
    f.write_all(&(data_len + 36).to_le_bytes())?;
    f.write_all(b"WAVEfmt ")?;
    f.write_all(&16u32.to_le_bytes())?; // Subchunk1Size
    f.write_all(&1u16.to_le_bytes())?;  // AudioFormat (1 = PCM)
    f.write_all(&channels.to_le_bytes())?;
    f.write_all(&sample_rate.to_le_bytes())?;
    f.write_all(&byte_rate.to_le_bytes())?;
    f.write_all(&block_align.to_le_bytes())?;
    f.write_all(&16u16.to_le_bytes())?; // BitsPerSample
    f.write_all(b"data")?;
    f.write_all(&data_len.to_le_bytes())?;

    for &s in pcm {
        f.write_all(&s.to_le_bytes())?;
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let chd_path = find_file("giochi/Hotel Mario (USA).chd").expect("Hotel Mario CHD not found");
    let mut cd = CdRom::open(&chd_path)?;
    println!("Opened CHD: {} total sectors", cd.total_sectors);

    let mut buf = [0u8; CD_SECTOR_RAW_SIZE];
    let mut audio_sectors = 0;
    let mut first_audio_lba = None;

    let mut pcm_samples = Vec::new();
    let mut left_state = ChannelState::default();
    let mut right_state = ChannelState::default();
    let mut detected_rate = 37800u32;
    let mut detected_stereo = true;

    for lba in 0..10000 {
        if !cd.read_sector(lba, &mut buf) {
            continue;
        }

        let submode = buf[18];
        let is_audio = (submode & 0x04) != 0;

        if is_audio {
            let channel = buf[17];
            let coding = buf[19];

            let freq_code = (coding >> 2) & 0x03;
            let bits_code = (coding >> 4) & 0x03;
            let is_stereo = (coding & 0x40) != 0;

            let rate = if freq_code == 1 { 18900 } else { 37800 };
            detected_rate = rate;
            detected_stereo = is_stereo;

            if first_audio_lba.is_none() {
                first_audio_lba = Some(lba);
                println!(
                    "First audio sector at LBA {}: Channel={}, Submode=0x{:02X}, Coding=0x{:02X} (Rate={}Hz, Stereo={}, Bits={})",
                    lba, channel, submode, coding, rate, is_stereo, if bits_code == 0 { 8 } else { 4 }
                );
            }

            audio_sectors += 1;

            // Decode the audio sector (payload starts at byte 24, 2304 bytes)
            decode_audio_sector(
                &buf[24..24 + 2304],
                is_stereo,
                &mut left_state,
                &mut right_state,
                &mut pcm_samples,
            );

            if audio_sectors <= 10 || audio_sectors % 100 == 0 {
                println!(
                    "Audio Sector #{}: LBA={}, Channel={}, Submode=0x{:02X}, Total PCM samples={}",
                    audio_sectors, lba, channel, submode, pcm_samples.len()
                );
            }

            // Stop after ~1000 audio sectors (~13 seconds of audio!)
            if audio_sectors >= 1000 {
                break;
            }
        }
    }

    println!("\n=== DECODING SUMMARY ===");
    println!("Total audio sectors decoded: {}", audio_sectors);
    println!("Total PCM samples decoded: {}", pcm_samples.len());

    let wav_path = "screenshots_hotel_mario/hotel_mario_intro.wav";
    write_wav(wav_path, &pcm_samples, detected_rate, 2)?;
    println!("Saved audio to {} (Sample rate: {} Hz, Stereo)", wav_path, detected_rate);

    Ok(())
}
