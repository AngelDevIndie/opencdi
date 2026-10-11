use std::fs::File;
use chd::Chd;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = "../file utili/giochi/Hotel Mario (USA).chd";
    let file = File::open(path)?;
    let mut chd = Chd::open(file, None)?;

    println!("=== CHD Opened Successfully ===");
    let header = chd.header();
    println!("CHD version: {}", header.version() as u32);
    println!("Hunk count: {}", header.hunk_count());
    println!("Hunk size: {} bytes", header.hunk_size());
    println!("Logical bytes: {}", header.logical_bytes());

    // Read the first hunk (sector 0)
    let mut hunk_buf = chd.get_hunksized_buffer();
    let mut comp_buf = Vec::new();
    let mut hunk = chd.hunk(0)?;
    let bytes_read = hunk.read_hunk_in(&mut comp_buf, &mut hunk_buf)?;
    println!("Read hunk 0 successfully! Bytes read: {}", bytes_read);
    println!("Read hunk 0 successfully! Buffer length: {}", hunk_buf.len());

    // Check first sector sync pattern: 00 FF FF FF FF FF FF FF FF FF FF 00
    if hunk_buf.len() >= 16 {
        println!("First 16 bytes: {:02X?}", &hunk_buf[0..16]);
    }

    Ok(())
}
