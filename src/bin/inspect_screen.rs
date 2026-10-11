fn main() {
    let bmp = std::fs::read("C:/Users/Gian/.gemini/antigravity/brain/47421117-6e2a-4536-8928-519a0c9b76ed/debug_mario_4200.bmp").expect("read bmp");
    let w = i32::from_le_bytes(bmp[18..22].try_into().unwrap());
    let h = i32::from_le_bytes(bmp[22..26].try_into().unwrap());
    let bpp = u16::from_le_bytes(bmp[28..30].try_into().unwrap());
    println!("BMP w={}, h={}, bpp={}", w, h, bpp);

    // Let's sample pixels at various positions
    for y in (200..450).step_by(5) {
        for x in (500..750).step_by(5) {
            let offset = 54 + ((y * w + x) * 4) as usize;
            let r = bmp[offset+2];
            let g = bmp[offset+1];
            let b = bmp[offset];
            if (r as i32 - g as i32).abs() > 10 || (r as i32 - b as i32).abs() > 10 {
                println!("Color pixel at x={}, y={}: R={:02X} G={:02X} B={:02X}", x, y, r, g, b);
            }
        }
    }
}
