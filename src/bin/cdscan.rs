//! Scansiona il CHD cercando header moduli OS-9 (4A FC) a inizio settore dati.
//! Bypass del filesystem CD-RTOS: trova LBA di 'play' e verifica header.
#[path = "../cdrom.rs"]
mod cdrom;
use crate::cdrom::{CdRom, CD_SECTOR_RAW_SIZE};

fn find_file(rel: &str) -> Option<std::path::PathBuf> {
    for c in [std::path::Path::new(rel).to_path_buf(), std::path::Path::new("..").join(rel),
        std::path::Path::new("file utili").join(rel), std::path::Path::new("..").join("file utili").join(rel)] {
        if c.exists() { return Some(c); }
    }
    None
}

fn mod_name(data: &[u8]) -> Option<(String, u32)> {
    if data.len() < 20 || data[0] != 0x4A || data[1] != 0xFC { return None; }
    let size = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);
    if size < 20 || size > 2_000_000 { return None; }
    let name_off = u32::from_be_bytes([data[12], data[13], data[14], data[15]]) as usize;
    if name_off + 2 >= data.len() || name_off as u32 >= size { return None; }
    let mut name = String::new();
    for &b in &data[name_off..] {
        if b == 0 { break; }
        if !(0x20..0x7E).contains(&b) || name.len() > 40 { return None; }
        name.push(b as char);
    }
    if name.is_empty() { return None; }
    Some((name, size))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let game = if args.len() > 1 { args[1].clone() } else { "giochi/Hotel Mario (USA).chd".to_string() };
    let only_play = args.iter().any(|a| a == "--play-only");
    let mut cd = CdRom::open(find_file(&game).expect("chd not found"))?;
    println!("total_sectors={}", cd.total_sectors);
    let mut found: Vec<(u32, String, u32)> = Vec::new();
    let mut buf = [0u8; CD_SECTOR_RAW_SIZE];
    let mut form1_count = 0u32;
    for lba in 0..cd.total_sectors {
        if !cd.read_sector(lba, &mut buf) { continue; }
        // submode a byte 18: bit5 (0x20) = Form1? bit4 EOR/EOF... usiamo euristica:
        // i dati file stanno a offset 24; per Form1 sono 2048 byte.
        let submode = buf[18];
        let is_form1 = (submode & 0x20) == 0;
        if is_form1 { form1_count += 1; }
        let data = &buf[24..];
        if let Some((name, size)) = mod_name(data) {
            // sanity: submode deve avere bit data (0x40?) o video? accetta tutto ma logga submode
            println!("LBA {:6} submode=0x{:02X} mod='{}' size={}", lba, submode, name, size);
            found.push((lba, name, size));
            if only_play && found.iter().any(|(_, n, _)| n == "play") && lba > 20000 {
                break;
            }
        }
    }
    println!("--- riepilogo: {} moduli, settori form1~{} ---", found.len(), form1_count);
    let plays: Vec<_> = found.iter().filter(|(_, n, _)| n == "play").collect();
    if plays.is_empty() {
        println!("NESSUN modulo 'play' trovato a inizio settore!");
        // fallback: cerca 'play' come stringa nei dati (può partire mid-sector se file non allineato? raro)
        println!("(file potrebbe non iniziare a un sync 4AFC in testa settore: da verificare con dir CD-RTOS)");
    } else {
        for (lba, _, size) in plays {
            println!("PLAY a LBA={} size={}", lba, size);
        }
    }
    Ok(())
}
