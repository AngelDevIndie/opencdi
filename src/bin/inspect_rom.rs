use std::fs;

fn main() {
    let rom = fs::read("../file utili/bios/cdi220b.rom")
        .or_else(|_| fs::read("bios/cdi220b.rom"))
        .expect("ROM not found");
    let target = b"Error";
    for i in 0..rom.len().saturating_sub(target.len()) {
        if &rom[i..i + target.len()] == target {
            let addr = 0x400000 + i;
            let mut s = String::new();
            for k in 0..40 {
                if i + k < rom.len() {
                    let b = rom[i + k];
                    if b >= 32 && b <= 126 {
                        s.push(b as char);
                    } else {
                        break;
                    }
                }
            }
            println!("Found at 0x{:06X}: '{}'", addr, s);
        }
    }
}
