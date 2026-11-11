use super::{arithmetic::cache_blocking, GeometryContext};
use std::sync::OnceLock;
#[path = "desktop_layout_cache_host_cpu.rs"]
mod host_cpu;

#[derive(Clone, Debug)]
pub enum Error {
    UnsupportedArchitecture,
    FloatingPointEnvironment { observed: u32 },
    SeparateOperationsNotDeclared,
    Query(cache_blocking::Error),
    Context(super::arithmetic::context::Error),
}
static PROCESS_CONTEXT: OnceLock<Result<GeometryContext, Error>> = OnceLock::new();

pub fn check_current_thread() -> Result<u32, Error> {
    #[cfg(not(any(target_arch = "x86_64", target_arch = "x86")))]
    return Err(Error::UnsupportedArchitecture);
    #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
    {
        let mut observed = 0_u32;
        unsafe {
            std::arch::asm!("stmxcsr [{p}]", p = in(reg) &mut observed,
                options(nostack, preserves_flags));
        }
        if observed & 0xffc0 != 0x1f80 { return Err(Error::FloatingPointEnvironment { observed }); }
        Ok(observed)
    }
}

pub fn process_context(separate_operations: bool) -> Result<GeometryContext, Error> {
    if !separate_operations { return Err(Error::SeparateOperationsNotDeclared); }
    let controls = check_current_thread()?;
    PROCESS_CONTEXT.get_or_init(|| {
        #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
        {
            let snapshot = host_cpu::collect_host().map_err(Error::Query)?;
            GeometryContext::from_owned_records(snapshot.records, controls, separate_operations).map_err(Error::Context)
        }
        #[cfg(not(any(target_arch = "x86_64", target_arch = "x86")))]
        { Err(Error::UnsupportedArchitecture) }
    }).clone()
}
