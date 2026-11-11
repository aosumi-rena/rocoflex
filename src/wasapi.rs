use std::{collections::TryReserveError, fmt, marker::PhantomData, ptr::NonNull,
    rc::Rc, string::FromUtf16Error};
use windows::{core::PWSTR, Win32::{Media::Audio::{IMMDevice, IMMDeviceEnumerator, MMDeviceEnumerator},
    System::Com::{CoCreateInstance, CoInitializeEx, CoTaskMemFree, CoUninitialize,
        CLSCTX_ALL, COINIT_APARTMENTTHREADED}}};

#[derive(Debug)]
pub enum Error {
    Native { operation: &'static str, code: i32, message: String },
    NullOutput { operation: &'static str },
    Text { operation: &'static str, cause: FromUtf16Error },
    Allocation { operation: &'static str, cause: TryReserveError },
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Native { operation, code, message } => write!(f, "{operation}: {message} (0x{:08X})", *code as u32),
            Self::NullOutput { operation } => write!(f, "{operation}: null native output"),
            Self::Text { operation, cause } => write!(f, "{operation}: {cause}"),
            Self::Allocation { operation, cause } => write!(f, "{operation}: {cause}"),
        }
    }
}
impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self { Self::Text { cause, .. } => Some(cause), Self::Allocation { cause, .. } => Some(cause), _ => None }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EndpointChoice { DefaultMultimedia, Id(String) }

pub(crate) fn native(operation: &'static str, error: windows::core::Error) -> Error {
    Error::Native { operation, code: error.code().0, message: error.message() }
}

pub(crate) struct Apartment(PhantomData<Rc<()>>);
impl Apartment {
    pub(crate) fn sta() -> Result<Self, Error> {
        unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) }.ok()
            .map_err(|error| native("CoInitializeEx", error))?;
        Ok(Self(PhantomData))
    }
}
impl Drop for Apartment { fn drop(&mut self) { unsafe { CoUninitialize() }; } }

pub(crate) fn enumerator() -> Result<IMMDeviceEnumerator, Error> {
    unsafe { CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL) }
        .map_err(|error| native("CoCreateInstance(MMDeviceEnumerator)", error))
}

pub(crate) struct TaskMemory<T>(NonNull<T>);
impl<T> TaskMemory<T> {
    pub(crate) unsafe fn from_raw(pointer: *mut T, operation: &'static str) -> Result<Self, Error> {
        NonNull::new(pointer).map(Self).ok_or(Error::NullOutput { operation })
    }
    pub(crate) fn as_ptr(&self) -> *mut T { self.0.as_ptr() }
}
impl<T> Drop for TaskMemory<T> {
    fn drop(&mut self) { unsafe { CoTaskMemFree(Some(self.0.as_ptr().cast())) }; }
}

pub(crate) fn take_text(text: PWSTR, operation: &'static str) -> Result<String, Error> {

    let owned = unsafe { TaskMemory::from_raw(text.0, operation) }?;
    unsafe { PWSTR(owned.as_ptr()).to_string() }.map_err(|cause| Error::Text { operation, cause })
}

pub(crate) fn endpoint_id(device: &IMMDevice) -> Result<String, Error> {
    let value = unsafe { device.GetId() }.map_err(|error| native("IMMDevice::GetId", error))?;
    take_text(value, "IMMDevice::GetId")
}
