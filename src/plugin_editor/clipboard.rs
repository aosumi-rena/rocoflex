use std::ptr;
use windows::{core::{PCWSTR, w}, Win32::{
    Foundation::{GlobalFree, HANDLE, HGLOBAL, HWND},
    System::{DataExchange::*, Memory::*},
    UI::{Shell::ShellExecuteW, WindowsAndMessaging::SW_SHOWNORMAL},
}};
use eframe::egui;
type Result<T> = std::result::Result<T, String>;
const UNICODE_TEXT: u32 = 13;
const DIB_V5: u32 = 17;
struct Open;
impl Open {
    fn new(hwnd: HWND) -> Result<Self> {
        unsafe { OpenClipboard(Some(hwnd)) }.map_err(|e| format!("Open clipboard: {e}"))?;
        Ok(Self)
    }
}
impl Drop for Open { fn drop(&mut self) { let _ = unsafe { CloseClipboard() }; } }
struct OwnedMemory(Option<HGLOBAL>);
impl Drop for OwnedMemory {
    fn drop(&mut self) { if let Some(memory) = self.0.take() { let _ = unsafe { GlobalFree(Some(memory)) }; } }
}
fn publish(hwnd: HWND, format: u32, bytes: &[u8]) -> Result<()> {
    let allocation = unsafe { GlobalAlloc(GMEM_MOVEABLE, bytes.len()) }.map_err(|e| e.to_string())?;
    let mut memory = OwnedMemory(Some(allocation));
    let address = unsafe { GlobalLock(allocation) };
    if address.is_null() { return Err("Lock clipboard allocation failed.".into()); }
    unsafe { ptr::copy_nonoverlapping(bytes.as_ptr(), address.cast::<u8>(), bytes.len()); }
    let _ = unsafe { GlobalUnlock(allocation) };
    let _open = Open::new(hwnd)?;
    unsafe { EmptyClipboard() }.map_err(|e| e.to_string())?;
    unsafe { SetClipboardData(format, Some(HANDLE(allocation.0))) }.map_err(|e| e.to_string())?;
    memory.0 = None;
    Ok(())
}
pub(super) fn write_text(hwnd: HWND, text: &str) -> Result<()> {
    let words: Vec<u16> = text.encode_utf16().chain(Some(0)).collect();
    let bytes = unsafe { std::slice::from_raw_parts(words.as_ptr().cast::<u8>(), words.len() * 2) };
    publish(hwnd, UNICODE_TEXT, bytes)
}
pub(super) fn read_text(hwnd: HWND) -> Result<Option<String>> {
    let _open = Open::new(hwnd)?;
    if unsafe { IsClipboardFormatAvailable(UNICODE_TEXT) }.is_err() { return Ok(None); }
    let handle = unsafe { GetClipboardData(UNICODE_TEXT) }.map_err(|e| e.to_string())?;
    let global = HGLOBAL(handle.0);
    let length = unsafe { GlobalSize(global) };
    if length < 2 || length > 32 * 1024 * 1024 || length % 2 != 0 {
        return Err("Clipboard Unicode text has invalid bounds.".into());
    }
    let address = unsafe { GlobalLock(global) };
    if address.is_null() { return Err("Lock clipboard text failed.".into()); }
    let words = unsafe { std::slice::from_raw_parts(address.cast::<u16>(), length / 2) };
    let result = words.iter().position(|&word| word == 0)
        .ok_or_else(|| "Clipboard Unicode text is not terminated.".to_owned())
        .and_then(|end| String::from_utf16(&words[..end]).map_err(|e| e.to_string()));
    let _ = unsafe { GlobalUnlock(global) };
    result.map(Some)
}
pub(super) fn write_image(hwnd: HWND, image: &egui::ColorImage) -> Result<()> {
    let [width, height] = image.size;
    let pixels = width.checked_mul(height).ok_or("Clipboard image dimensions overflow")?;
    let payload = pixels.checked_mul(4).ok_or("Clipboard image byte size overflows")?;
    if width == 0 || height == 0 || width > i32::MAX as usize || height > i32::MAX as usize
        || payload > u32::MAX as usize || pixels != image.pixels.len()
    { return Err("Clipboard image has invalid dimensions.".into()); }
    let mut bytes = Vec::new();
    bytes.try_reserve_exact(124 + payload).map_err(|e| e.to_string())?;
    bytes.resize(124 + payload, 0);
    let put = |bytes: &mut [u8], offset: usize, value: u32| {
        bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    };
    put(&mut bytes, 0, 124);
    put(&mut bytes, 4, width as u32);
    put(&mut bytes, 8, (-(height as i32)) as u32);
    bytes[12..14].copy_from_slice(&1u16.to_le_bytes());
    bytes[14..16].copy_from_slice(&32u16.to_le_bytes());
    put(&mut bytes, 16, 3);
    put(&mut bytes, 20, payload as u32);
    for (offset, mask) in [(40, 0x00ff0000), (44, 0x0000ff00), (48, 0x000000ff), (52, 0xff000000)] {
        put(&mut bytes, offset, mask);
    }
    put(&mut bytes, 56, 0x73524742);
    put(&mut bytes, 108, 4);
    for (destination, color) in bytes[124..].chunks_exact_mut(4).zip(&image.pixels) {
        let [red, green, blue, alpha] = color.to_srgba_unmultiplied();
        destination.copy_from_slice(&[blue, green, red, alpha]);
    }
    publish(hwnd, DIB_V5, &bytes)
}
pub(super) fn open_url(hwnd: HWND, url: &str) -> Result<()> {
    if url.contains('\0') { return Err("A URL cannot contain a null character.".into()); }
    let wide: Vec<u16> = url.encode_utf16().chain(Some(0)).collect();
    let result = unsafe { ShellExecuteW(Some(hwnd), w!("open"), PCWSTR(wide.as_ptr()),
        PCWSTR::null(), PCWSTR::null(), SW_SHOWNORMAL) };
    if result.0 as isize <= 32 { return Err(format!("Open URL failed ({:?}).", result.0)); }
    Ok(())
}
