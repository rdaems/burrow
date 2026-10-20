// BURROW_SHOT=<file.png>: burrow saves what its window shows to that file once a second, so a Claude
// session working on the room can look at it (it reads its own pixels: no screen-recording permission).
// The PNG is written by hand, uncompressed, so this needs no crate.

use std::io::Write;

pub fn tick(ctx: &egui::Context, path: &str) {
    static LAST: std::sync::Mutex<f64> = std::sync::Mutex::new(-1.0);
    let now = ctx.input(|i| i.time);
    let mut last = LAST.lock().unwrap();
    if now - *last < 1.0 {
        return;
    }
    *last = now;
    let path = path.to_string();
    ctx.request_screenshot(move |img| {
        let tmp = format!("{path}.tmp");
        if png(&img, &tmp).is_ok() {
            let _ = std::fs::rename(&tmp, &path);
        }
    });
}

fn png(img: &egui::ColorImage, path: &str) -> std::io::Result<()> {
    let [w, h] = img.size;
    let mut raw = Vec::with_capacity((w * 4 + 1) * h);
    for row in img.pixels.chunks(w) {
        raw.push(0);
        for p in row {
            raw.extend_from_slice(&[p.r(), p.g(), p.b(), 255]);
        }
    }
    // zlib with stored (uncompressed) deflate blocks
    let mut z = vec![0x78, 0x01];
    for (i, block) in raw.chunks(65535).enumerate() {
        let last = (i + 1) * 65535 >= raw.len();
        z.push(last as u8);
        z.extend_from_slice(&(block.len() as u16).to_le_bytes());
        z.extend_from_slice(&(!(block.len() as u16)).to_le_bytes());
        z.extend_from_slice(block);
    }
    let (mut a, mut b) = (1u32, 0u32);
    for &x in &raw {
        a = (a + x as u32) % 65521;
        b = (b + a) % 65521;
    }
    z.extend_from_slice(&((b << 16) | a).to_be_bytes());

    let mut out = b"\x89PNG\r\n\x1a\n".to_vec();
    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(&(w as u32).to_be_bytes());
    ihdr.extend_from_slice(&(h as u32).to_be_bytes());
    ihdr.extend_from_slice(&[8, 6, 0, 0, 0]);
    for (t, d) in [(b"IHDR", &ihdr), (b"IDAT", &z), (b"IEND", &Vec::new())] {
        out.extend_from_slice(&(d.len() as u32).to_be_bytes());
        let mut c = t.to_vec();
        c.extend_from_slice(d);
        out.extend_from_slice(&c);
        out.extend_from_slice(&crc32(&c).to_be_bytes());
    }
    std::fs::File::create(path)?.write_all(&out)
}

fn crc32(data: &[u8]) -> u32 {
    let mut c = !0u32;
    for &x in data {
        c ^= x as u32;
        for _ in 0..8 {
            c = if c & 1 != 0 { 0xedb88320 ^ (c >> 1) } else { c >> 1 };
        }
    }
    !c
}
