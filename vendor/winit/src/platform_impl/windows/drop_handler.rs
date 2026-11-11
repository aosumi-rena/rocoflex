// Modified for Rocoflex: Windows file-drop event delivery.
use std::ffi::{c_void, OsString};
use std::os::windows::ffi::OsStringExt;
use std::path::PathBuf;
use std::ptr;
use std::sync::atomic::{AtomicUsize, Ordering};

use windows_sys::core::{IUnknown, GUID, HRESULT};
use windows_sys::Win32::Foundation::{E_INVALIDARG, E_NOINTERFACE, E_POINTER, HWND, POINTL, S_OK};
use windows_sys::Win32::System::Com::{IDataObject, DVASPECT_CONTENT, FORMATETC, STGMEDIUM, TYMED_HGLOBAL};
use windows_sys::Win32::System::Ole::{ReleaseStgMedium, CF_HDROP, DROPEFFECT_COPY, DROPEFFECT_NONE};
use windows_sys::Win32::UI::Shell::{DragQueryFileW, HDROP};

use crate::platform_impl::platform::definitions::{
    IDataObjectVtbl, IDropTarget, IDropTargetVtbl, IUnknownVtbl,
};
use crate::platform_impl::platform::WindowId;

use crate::event::Event;
use crate::window::WindowId as RootWindowId;

// IDropTarget inherits IUnknown; the same interface pointer is its COM identity.
// GUID values match the Windows SDK IUnknown and IDropTarget declarations.
const IID_IUNKNOWN: GUID = GUID::from_u128(0x00000000_0000_0000_c000_000000000046);
const IID_IDROP_TARGET: GUID = GUID::from_u128(0x00000122_0000_0000_c000_000000000046);

fn same_guid(a: &GUID, b: &GUID) -> bool {
    a.data1 == b.data1 && a.data2 == b.data2 && a.data3 == b.data3 && a.data4 == b.data4
}

// GetData transfers a medium with an optional source-owned releaser.
struct DropMedium(STGMEDIUM);
impl Drop for DropMedium {
    fn drop(&mut self) { unsafe { ReleaseStgMedium(&mut self.0) }; }
}

#[repr(C)]
pub struct FileDropHandlerData {
    pub interface: IDropTarget,
    refcount: AtomicUsize,
    window: HWND,
    send_event: Box<dyn Fn(Event<()>)>,
    cursor_effect: u32,
    accepts_file_drop: bool,
    hover_published: bool,
}

pub struct FileDropHandler {
    pub data: *mut FileDropHandlerData,
}

#[allow(non_snake_case)]
impl FileDropHandler {
    pub fn new(window: HWND, send_event: Box<dyn Fn(Event<()>)>) -> FileDropHandler {
        let data = Box::new(FileDropHandlerData {
            interface: IDropTarget { lpVtbl: &DROP_TARGET_VTBL as *const IDropTargetVtbl },
            refcount: AtomicUsize::new(1),
            window,
            send_event,
            cursor_effect: DROPEFFECT_NONE,
            accepts_file_drop: false,
            hover_published: false,
        });
        FileDropHandler { data: Box::into_raw(data) }
    }

    // One COM identity: IUnknown and IDropTarget share this interface pointer.
    pub unsafe extern "system" fn QueryInterface(
        this: *mut IUnknown,
        riid: *const GUID,
        ppvObject: *mut *mut c_void,
    ) -> HRESULT {
        if ppvObject.is_null() { return E_POINTER; }
        unsafe { *ppvObject = ptr::null_mut(); }
        if this.is_null() || riid.is_null() { return E_POINTER; }
        let requested = unsafe { &*riid };
        if !same_guid(requested, &IID_IUNKNOWN) && !same_guid(requested, &IID_IDROP_TARGET) {
            return E_NOINTERFACE;
        }
        unsafe {
            Self::AddRef(this);
            *ppvObject = this.cast();
        }
        S_OK
    }

    pub unsafe extern "system" fn AddRef(this: *mut IUnknown) -> u32 {
        let drop_handler_data = unsafe { Self::from_interface(this) };
        let count = drop_handler_data.refcount.fetch_add(1, Ordering::Release) + 1;
        count as u32
    }

    pub unsafe extern "system" fn Release(this: *mut IUnknown) -> u32 {
        let drop_handler = unsafe { Self::from_interface(this) };
        let count = drop_handler.refcount.fetch_sub(1, Ordering::Release) - 1;
        if count == 0 {
            // Destroy the underlying data
            drop(unsafe { Box::from_raw(drop_handler as *mut FileDropHandlerData) });
        }
        count as u32
    }

    pub unsafe extern "system" fn DragEnter(
        this: *mut IDropTarget,
        pDataObj: *const IDataObject,
        _grfKeyState: u32,
        _pt: POINTL,
        pdwEffect: *mut u32,
    ) -> HRESULT {
        use crate::event::WindowEvent::HoveredFile;
        if pdwEffect.is_null() { return E_INVALIDARG; }
        let drop_handler = unsafe { Self::from_interface(this) };
        let allowed = unsafe { *pdwEffect };
        drop_handler.accepts_file_drop = false;
        drop_handler.hover_published = false;
        if allowed & DROPEFFECT_COPY != 0 {
            // Preserve the existing hover path when data is immediately
            // available, while releasing the returned storage medium.
            if let Some(paths) = unsafe { Self::file_paths(pDataObj) } {
                drop_handler.accepts_file_drop = true;
                drop_handler.hover_published = !paths.is_empty();
                for filename in paths {
                    drop_handler.send_event(Event::WindowEvent {
                        window_id: RootWindowId(WindowId(drop_handler.window)),
                        event: HoveredFile(filename),
                    });
                }
            } else {
                // A source may advertise CF_HDROP but defer rendering until Drop.
                drop_handler.accepts_file_drop = unsafe { Self::accepts_file_format(pDataObj) };
            }
        }
        drop_handler.cursor_effect = if drop_handler.accepts_file_drop {
            DROPEFFECT_COPY
        } else {
            DROPEFFECT_NONE
        };
        unsafe { *pdwEffect = drop_handler.cursor_effect; }
        S_OK
    }

    pub unsafe extern "system" fn DragOver(
        this: *mut IDropTarget,
        _grfKeyState: u32,
        _pt: POINTL,
        pdwEffect: *mut u32,
    ) -> HRESULT {
        if pdwEffect.is_null() { return E_INVALIDARG; }
        let drop_handler = unsafe { Self::from_interface(this) };
        unsafe { *pdwEffect &= drop_handler.cursor_effect; }
        S_OK
    }

    pub unsafe extern "system" fn DragLeave(this: *mut IDropTarget) -> HRESULT {
        let drop_handler = unsafe { Self::from_interface(this) };
        drop_handler.accepts_file_drop = false;
        drop_handler.cursor_effect = DROPEFFECT_NONE;
        drop_handler.clear_hover();
        S_OK
    }

    pub unsafe extern "system" fn Drop(
        this: *mut IDropTarget,
        pDataObj: *const IDataObject,
        _grfKeyState: u32,
        _pt: POINTL,
        pdwEffect: *mut u32,
    ) -> HRESULT {
        use crate::event::WindowEvent::DroppedFile;
        if pdwEffect.is_null() { return E_INVALIDARG; }
        let allowed = unsafe { *pdwEffect };
        unsafe { *pdwEffect = DROPEFFECT_NONE; }
        let drop_handler = unsafe { Self::from_interface(this) };
        let accepted = std::mem::replace(&mut drop_handler.accepts_file_drop, false);
        drop_handler.cursor_effect = DROPEFFECT_NONE;
        if !accepted || allowed & DROPEFFECT_COPY == 0 {
            drop_handler.clear_hover();
            return S_OK;
        }

        // Collect all paths before publishing, and honor medium ownership.
        let Some(paths) = (unsafe { Self::file_paths(pDataObj) }) else {
            drop_handler.clear_hover();
            return S_OK;
        };
        if paths.is_empty() {
            drop_handler.clear_hover();
            return S_OK;
        }
        // DroppedFile itself clears the corresponding egui hover state.
        drop_handler.hover_published = false;
        for filename in paths {
            drop_handler.send_event(Event::WindowEvent {
                window_id: RootWindowId(WindowId(drop_handler.window)),
                event: DroppedFile(filename),
            });
        }
        unsafe { *pdwEffect = DROPEFFECT_COPY; }
        S_OK
    }

    unsafe fn from_interface<'a, InterfaceT>(this: *mut InterfaceT) -> &'a mut FileDropHandlerData {
        unsafe { &mut *(this as *mut _) }
    }

    fn file_format() -> FORMATETC {
        FORMATETC {
            cfFormat: CF_HDROP,
            ptd: ptr::null_mut(),
            dwAspect: DVASPECT_CONTENT,
            lindex: -1,
            tymed: TYMED_HGLOBAL as u32,
        }
    }

    unsafe fn accepts_file_format(data_obj: *const IDataObject) -> bool {
        if data_obj.is_null() { return false; }
        let query = unsafe { (*(*data_obj).cast::<IDataObjectVtbl>()).QueryGetData };
        unsafe { query(data_obj as *mut _, &Self::file_format()) == S_OK }
    }

    unsafe fn file_paths(data_obj: *const IDataObject) -> Option<Vec<PathBuf>> {
        if data_obj.is_null() { return None; }
        let mut medium: STGMEDIUM = unsafe { std::mem::zeroed() };
        let get_data = unsafe { (*(*data_obj).cast::<IDataObjectVtbl>()).GetData };
        if unsafe { get_data(data_obj as *mut _, &Self::file_format(), &mut medium) } != S_OK {
            return None;
        }
        let medium = DropMedium(medium);
        if medium.0.tymed != TYMED_HGLOBAL as u32 { return None; }
        let hdrop = unsafe { medium.0.u.hGlobal as HDROP };
        if hdrop == 0 { return None; }

        let count = unsafe { DragQueryFileW(hdrop, u32::MAX, ptr::null_mut(), 0) };
        let mut paths = Vec::new();
        paths.try_reserve_exact(count as usize).ok()?;
        for index in 0..count {
            let len = unsafe { DragQueryFileW(hdrop, index, ptr::null_mut(), 0) };
            if len == 0 { return None; }
            let capacity = len.checked_add(1)?;
            let mut wide = Vec::new();
            wide.try_reserve_exact(capacity as usize).ok()?;
            wide.resize(capacity as usize, 0u16);
            if unsafe { DragQueryFileW(hdrop, index, wide.as_mut_ptr(), capacity) } != len {
                return None;
            }
            paths.push(OsString::from_wide(&wide[..len as usize]).into());
        }
        Some(paths)
    }
}

impl FileDropHandlerData {
    fn send_event(&self, event: Event<()>) {
        (self.send_event)(event);
    }

    fn clear_hover(&mut self) {
        if std::mem::replace(&mut self.hover_published, false) {
            self.send_event(Event::WindowEvent {
                window_id: RootWindowId(WindowId(self.window)),
                event: crate::event::WindowEvent::HoveredFileCancelled,
            });
        }
    }
}

impl Drop for FileDropHandler {
    fn drop(&mut self) {
        unsafe {
            FileDropHandler::Release(self.data as *mut IUnknown);
        }
    }
}

static DROP_TARGET_VTBL: IDropTargetVtbl = IDropTargetVtbl {
    parent: IUnknownVtbl {
        QueryInterface: FileDropHandler::QueryInterface,
        AddRef: FileDropHandler::AddRef,
        Release: FileDropHandler::Release,
    },
    DragEnter: FileDropHandler::DragEnter,
    DragOver: FileDropHandler::DragOver,
    DragLeave: FileDropHandler::DragLeave,
    Drop: FileDropHandler::Drop,
};


#[cfg(test)]
mod deferred_file_drop_tests {
    use super::*;
    use std::{cell::{Cell, RefCell}, rc::Rc};
    use windows_sys::Win32::{
        Foundation::{BOOL, E_FAIL, E_NOINTERFACE, E_NOTIMPL, HGLOBAL, POINT, DV_E_FORMATETC, GlobalFree},
        System::{
            Com::{IAdviseSink, IEnumFORMATETC, IEnumSTATDATA},
            Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE, GMEM_ZEROINIT},
            Ole::DROPEFFECT_MOVE,
        },
        UI::Shell::DROPFILES,
    };

    // In-memory IDataObject: advertises CF_HDROP, but may refuse GetData until
    // the test allows the drop. No HWND, message loop or filesystem is used.
    #[repr(C)]
    struct MediumOwner {
        vtable: *const IUnknownVtbl,
        handle: Cell<HGLOBAL>,
        releases: Cell<u32>,
    }

    unsafe extern "system" fn owner_query(
        _: *mut IUnknown, _: *const GUID, _: *mut *mut c_void,
    ) -> HRESULT { E_NOINTERFACE }
    unsafe extern "system" fn owner_addref(_: *mut IUnknown) -> u32 { 1 }
    unsafe extern "system" fn owner_release(this: *mut IUnknown) -> u32 {
        let owner = unsafe { &*this.cast::<MediumOwner>() };
        owner.releases.set(owner.releases.get() + 1);
        let handle = owner.handle.replace(ptr::null_mut());
        if !handle.is_null() { unsafe { GlobalFree(handle) }; }
        1
    }
    static OWNER_VTABLE: IUnknownVtbl = IUnknownVtbl {
        QueryInterface: owner_query, AddRef: owner_addref, Release: owner_release,
    };

    impl MediumOwner {
        fn with_path(path: Option<&str>) -> Self {
            match path {
                Some(path) => Self::with_paths(&[path]),
                None => Self::with_paths(&[]),
            }
        }
        fn with_paths(paths: &[&str]) -> Self {
            let mut names = Vec::<u16>::new();
            for path in paths {
                names.extend(path.encode_utf16());
                names.push(0);
            }
            names.push(0);
            if names.len() == 1 { names.push(0); }
            let size = std::mem::size_of::<DROPFILES>() + names.len() * 2;
            let handle = unsafe { GlobalAlloc(GMEM_MOVEABLE | GMEM_ZEROINIT, size) };
            assert!(!handle.is_null());
            let memory = unsafe { GlobalLock(handle) }.cast::<u8>();
            assert!(!memory.is_null());
            unsafe {
                memory.cast::<DROPFILES>().write(DROPFILES {
                    pFiles: std::mem::size_of::<DROPFILES>() as u32,
                    pt: POINT { x: 0, y: 0 },
                    fNC: 0,
                    fWide: 1,
                });
                std::ptr::copy_nonoverlapping(
                    names.as_ptr(),
                    memory.add(std::mem::size_of::<DROPFILES>()).cast::<u16>(),
                    names.len(),
                );
                GlobalUnlock(handle);
            }
            Self { vtable: &OWNER_VTABLE, handle: Cell::new(handle), releases: Cell::new(0) }
        }
    }

    impl MediumOwner {
        fn refill(&self, path: &str) {
            assert!(self.handle.get().is_null());
            let replacement = Self::with_path(Some(path));
            self.handle.set(replacement.handle.replace(ptr::null_mut()));
        }
    }

    impl Drop for MediumOwner {
        fn drop(&mut self) {
            let handle = self.handle.replace(ptr::null_mut());
            if !handle.is_null() { unsafe { GlobalFree(handle) }; }
        }
    }

    #[repr(C)]
    struct FakeDataObject {
        vtable: *const IDataObjectVtbl,
        supported: bool,
        ready: Cell<bool>,
        queries: Cell<u32>,
        gets: Cell<u32>,
        owner: *mut MediumOwner,
    }
    impl FakeDataObject {
        fn new(owner: &mut MediumOwner, supported: bool) -> Self {
            Self {
                vtable: &FAKE_VTABLE, supported, ready: Cell::new(false),
                queries: Cell::new(0), gets: Cell::new(0), owner,
            }
        }
        fn object(&mut self) -> *const IDataObject {
            (self as *mut Self).cast::<IDataObject>()
        }
    }

    unsafe extern "system" fn object_query(
        _: *mut IUnknown, _: *const GUID, _: *mut *mut c_void,
    ) -> HRESULT { E_NOINTERFACE }
    unsafe extern "system" fn object_addref(_: *mut IUnknown) -> u32 { 1 }
    unsafe extern "system" fn object_release(_: *mut IUnknown) -> u32 { 1 }
    unsafe extern "system" fn object_get(
        this: *mut IDataObject, _: *const FORMATETC, medium: *mut STGMEDIUM,
    ) -> HRESULT {
        let object = unsafe { &*this.cast::<FakeDataObject>() };
        object.gets.set(object.gets.get() + 1);
        if !object.ready.get() || medium.is_null() { return E_FAIL; }
        let owner = unsafe { &*object.owner };
        unsafe {
            (*medium).tymed = TYMED_HGLOBAL as u32;
            (*medium).u.hGlobal = owner.handle.get();
            (*medium).pUnkForRelease = object.owner.cast::<c_void>();
        }
        S_OK
    }
    unsafe extern "system" fn object_get_here(
        _: *mut IDataObject, _: *const FORMATETC, _: *mut STGMEDIUM,
    ) -> HRESULT { E_NOTIMPL }
    unsafe extern "system" fn object_query_get(
        this: *mut IDataObject, format: *const FORMATETC,
    ) -> HRESULT {
        let object = unsafe { &*this.cast::<FakeDataObject>() };
        object.queries.set(object.queries.get() + 1);
        if object.supported && !format.is_null() && unsafe {
            (*format).cfFormat == CF_HDROP && (*format).tymed & TYMED_HGLOBAL as u32 != 0
        } { S_OK } else { DV_E_FORMATETC }
    }
    unsafe extern "system" fn object_canonical(
        _: *mut IDataObject, _: *const FORMATETC, _: *mut FORMATETC,
    ) -> HRESULT { E_NOTIMPL }
    unsafe extern "system" fn object_set(
        _: *mut IDataObject, _: *const FORMATETC, _: *const FORMATETC, _: BOOL,
    ) -> HRESULT { E_NOTIMPL }
    unsafe extern "system" fn object_enum_formats(
        _: *mut IDataObject, _: u32, _: *mut *mut IEnumFORMATETC,
    ) -> HRESULT { E_NOTIMPL }
    unsafe extern "system" fn object_advise(
        _: *mut IDataObject, _: *const FORMATETC, _: u32,
        _: *const IAdviseSink, _: *mut u32,
    ) -> HRESULT { E_NOTIMPL }
    unsafe extern "system" fn object_unadvise(_: *mut IDataObject, _: u32) -> HRESULT { E_NOTIMPL }
    unsafe extern "system" fn object_enum_advise(
        _: *mut IDataObject, _: *const *const IEnumSTATDATA,
    ) -> HRESULT { E_NOTIMPL }
    static FAKE_VTABLE: IDataObjectVtbl = IDataObjectVtbl {
        parent: IUnknownVtbl {
            QueryInterface: object_query, AddRef: object_addref, Release: object_release,
        },
        GetData: object_get, GetDataHere: object_get_here, QueryGetData: object_query_get,
        GetCanonicalFormatEtc: object_canonical, SetData: object_set,
        EnumFormatEtc: object_enum_formats, DAdvise: object_advise,
        DUnadvise: object_unadvise, EnumDAdvise: object_enum_advise,
    };

    #[derive(Debug, PartialEq)]
    enum Seen {
        Hovered(PathBuf),
        Dropped(PathBuf),
        HoverCancelled,
    }
    fn fixture() -> (FileDropHandler, Rc<RefCell<Vec<Seen>>>) {
        let seen = Rc::new(RefCell::new(Vec::new()));
        let captured = seen.clone();
        let handler = FileDropHandler::new(0, Box::new(move |event| {
            if let Event::WindowEvent { event, .. } = event {
                let seen = match event {
                    crate::event::WindowEvent::HoveredFile(path) => Some(Seen::Hovered(path)),
                    crate::event::WindowEvent::DroppedFile(path) => Some(Seen::Dropped(path)),
                    crate::event::WindowEvent::HoveredFileCancelled => Some(Seen::HoverCancelled),
                    _ => None,
                };
                if let Some(seen) = seen { captured.borrow_mut().push(seen); }
            }
        }));
        (handler, seen)
    }
    fn enter(handler: &FileDropHandler, source: &mut FakeDataObject, allowed: u32) -> u32 {
        let mut effect = allowed;
        assert_eq!(unsafe { FileDropHandler::DragEnter(
            handler.data.cast(), source.object(), 0, POINTL { x: 17, y: -23 }, &mut effect,
        ) }, S_OK);
        effect
    }
    fn drop_now(handler: &FileDropHandler, source: &mut FakeDataObject, allowed: u32) -> u32 {
        let mut effect = allowed;
        assert_eq!(unsafe { FileDropHandler::Drop(
            handler.data.cast(), source.object(), 0, POINTL { x: 17, y: -23 }, &mut effect,
        ) }, S_OK);
        effect
    }

    // Call through the actual IDropTarget vtable, not a copied QI implementation.
    unsafe fn query_through_vtable(this: *mut c_void, iid: *const GUID, out: *mut *mut c_void) -> HRESULT {
        let table = unsafe { (*this.cast::<IDropTarget>()).lpVtbl };
        unsafe { ((*table).parent.QueryInterface)(this.cast::<IUnknown>(), iid, out) }
    }
    unsafe fn release_through_vtable(this: *mut c_void) -> u32 {
        let table = unsafe { (*this.cast::<IDropTarget>()).lpVtbl };
        unsafe { ((*table).parent.Release)(this.cast::<IUnknown>()) }
    }

    #[test]
    fn com_query_interface_returns_one_identity_and_balanced_owned_references() {
        let (handler, _) = fixture();
        let this = handler.data.cast::<c_void>();
        assert_eq!(unsafe { (*handler.data).refcount.load(Ordering::Acquire) }, 1);
        let mut unknown = ptr::null_mut();
        assert_eq!(unsafe { query_through_vtable(this, &IID_IUNKNOWN, &mut unknown) }, S_OK);
        assert_eq!(unknown, this);
        assert_eq!(unsafe { (*handler.data).refcount.load(Ordering::Acquire) }, 2);
        let mut target = ptr::null_mut();
        assert_eq!(unsafe { query_through_vtable(unknown, &IID_IDROP_TARGET, &mut target) }, S_OK);
        assert_eq!(target, this);
        assert_eq!(unsafe { (*handler.data).refcount.load(Ordering::Acquire) }, 3);
        let mut again = ptr::null_mut();
        assert_eq!(unsafe { query_through_vtable(target, &IID_IUNKNOWN, &mut again) }, S_OK);
        assert_eq!(again, unknown);
        assert_eq!(unsafe { (*handler.data).refcount.load(Ordering::Acquire) }, 4);
        assert_eq!(unsafe { release_through_vtable(again) }, 3);
        assert_eq!(unsafe { release_through_vtable(target) }, 2);
        assert_eq!(unsafe { release_through_vtable(unknown) }, 1);
    }

    #[test]
    fn com_query_interface_refuses_unknown_and_null_without_incrementing() {
        let (handler, _) = fixture();
        let this = handler.data.cast::<c_void>();
        let unrelated = GUID::from_u128(0x12345678_9abc_def0_1234_56789abcdef0);
        let mut out = 1usize as *mut c_void;
        assert_eq!(unsafe { query_through_vtable(this, &unrelated, &mut out) }, E_NOINTERFACE);
        assert!(out.is_null());
        out = 1usize as *mut c_void;
        assert_eq!(unsafe { query_through_vtable(this, ptr::null(), &mut out) }, E_POINTER);
        assert!(out.is_null());
        assert_eq!(unsafe { query_through_vtable(this, &IID_IUNKNOWN, ptr::null_mut()) }, E_POINTER);
        out = 1usize as *mut c_void;
        assert_eq!(unsafe { FileDropHandler::QueryInterface(ptr::null_mut(), &IID_IUNKNOWN, &mut out) }, E_POINTER);
        assert!(out.is_null());
        assert_eq!(unsafe { (*handler.data).refcount.load(Ordering::Acquire) }, 1);
    }

    #[test]
    fn com_query_interface_reference_outlives_the_original_owner() {
        struct Lifetime(Rc<Cell<usize>>);
        impl Drop for Lifetime {
            fn drop(&mut self) { self.0.set(self.0.get() + 1); }
        }
        let destroyed = Rc::new(Cell::new(0));
        let lifetime = Lifetime(destroyed.clone());
        let handler = FileDropHandler::new(0, Box::new(move |_| { std::hint::black_box(&lifetime); }));
        let this = handler.data.cast::<c_void>();
        let mut owned = ptr::null_mut();
        assert_eq!(unsafe { query_through_vtable(this, &IID_IDROP_TARGET, &mut owned) }, S_OK);
        drop(handler);
        assert_eq!(destroyed.get(), 0, "QueryInterface must retain the object");
        assert_eq!(unsafe { release_through_vtable(owned) }, 0);
        assert_eq!(destroyed.get(), 1, "the last Release must destroy the object exactly once");
    }

    #[test]
    fn native_vtable_accepts_pointl_by_value_on_each_drop_callback() {
        // The SDK IDropTarget signatures pass POINTL by value. These exact
        // types also make a pointer-based declaration fail at compile time.
        type EnterOrDrop = unsafe extern "system" fn(
            *mut IDropTarget, *const IDataObject, u32, POINTL, *mut u32,
        ) -> HRESULT;
        type Over = unsafe extern "system" fn(*mut IDropTarget, u32, POINTL, *mut u32) -> HRESULT;
        assert_eq!(std::mem::size_of::<POINTL>(), 8);
        let (handler, paths) = fixture();
        let mut owner = MediumOwner::with_path(Some(r"C:\synthetic\voice.wav"));
        let mut source = FakeDataObject::new(&mut owner, true);
        source.ready.set(true);
        let interface = handler.data.cast::<IDropTarget>();
        let table = unsafe { &*(*interface).lpVtbl };
        let enter: EnterOrDrop = table.DragEnter;
        let over: Over = table.DragOver;
        let drop_file: EnterOrDrop = table.Drop;
        let point = POINTL { x: -123, y: 456 };
        let mut effect = DROPEFFECT_COPY | DROPEFFECT_MOVE;
        assert_eq!(unsafe { enter(interface, source.object(), 0, point, &mut effect) }, S_OK);
        assert_eq!(effect, DROPEFFECT_COPY);
        assert_eq!(owner.releases.get(), 1);
        effect = DROPEFFECT_COPY | DROPEFFECT_MOVE;
        assert_eq!(unsafe { over(interface, 0, point, &mut effect) }, S_OK);
        assert_eq!(effect, DROPEFFECT_COPY);
        owner.refill(r"C:\synthetic\voice.wav");
        effect = DROPEFFECT_COPY;
        assert_eq!(unsafe { drop_file(interface, source.object(), 0, point, &mut effect) }, S_OK);
        assert_eq!(effect, DROPEFFECT_COPY);
        assert_eq!(owner.releases.get(), 2);
        assert_eq!(paths.borrow().as_slice(), &[
            Seen::Hovered(PathBuf::from(r"C:\synthetic\voice.wav")),
            Seen::Dropped(PathBuf::from(r"C:\synthetic\voice.wav")),
        ]);
    }

    #[test]
    fn deferred_file_drop_is_admitted_without_early_render_and_releases_medium_once() {
        let (handler, paths) = fixture();
        let mut owner = MediumOwner::with_path(Some(r"C:\synthetic\voice.wav"));
        let mut source = FakeDataObject::new(&mut owner, true);
        assert_eq!(enter(&handler, &mut source, DROPEFFECT_COPY | DROPEFFECT_MOVE), DROPEFFECT_COPY);
        assert_eq!(source.queries.get(), 1);
        assert_eq!(source.gets.get(), 1);
        source.ready.set(true);
        assert_eq!(drop_now(&handler, &mut source, DROPEFFECT_COPY), DROPEFFECT_COPY);
        assert_eq!(source.gets.get(), 2);
        assert_eq!(owner.releases.get(), 1);
        assert!(owner.handle.get().is_null());
        assert_eq!(paths.borrow().as_slice(), &[Seen::Dropped(PathBuf::from(r"C:\synthetic\voice.wav"))]);
        assert_eq!(drop_now(&handler, &mut source, DROPEFFECT_COPY), DROPEFFECT_NONE);
        assert_eq!(source.gets.get(), 2);
        assert_eq!(owner.releases.get(), 1);
    }

    #[test]
    fn deferred_file_drop_immediate_source_keeps_hover_and_releases_both_media() {
        let (handler, paths) = fixture();
        let mut owner = MediumOwner::with_path(Some(r"C:\synthetic\voice.wav"));
        let mut source = FakeDataObject::new(&mut owner, true);
        source.ready.set(true);
        assert_eq!(enter(&handler, &mut source, DROPEFFECT_COPY), DROPEFFECT_COPY);
        assert_eq!(source.gets.get(), 1);
        assert_eq!(source.queries.get(), 0, "successful old sources need no new query");
        assert_eq!(owner.releases.get(), 1);
        assert_eq!(paths.borrow().as_slice(), &[
            Seen::Hovered(PathBuf::from(r"C:\synthetic\voice.wav")),
        ]);
        owner.refill(r"C:\synthetic\voice.wav");
        assert_eq!(drop_now(&handler, &mut source, DROPEFFECT_COPY), DROPEFFECT_COPY);
        assert_eq!(source.gets.get(), 2);
        assert_eq!(owner.releases.get(), 2);
        assert!(owner.handle.get().is_null());
        assert_eq!(paths.borrow().as_slice(), &[
            Seen::Hovered(PathBuf::from(r"C:\synthetic\voice.wav")),
            Seen::Dropped(PathBuf::from(r"C:\synthetic\voice.wav")),
        ]);
    }

    #[test]
    fn deferred_file_drop_eager_source_does_not_require_query_get_data() {
        let (handler, paths) = fixture();
        let mut owner = MediumOwner::with_path(Some(r"C:\synthetic\voice.wav"));
        let mut source = FakeDataObject::new(&mut owner, false);
        source.ready.set(true);
        assert_eq!(enter(&handler, &mut source, DROPEFFECT_COPY), DROPEFFECT_COPY);
        assert_eq!(source.queries.get(), 0);
        assert_eq!(source.gets.get(), 1);
        assert_eq!(owner.releases.get(), 1);
        assert_eq!(paths.borrow().as_slice(), &[
            Seen::Hovered(PathBuf::from(r"C:\synthetic\voice.wav")),
        ]);
    }

    #[test]
    fn deferred_file_drop_failed_after_hover_cancels_the_hover() {
        let (handler, paths) = fixture();
        let mut owner = MediumOwner::with_path(Some(r"C:\synthetic\voice.wav"));
        let mut source = FakeDataObject::new(&mut owner, true);
        source.ready.set(true);
        assert_eq!(enter(&handler, &mut source, DROPEFFECT_COPY), DROPEFFECT_COPY);
        source.ready.set(false);
        assert_eq!(drop_now(&handler, &mut source, DROPEFFECT_COPY), DROPEFFECT_NONE);
        assert_eq!(source.gets.get(), 2);
        assert_eq!(owner.releases.get(), 1);
        assert_eq!(paths.borrow().as_slice(), &[
            Seen::Hovered(PathBuf::from(r"C:\synthetic\voice.wav")),
            Seen::HoverCancelled,
        ]);
    }

    #[test]
    fn deferred_file_drop_preserves_multiple_paths_and_releases_one_medium() {
        let (handler, paths) = fixture();
        let mut owner = MediumOwner::with_paths(&[
            r"C:\synthetic\first.wav", r"C:\synthetic\second.wav",
        ]);
        let mut source = FakeDataObject::new(&mut owner, true);
        assert_eq!(enter(&handler, &mut source, DROPEFFECT_COPY), DROPEFFECT_COPY);
        assert_eq!(source.gets.get(), 1);
        source.ready.set(true);
        assert_eq!(drop_now(&handler, &mut source, DROPEFFECT_COPY), DROPEFFECT_COPY);
        assert_eq!(paths.borrow().as_slice(), &[
            Seen::Dropped(PathBuf::from(r"C:\synthetic\first.wav")),
            Seen::Dropped(PathBuf::from(r"C:\synthetic\second.wav")),
        ]);
        assert_eq!(source.gets.get(), 2);
        assert_eq!(owner.releases.get(), 1);
    }

    #[test]
    fn deferred_file_drop_rejects_unsupported_format_or_move_only() {
        let (handler, paths) = fixture();
        let mut owner = MediumOwner::with_path(Some(r"C:\synthetic\voice.wav"));
        let mut unsupported = FakeDataObject::new(&mut owner, false);
        assert_eq!(enter(&handler, &mut unsupported, DROPEFFECT_COPY), DROPEFFECT_NONE);
        assert_eq!(drop_now(&handler, &mut unsupported, DROPEFFECT_COPY), DROPEFFECT_NONE);
        assert_eq!(unsupported.gets.get(), 1);
        let mut move_only = FakeDataObject::new(&mut owner, true);
        assert_eq!(enter(&handler, &mut move_only, DROPEFFECT_MOVE), DROPEFFECT_NONE);
        assert_eq!(move_only.queries.get(), 0);
        assert_eq!(drop_now(&handler, &mut move_only, DROPEFFECT_COPY), DROPEFFECT_NONE);
        assert_eq!(move_only.gets.get(), 0);
        assert!(paths.borrow().is_empty());
        assert_eq!(owner.releases.get(), 0);
    }

    #[test]
    fn deferred_file_drop_leave_retires_admission() {
        let (handler, paths) = fixture();
        let mut owner = MediumOwner::with_path(Some(r"C:\synthetic\voice.wav"));
        let mut source = FakeDataObject::new(&mut owner, true);
        source.ready.set(true);
        assert_eq!(enter(&handler, &mut source, DROPEFFECT_COPY), DROPEFFECT_COPY);
        assert_eq!(unsafe { FileDropHandler::DragLeave(handler.data.cast()) }, S_OK);
        assert_eq!(drop_now(&handler, &mut source, DROPEFFECT_COPY), DROPEFFECT_NONE);
        assert_eq!(source.gets.get(), 1);
        assert_eq!(owner.releases.get(), 1);
        assert_eq!(paths.borrow().as_slice(), &[
            Seen::Hovered(PathBuf::from(r"C:\synthetic\voice.wav")), Seen::HoverCancelled,
        ]);
    }

    #[test]
    fn deferred_file_drop_failed_or_empty_medium_is_refused() {
        let (handler, paths) = fixture();
        let mut failed_owner = MediumOwner::with_path(Some(r"C:\synthetic\voice.wav"));
        let mut failed = FakeDataObject::new(&mut failed_owner, true);
        assert_eq!(enter(&handler, &mut failed, DROPEFFECT_COPY), DROPEFFECT_COPY);
        assert_eq!(drop_now(&handler, &mut failed, DROPEFFECT_COPY), DROPEFFECT_NONE);
        assert_eq!(failed.gets.get(), 2);
        assert_eq!(failed_owner.releases.get(), 0);
        let mut empty_owner = MediumOwner::with_path(None);
        let mut empty = FakeDataObject::new(&mut empty_owner, true);
        assert_eq!(enter(&handler, &mut empty, DROPEFFECT_COPY), DROPEFFECT_COPY);
        empty.ready.set(true);
        assert_eq!(drop_now(&handler, &mut empty, DROPEFFECT_COPY), DROPEFFECT_NONE);
        assert_eq!(empty_owner.releases.get(), 1);
        assert!(empty_owner.handle.get().is_null());
        assert!(paths.borrow().is_empty());
    }
}
