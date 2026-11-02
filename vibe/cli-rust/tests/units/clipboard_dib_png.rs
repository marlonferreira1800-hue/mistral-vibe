//! CF_DIB to PNG conversion: PNG structure, pixel round-trip, rejection cases.

use std::io::Read;

use vibe_rs::clipboard::dib_png::dib_to_png;

fn dib(width: i32, height: i32, bpp: u16, compression: u32, pixels: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&40u32.to_le_bytes());
    out.extend_from_slice(&width.to_le_bytes());
    out.extend_from_slice(&height.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&bpp.to_le_bytes());
    out.extend_from_slice(&compression.to_le_bytes());
    out.extend_from_slice(&[0u8; 20]);
    out.extend_from_slice(pixels);
    out
}

fn chunks(png: &[u8]) -> Vec<(String, Vec<u8>)> {
    assert!(png.starts_with(b"\x89PNG\r\n\x1a\n"), "PNG signature");
    let mut rest = &png[8..];
    let mut out = Vec::new();
    while !rest.is_empty() {
        let len = u32::from_be_bytes(rest[..4].try_into().unwrap()) as usize;
        let name = String::from_utf8(rest[4..8].to_vec()).unwrap();
        let data = rest[8..8 + len].to_vec();
        let crc = u32::from_be_bytes(rest[8 + len..12 + len].try_into().unwrap());
        let mut hasher = crc32fast::Hasher::new();
        hasher.update(&rest[4..8]);
        hasher.update(&data);
        assert_eq!(crc, hasher.finalize(), "CRC of chunk {name}");
        out.push((name, data));
        rest = &rest[12 + len..];
    }
    out
}

fn idat_pixels(png: &[u8]) -> (u32, u32, u8, Vec<u8>) {
    let chunks = chunks(png);
    let names: Vec<&str> = chunks.iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(names, ["IHDR", "IDAT", "IEND"]);
    let ihdr = &chunks[0].1;
    let width = u32::from_be_bytes(ihdr[0..4].try_into().unwrap());
    let height = u32::from_be_bytes(ihdr[4..8].try_into().unwrap());
    assert_eq!(ihdr[8], 8, "bit depth");
    let color_type = ihdr[9];
    let mut raw = Vec::new();
    flate2::read::ZlibDecoder::new(&chunks[1].1[..])
        .read_to_end(&mut raw)
        .unwrap();
    (width, height, color_type, raw)
}

#[test]
fn converts_24bpp_bottom_up_dib_to_rgb_png() {
    // Image rows top-to-bottom: [red, green] / [blue, white]; the DIB stores the bottom row first.
    let bottom = [255, 0, 0, 255, 255, 255, 0, 0];
    let top = [0, 0, 255, 0, 255, 0, 0, 0];
    let mut pixels = Vec::new();
    pixels.extend_from_slice(&bottom);
    pixels.extend_from_slice(&top);
    let png = dib_to_png(&dib(2, 2, 24, 0, &pixels)).expect("convert");

    let (width, height, color_type, raw) = idat_pixels(&png);
    assert_eq!((width, height, color_type), (2, 2, 2));
    let expected: &[u8] = &[
        0, 255, 0, 0, 0, 255, 0, // top row RGB: red, green
        0, 0, 0, 255, 255, 255, 255, // bottom row RGB: blue, white
    ];
    assert_eq!(raw, expected);
}

#[test]
fn converts_32bpp_top_down_dib_to_rgba_png() {
    // Negative height: rows are stored top-down; BGRA becomes RGBA with alpha kept.
    let pixels = [
        10, 20, 30, 40, 50, 60, 70, 80, // top row
        90, 100, 110, 120, 130, 140, 150, 160, // bottom row
    ];
    let png = dib_to_png(&dib(2, -2, 32, 0, &pixels)).expect("convert");

    let (width, height, color_type, raw) = idat_pixels(&png);
    assert_eq!((width, height, color_type), (2, 2, 6));
    let expected: &[u8] = &[
        0, 30, 20, 10, 40, 70, 60, 50, 80, 0, 110, 100, 90, 120, 150, 140, 130, 160,
    ];
    assert_eq!(raw, expected);
}

#[test]
fn rejects_unsupported_dibs() {
    assert!(dib_to_png(&[]).is_none());
    assert!(dib_to_png(&dib(2, 2, 24, 0, &[])[..20]).is_none());
    // BI_BITFIELDS is not handled.
    assert!(dib_to_png(&dib(2, 2, 32, 3, &[0u8; 16])).is_none());
    // 8-bpp paletted DIBs are not handled.
    assert!(dib_to_png(&dib(2, 2, 8, 0, &[0u8; 8])).is_none());
    // Non-positive dimensions.
    assert!(dib_to_png(&dib(0, 2, 24, 0, &[])).is_none());
    assert!(dib_to_png(&dib(2, 0, 24, 0, &[])).is_none());
    // Truncated pixel data.
    assert!(dib_to_png(&dib(2, 2, 24, 0, &[0u8; 4])).is_none());
}
