fn main() {
    let bytes = std::fs::read(r"C:\Users\Gian\.gemini\antigravity\brain\47421117-6e2a-4536-8928-519a0c9b76ed\.tempmediaStorage\a42db4733c1cab32.pdf").expect("pdf");
    let query = std::env::args().nth(1).unwrap_or_else(|| "soundmap".to_string()).to_lowercase();
    let query_bytes = query.as_bytes();
    let lower: Vec<u8> = bytes.iter().map(|&b| b.to_ascii_lowercase()).collect();
    for i in 0..lower.len().saturating_sub(query_bytes.len()) {
        if &lower[i..i+query_bytes.len()] == query_bytes {
            let start = i.saturating_sub(40);
            let end = (i + query_bytes.len() + 60).min(bytes.len());
            let snippet: String = bytes[start..end].iter().map(|&b| if b.is_ascii_graphic() || b == b' ' { b as char } else { '.' }).collect();
            println!("At {}: {}", i, snippet);
        }
    }
}
