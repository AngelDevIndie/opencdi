use std::fs::File;
use chd::Chd;
use chd::metadata::Metadata;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let file = File::open("../file utili/giochi/Hotel Mario (USA).chd")?;
    let mut chd = Chd::open(file, None)?;

    println!("Hunk count: {}", chd.header().hunk_count());
    println!("Hunk size: {}", chd.header().hunk_size());

    let metadatas: Vec<Metadata> = chd.metadata_refs().try_into()?;
    for meta in metadatas {
        let tag = meta.metatag;
        let tag_bytes = tag.to_be_bytes();
        let tag_str = String::from_utf8_lossy(&tag_bytes);
        println!("Tag: '{}' (0x{:08X})", tag_str, tag);
        if let Ok(s) = String::from_utf8(meta.value.clone()) {
            println!("  Value: {}", s);
        } else {
            println!("  Hex: {:02X?}", &meta.value[..meta.value.len().min(64)]);
        }
    }
    Ok(())
}
