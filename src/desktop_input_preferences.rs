//! Input preferences have their own cross-process transaction and never carry
//! unrelated application settings or credentials.
use serde_json::{Value, json};
use std::{fs::{self, File, OpenOptions}, io::{Read, Write}, path::{Path, PathBuf}, sync::atomic::{AtomicU64, Ordering}};
const NAME: &str = "input-bindings.json";
const MAX_BYTES: u64 = 65_536;
static NEXT_TEMP: AtomicU64 = AtomicU64::new(1);

pub(super) fn read(dir: &Path) -> Result<Value, String> {
    let path=dir.join(NAME);
    let file=match File::open(&path) {
        Ok(file)=>file,
        Err(error) if error.kind()==std::io::ErrorKind::NotFound=>return Ok(json!({})),
        Err(error)=>return Err(format!("Could not read input bindings: {error}")),
    };
    let size=file.metadata().map_err(|e|format!("Could not inspect input bindings: {e}"))?.len();
    if size>MAX_BYTES {return Err("Input bindings file is too large.".into());}
    let mut bytes=Vec::new();
    file.take(MAX_BYTES+1).read_to_end(&mut bytes).map_err(|e|format!("Could not read input bindings: {e}"))?;
    if bytes.len() as u64>MAX_BYTES {return Err("Input bindings file is too large.".into());}
    let value:Value=serde_json::from_slice(&bytes).map_err(|e|format!("Input bindings file is invalid: {e}"))?;
    if !value.is_object() {return Err("Input bindings file must be an object.".into());}
    Ok(value)
}

pub(super) fn edit(dir: &Path, f: impl FnOnce(&mut Value) -> Result<(), String>) -> Result<Value, String> {
    fs::create_dir_all(dir).map_err(|e|format!("Could not create input bindings directory: {e}"))?;
    let _lock=Lock::acquire(&dir.join("input-bindings.lock"))?;
    let mut value=read(dir)?;
    f(&mut value)?;
    let data=serde_json::to_vec_pretty(&value).map_err(|e|format!("Could not serialize input bindings: {e}"))?;
    if data.len() as u64>MAX_BYTES {return Err("Input bindings are too large to save.".into());}
    let mut created:Option<(PathBuf,File)>=None;
    for _ in 0..16 {
        let serial=NEXT_TEMP.fetch_add(1,Ordering::Relaxed);
        let name=format!("input-bindings.tmp.{}.{}",std::process::id(),serial);
        let path=dir.join(name);
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(file)=>{created=Some((path,file));break;},
            Err(error) if error.kind()==std::io::ErrorKind::AlreadyExists=>continue,
            Err(error)=>return Err(format!("Could not create temporary input bindings: {error}")),
        }
    }
    let (temp,mut file)=created.ok_or("Could not allocate an input bindings temporary file.")?;
    let write=file.write_all(&data).and_then(|_|file.sync_all());
    drop(file);
    if let Err(error)=write {let _=fs::remove_file(&temp);return Err(format!("Could not save input bindings: {error}"));}
    if let Err(error)=replace(&temp,&dir.join(NAME)) {
        let _=fs::remove_file(&temp);return Err(format!("Could not replace input bindings: {error}"));
    }
    Ok(value)
}

#[cfg(windows)]
fn replace(source: &Path, destination: &Path) -> std::io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    let from:Vec<u16>=source.as_os_str().encode_wide().chain(Some(0)).collect();
    let to:Vec<u16>=destination.as_os_str().encode_wide().chain(Some(0)).collect();
    #[link(name="kernel32")]
    unsafe extern "system" { fn MoveFileExW(from:*const u16,to:*const u16,flags:u32)->i32; }
    if unsafe { MoveFileExW(from.as_ptr(),to.as_ptr(),0x1|0x8) }==0 { Err(std::io::Error::last_os_error()) } else { Ok(()) }
}
#[cfg(not(windows))]
fn replace(source: &Path, destination: &Path) -> std::io::Result<()> { fs::rename(source,destination) }

struct Lock { file: File }
impl Lock {
    fn acquire(path: &Path) -> Result<Self,String> {
        let file=OpenOptions::new().read(true).write(true).create(true).open(path)
            .map_err(|e|format!("Could not open input bindings lock: {e}"))?;
        lock(&file).map_err(|e|format!("Input bindings are busy or locked: {e}. Try again."))?;
        Ok(Self{file})
    }
}
impl Drop for Lock { fn drop(&mut self) { unlock(&self.file); } }
#[cfg(windows)]
#[repr(C)]
struct Overlapped { internal:usize, internal_high:usize, offset:u32, offset_high:u32, event:usize }
#[cfg(windows)]
fn overlapped() -> Overlapped { Overlapped {internal:0,internal_high:0,offset:0,offset_high:0,event:0} }
#[cfg(windows)]
#[link(name="kernel32")]
unsafe extern "system" {
    fn LockFileEx(handle:usize,flags:u32,reserved:u32,low:u32,high:u32,overlapped:*mut Overlapped)->i32;
    fn UnlockFileEx(handle:usize,reserved:u32,low:u32,high:u32,overlapped:*mut Overlapped)->i32;
}
#[cfg(windows)]
fn lock(file: &File) -> std::io::Result<()> {
    use std::os::windows::io::AsRawHandle;
    let mut ov=overlapped();
    if unsafe { LockFileEx(file.as_raw_handle() as usize,0x3,0,1,0,&mut ov) }==0 {Err(std::io::Error::last_os_error())} else {Ok(())}
}
#[cfg(windows)]
fn unlock(file: &File) {
    use std::os::windows::io::AsRawHandle;
    let mut ov=overlapped();unsafe {UnlockFileEx(file.as_raw_handle() as usize,0,1,0,&mut ov);}
}
#[cfg(unix)]
unsafe extern "C" { fn flock(fd:i32,operation:i32)->i32; }
#[cfg(unix)]
fn lock(file: &File) -> std::io::Result<()> {
    use std::os::fd::AsRawFd;
    if unsafe {flock(file.as_raw_fd(),2|4)}!=0 {Err(std::io::Error::last_os_error())} else {Ok(())}
}
#[cfg(unix)]
fn unlock(file: &File) { use std::os::fd::AsRawFd;unsafe {flock(file.as_raw_fd(),8);}}
#[cfg(not(any(windows,unix)))]
fn lock(_file:&File)->std::io::Result<()> {Err(std::io::Error::new(std::io::ErrorKind::Unsupported,"No input bindings lock"))}
#[cfg(not(any(windows,unix)))]
fn unlock(_file:&File) {}
