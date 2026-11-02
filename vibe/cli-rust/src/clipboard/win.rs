//! Windows system-clipboard reads: text, images (PNG format / CF_DIB) and file lists (CF_HDROP).

use std::path::PathBuf;

use windows_sys::Win32::System::DataExchange::{
    CloseClipboard, GetClipboardData, OpenClipboard, RegisterClipboardFormatW,
};
use windows_sys::Win32::System::Memory::{GlobalLock, GlobalSize, GlobalUnlock};
use windows_sys::Win32::UI::Shell::{DragQueryFileW, HDROP};

use super::dib_png::dib_to_png;
use crate::paste_image::PNG_MAGIC;

// Hardcoded to avoid pulling in the Win32_System_Ole feature.
const CF_UNICODETEXT: u32 = 13;
const CF_DIB: u32 = 8;
const CF_DIBV5: u32 = 17;
const CF_HDROP: u32 = 15;

/// Read the system clipboard as text; None when empty, non-text or unopenable.
pub fn read_clipboard_text() -> Option<String> {
    with_clipboard(|| {
        let handle = unsafe { GetClipboardData(CF_UNICODETEXT) };
        if handle.is_null() {
            return None;
        }
        unsafe {
            let ptr = GlobalLock(handle) as *const u16;
            if ptr.is_null() {
                return None;
            }
            let mut len = 0usize;
            while *ptr.add(len) != 0 {
                len += 1;
            }
            let text = String::from_utf16_lossy(std::slice::from_raw_parts(ptr, len));
            GlobalUnlock(handle);
            Some(text)
        }
    })
    .flatten()
}

/// Read the clipboard as PNG bytes: the registered "PNG" format first, then
/// CF_DIBV5/CF_DIB converted to PNG. CF_BITMAP-only clipboards are skipped;
/// apps that set a bitmap almost always set a DIB too.
pub fn read_clipboard_image() -> Option<Vec<u8>> {
    with_clipboard(|| {
        let mut name = "PNG".encode_utf16().chain(std::iter::once(0)).collect::<Vec<u16>>();
        let png_format = unsafe { RegisterClipboardFormatW(name.as_mut_ptr()) };
        if png_format != 0 {
            if let Some(data) = clipboard_data(png_format).filter(|d| d.starts_with(PNG_MAGIC)) {
                return Some(data);
            }
        }
        for format in [CF_DIBV5, CF_DIB] {
            if let Some(dib) = clipboard_data(format) {
                if let Some(png) = dib_to_png(&dib) {
                    return Some(png);
                }
            }
        }
        None
    })
    .flatten()
}

/// File paths of a CF_HDROP clipboard, e.g. files copied in Explorer.
pub fn read_clipboard_files() -> Vec<PathBuf> {
    with_clipboard(|| {
        let handle = unsafe { GetClipboardData(CF_HDROP) };
        if handle.is_null() {
            return Vec::new();
        }
        let hdrop = handle as HDROP;
        let count = unsafe { DragQueryFileW(hdrop, u32::MAX, std::ptr::null_mut(), 0) };
        let mut paths = Vec::new();
        for index in 0..count {
            let len = unsafe { DragQueryFileW(hdrop, index, std::ptr::null_mut(), 0) };
            if len == 0 {
                continue;
            }
            let mut buf = vec![0u16; len as usize + 1];
            let written = unsafe { DragQueryFileW(hdrop, index, buf.as_mut_ptr(), buf.len() as u32) };
            if written > 0 {
                paths.push(PathBuf::from(String::from_utf16_lossy(&buf[..written as usize])));
            }
        }
        // No DragFinish: the clipboard owns the HDROP.
        paths
    })
    .unwrap_or_default()
}

/// Run `f` with the clipboard open; None when it cannot be opened.
fn with_clipboard<T>(f: impl FnOnce() -> T) -> Option<T> {
    unsafe {
        if OpenClipboard(std::ptr::null_mut()) == 0 {
            return None;
        }
        let out = f();
        CloseClipboard();
        Some(out)
    }
}

/// Copy the bytes of `format` out of the open clipboard; the handle stays owned by the clipboard.
fn clipboard_data(format: u32) -> Option<Vec<u8>> {
    unsafe {
        let handle = GetClipboardData(format);
        if handle.is_null() {
            return None;
        }
        let ptr = GlobalLock(handle);
        if ptr.is_null() {
            return None;
        }
        let size = GlobalSize(handle);
        let data = if size > 0 {
            std::slice::from_raw_parts(ptr as *const u8, size).to_vec()
        } else {
            Vec::new()
        };
        GlobalUnlock(handle);
        Some(data)
    }
}
