fn main() {
    let data = std::fs::read("C:/Users/Gian/.gemini/antigravity/brain/47421117-6e2a-4536-8928-519a0c9b76ed/inter_mario_2000.bmp")
        .expect("read bmp");

    let mut cursor_pixels = Vec::new();
    for y in 0..560 {
        for x in 0..768 {
            let idx = 54 + (y * 768 + x) * 4;
            let b = data[idx];
            let g = data[idx+1];
            let r = data[idx+2];
            if r == 0 && g == 0 && b == 0 {
                // Ignore black borders at top/bottom (e.g. y < 20 or y > 530)
                if y >= 20 && y <= 530 {
                    cursor_pixels.push((x, y));
                }
            }
        }
    }

    if !cursor_pixels.is_empty() {
        let min_x = cursor_pixels.iter().map(|p| p.0).min().unwrap();
        let max_x = cursor_pixels.iter().map(|p| p.0).max().unwrap();
        let min_y = cursor_pixels.iter().map(|p| p.1).min().unwrap();
        let max_y = cursor_pixels.iter().map(|p| p.1).max().unwrap();
        println!("Black cursor: X {}..{}, Y {}..{}", min_x, max_x, min_y, max_y);
    }
}
