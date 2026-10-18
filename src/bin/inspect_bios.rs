use opencdi::cdrom::{CdRom, CD_SECTOR_RAW_SIZE};

fn inspect_disc(path: &str) {
    println!("=== Inspecting {:?} ===", path);
    let mut cd = CdRom::open(path).expect("open cd");
    let mut s16 = [0u8; CD_SECTOR_RAW_SIZE];
    if cd.read_sector(16, &mut s16) {
        let payload = &s16[24..24 + 2048];
        println!("Header standard: {:?}", String::from_utf8_lossy(&payload[1..6]));
        println!("System ID: {:?}", String::from_utf8_lossy(&payload[8..40]));
        println!("Volume ID: {:?}", String::from_utf8_lossy(&payload[40..72]));
        println!("Publisher ID: {:?}", String::from_utf8_lossy(&payload[318..446]));
        println!("Preparer ID: {:?}", String::from_utf8_lossy(&payload[446..574]));
        println!("Application ID: {:?}", String::from_utf8_lossy(&payload[574..702]));
        println!("Root dir record at offset 156:");
        let root = &payload[156..156 + 38];
        let root_lbn = u32::from_be_bytes([root[6], root[7], root[8], root[9]]);
        let root_size = u32::from_be_bytes([root[14], root[15], root[16], root[17]]);
        println!("Root dir LBN: {}, size: {}", root_lbn, root_size);

        let mut root_sec = [0u8; CD_SECTOR_RAW_SIZE];
        if cd.read_sector(root_lbn, &mut root_sec) {
            let rdata = &root_sec[24..24 + 2048];
            println!("Root directory entries:");
            let mut off = 0;
            while off + 32 <= rdata.len() {
                let name_len = rdata[off + 32 - 1]; // check OS-9 dir entry structure
                let name = String::from_utf8_lossy(&rdata[off..off + 28]);
                if rdata[off] != 0 {
                    println!("  File: {:?}", name.trim_matches(char::from(0)));
                }
                off += 32;
            }
        }
    }
}

fn main() {
    inspect_disc("../file utili/giochi/Mutant Rampage - Bodyslam (USA).chd");
}
