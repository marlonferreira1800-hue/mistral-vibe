//! CF_DIB/CF_DIBV5 to PNG conversion, pure Rust so it is unit-testable off Windows.

use std::io::Write;

const HEADER_LEN: usize = 40;
const BI_RGB: u32 = 0;

/// Convert a 24/32-bpp uncompressed DIB to PNG bytes; None when unsupported.
pub fn dib_to_png(dib: &[u8]) -> Option<Vec<u8>> {
    let header = dib.get(..HEADER_LEN)?;
    let header_size = u32::from_le_bytes(header[0..4].try_into().ok()?) as usize;
    if header_size < HEADER_LEN || header_size > dib.len() {
        return None;
    }
    let width = i32::from_le_bytes(header[4..8].try_into().ok()?);
    let raw_height = i32::from_le_bytes(header[8..12].try_into().ok()?);
    let planes = u16::from_le_bytes(header[12..14].try_into().ok()?);
    let bpp = u16::from_le_bytes(header[14..16].try_into().ok()?);
    let compression = u32::from_le_bytes(header[16..20].try_into().ok()?);
    if planes != 1 || compression != BI_RGB || width <= 0 || raw_height == 0 {
        return None;
    }
    let bytes_pp = match bpp {
        24 => 3,
        32 => 4,
        _ => return None,
    };
    let width = width as usize;
    let top_down = raw_height < 0;
    let height = raw_height.unsigned_abs() as usize;
    let row = (width * bytes_pp + 3) / 4 * 4;
    if dib.len() < header_size + row * height {
        return None;
    }
    let mut pixels = Vec::with_capacity(width * height * bytes_pp);
    for y in 0..height {
        let src_y = if top_down { y } else { height - 1 - y };
        let row_start = header_size + src_y * row;
        for x in 0..width {
            let p = row_start + x * bytes_pp;
            pixels.extend_from_slice(&[dib[p + 2], dib[p + 1], dib[p]]);
            if bytes_pp == 4 {
                pixels.push(dib[p + 3]);
            }
        }
    }
    let color_type = if bytes_pp == 4 { 6 } else { 2 };
    Some(encode_png(width as u32, height as u32, color_type, &pixels))
}

/// Minimal PNG encoder: 8-bit RGB (color type 2) or RGBA (color type 6), filter 0 rows.
fn encode_png(width: u32, height: u32, color_type: u8, pixels: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(b"\x89PNG\r\n\x1a\n");
    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend_from_slice(&width.to_be_bytes());
    ihdr.extend_from_slice(&height.to_be_bytes());
    ihdr.extend_from_slice(&[8, color_type, 0, 0, 0]);
    write_chunk(&mut out, b"IHDR", &ihdr);
    let channels = if color_type == 6 { 4 } else { 3 };
    let stride = width as usize * channels;
    let mut raw = Vec::with_capacity((stride + 1) * height as usize);
    for y in 0..height as usize {
        raw.push(0);
        raw.extend_from_slice(&pixels[y * stride..(y + 1) * stride]);
    }
    let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    let _ = encoder.write_all(&raw);
    let idat = encoder.finish().unwrap_or_default();
    write_chunk(&mut out, b"IDAT", &idat);
    write_chunk(&mut out, b"IEND", &[]);
    out
}

fn write_chunk(out: &mut Vec<u8>, name: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    out.extend_from_slice(name);
    out.extend_from_slice(data);
    let mut crc = crc32fast::Hasher::new();
    crc.update(name);
    crc.update(data);
    out.extend_from_slice(&crc.finalize().to_be_bytes());
}
