use std::{cell::Cell, path::{Path, PathBuf}, rc::Rc, sync::Arc, panic::{catch_unwind, AssertUnwindSafe},
    os::windows::ffi::OsStringExt};
use eframe::egui;
use windows::{core::{implement, Ref, Error, w}, Win32::{
    Foundation::{HANDLE, HWND, POINTL, E_FAIL, E_INVALIDARG},
    System::{Com::{IDataObject, FORMATETC, STGMEDIUM, DVASPECT_CONTENT, TYMED_HGLOBAL},
        Ole::*, SystemServices::MODIFIERKEYS_FLAGS},
    UI::{Shell::{DragQueryFileW, HDROP}, WindowsAndMessaging::{GetPropW, SetPropW, RemovePropW}},
}};
use super::input_queue::InputQueue;

#[derive(Debug)]
struct PathFile(PathBuf);
impl egui::DroppedFile for PathFile {
    fn path(&self) -> &Path { &self.0 }
    fn bytes(&self) -> std::result::Result<Vec<u8>, String> {
        std::fs::read(&self.0).map_err(|e| format!("Read dropped file {}: {e}", self.0.display()))
    }
}
struct Medium(STGMEDIUM);
impl Drop for Medium { fn drop(&mut self) { unsafe { ReleaseStgMedium(&mut self.0) }; } }
fn file_format() -> FORMATETC {
    FORMATETC { cfFormat: CF_HDROP.0, ptd: std::ptr::null_mut(),
        dwAspect: DVASPECT_CONTENT.0, lindex: -1, tymed: TYMED_HGLOBAL.0 as u32 }
}
fn medium_paths(medium: STGMEDIUM) -> windows::core::Result<Vec<PathBuf>> {
    let medium = Medium(medium);
    if medium.0.tymed != TYMED_HGLOBAL.0 as u32 { return Err(Error::from_hresult(E_FAIL)); }
    let handle = HDROP(unsafe { medium.0.u.hGlobal }.0);
    let count = unsafe { DragQueryFileW(handle, u32::MAX, None) };
    let mut paths = Vec::new();
    paths.try_reserve_exact(count as usize).map_err(|_| Error::from_hresult(E_FAIL))?;
    for index in 0..count {
        let length = unsafe { DragQueryFileW(handle, index, None) };
        let capacity = (length as usize).checked_add(1).ok_or_else(|| Error::from_hresult(E_FAIL))?;
        let mut wide = vec![0u16; capacity];
        if unsafe { DragQueryFileW(handle, index, Some(&mut wide)) } != length {
            return Err(Error::from_hresult(E_FAIL));
        }
        paths.push(PathBuf::from(std::ffi::OsString::from_wide(&wide[..length as usize])));
    }
    Ok(paths)
}
fn paths(object: &IDataObject, input: &InputQueue) -> windows::core::Result<Option<Vec<PathBuf>>> {
    let format = file_format();

    let advertised = unsafe { object.QueryGetData(&format) }.is_ok();
    if input.is_cancelled() { return Ok(None); }
    let medium = match unsafe { object.GetData(&format) } {
        Ok(medium) => medium,
        Err(_) if !advertised || input.is_cancelled() => return Ok(None),
        Err(error) => return Err(error),
    };
    if input.is_cancelled() {
        drop(Medium(medium));
        return Ok(None);
    }
    Ok(Some(medium_paths(medium)?))
}
#[implement(IDropTarget)]
struct Target { input: Rc<InputQueue>, accepted: Cell<bool> }
impl Target_Impl {
    fn retire_hover(&self) -> windows::core::Result<()> {
        self.accepted.set(false);
        if self.input.set_hovered(Vec::new()) { Ok(()) } else { Err(Error::from_hresult(E_FAIL)) }
    }
    fn guard(&self, body: impl FnOnce() -> windows::core::Result<()>) -> windows::core::Result<()> {
        match catch_unwind(AssertUnwindSafe(body)) {
            Ok(Ok(())) => Ok(()),
            Ok(Err(error)) => {

                let _ = self.retire_hover();
                Err(error)
            }
            Err(_) => {
                let _ = self.retire_hover();
                self.input.fail("Native file drop callback panicked.");
                Err(Error::from_hresult(E_FAIL))
            }
        }
    }
}
impl IDropTarget_Impl for Target_Impl {
    fn DragEnter(&self, object: Ref<IDataObject>, _keys: MODIFIERKEYS_FLAGS,
        _point: &POINTL, effect: *mut DROPEFFECT) -> windows::core::Result<()>
    {
        self.guard(|| {
            if effect.is_null() { return Err(Error::from_hresult(E_INVALIDARG)); }
            let allowed = unsafe { (*effect).0 };
            unsafe { *effect = DROPEFFECT_NONE; }
            self.accepted.set(false);
            if self.input.is_cancelled() || allowed & DROPEFFECT_COPY.0 == 0 {
                return self.retire_hover();
            }
            let object = object.ok()?;
            let format = file_format();
            let advertised = unsafe { object.QueryGetData(&format) }.is_ok();
            if self.input.is_cancelled() { return self.retire_hover(); }

            let hover = match unsafe { object.GetData(&format) } {
                Ok(medium) => {
                    if self.input.is_cancelled() {
                        drop(Medium(medium));
                        return self.retire_hover();
                    }
                    let paths = medium_paths(medium)?;
                    if paths.is_empty() { return self.retire_hover(); }
                    paths.into_iter().map(|path| egui::HoveredFile {
                        path: Some(path), mime: String::new(),
                    }).collect()
                }
                Err(_) if advertised && !self.input.is_cancelled() => Vec::new(),
                Err(_) => return self.retire_hover(),
            };
            if self.input.is_cancelled() { return self.retire_hover(); }
            if !self.input.set_hovered(hover) { return Err(Error::from_hresult(E_FAIL)); }
            self.accepted.set(true);
            unsafe { *effect = DROPEFFECT(allowed & DROPEFFECT_COPY.0); }
            Ok(())
        })
    }
    fn DragOver(&self, _keys: MODIFIERKEYS_FLAGS, _point: &POINTL,
        effect: *mut DROPEFFECT) -> windows::core::Result<()>
    {
        self.guard(|| {
            if effect.is_null() { return Err(Error::from_hresult(E_INVALIDARG)); }
            if self.input.is_cancelled() {
                unsafe { *effect = DROPEFFECT_NONE; }
                return self.retire_hover();
            }
            unsafe { *effect = if !self.accepted.get() { DROPEFFECT_NONE }
                else { DROPEFFECT((*effect).0 & DROPEFFECT_COPY.0) }; }
            Ok(())
        })
    }
    fn DragLeave(&self) -> windows::core::Result<()> {
        self.guard(|| {
            self.retire_hover()
        })
    }
    fn Drop(&self, object: Ref<IDataObject>, _keys: MODIFIERKEYS_FLAGS,
        _point: &POINTL, effect: *mut DROPEFFECT) -> windows::core::Result<()>
    {
        self.guard(|| {
            if effect.is_null() { return Err(Error::from_hresult(E_INVALIDARG)); }
            let allowed = unsafe { (*effect).0 };
            unsafe { *effect = DROPEFFECT_NONE; }
            let accepted = self.accepted.replace(false);
            if self.input.is_cancelled() || !accepted || allowed & DROPEFFECT_COPY.0 == 0 {
                return self.retire_hover();
            }
            let result = paths(object.ok()?, &self.input)?;
            if self.input.is_cancelled() { return self.retire_hover(); }
            let paths = match result {
                Some(paths) if !paths.is_empty() => paths,
                _ => return self.retire_hover(),
            };
            let files = paths.into_iter().map(|path| Arc::new(PathFile(path))
                as egui::DroppedFileHandle).collect();
            if !self.input.drop_files(files) { return Err(Error::from_hresult(E_FAIL)); }
            unsafe { *effect = DROPEFFECT(allowed & DROPEFFECT_COPY.0); }
            Ok(())
        })
    }
}
const OWNER_PROPERTY: windows::core::PCWSTR = w!("Rocoflex.Editor.DropOwner.20261104");
pub(super) struct Registration { hwnd: HWND, _target: IDropTarget, token: Box<u8> }
impl Registration {
    pub fn new(hwnd: HWND, input: Rc<InputQueue>) -> std::result::Result<Self, String> {

        unsafe { OleInitialize(None) }.map_err(|e| format!("Enable file drag/drop in this host apartment: {e}"))?;
        if input.is_cancelled() {
            unsafe { OleUninitialize() }; return Err("File drop initialization was cancelled.".into());
        }
        let token = Box::new(0u8);
        let identity = HANDLE((&*token as *const u8).cast_mut().cast());
        if !unsafe { GetPropW(hwnd, OWNER_PROPERTY) }.0.is_null() {
            unsafe { OleUninitialize() }; return Err("The editor already has a native file drop owner.".into());
        }
        if let Err(error) = unsafe { SetPropW(hwnd, OWNER_PROPERTY, Some(identity)) } {
            unsafe { OleUninitialize() }; return Err(format!("Identify editor file drop owner: {error}"));
        }
        let target: IDropTarget = Target { input, accepted: Cell::new(false) }.into();
        if let Err(error) = unsafe { RegisterDragDrop(hwnd, &target) } {
            if unsafe { GetPropW(hwnd, OWNER_PROPERTY) } == identity { let _ = unsafe { RemovePropW(hwnd, OWNER_PROPERTY) }; }
            unsafe { OleUninitialize() };
            return Err(format!("Register editor file drop target: {error}"));
        }
        Ok(Self { hwnd, _target: target, token })
    }
}
impl Drop for Registration {
    fn drop(&mut self) {
        let identity = HANDLE((&*self.token as *const u8).cast_mut().cast());

        if unsafe { GetPropW(self.hwnd, OWNER_PROPERTY) } == identity {
            let _ = unsafe { RevokeDragDrop(self.hwnd) };
            if unsafe { GetPropW(self.hwnd, OWNER_PROPERTY) } == identity {
                let _ = unsafe { RemovePropW(self.hwnd, OWNER_PROPERTY) };
            }
        }
        unsafe { OleUninitialize() };
    }
}

