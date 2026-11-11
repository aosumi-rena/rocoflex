use super::cache_blocking::{select_cache,CacheSelection,CpuidRecord,CpuidRegisters,Error};

#[derive(Debug)]
pub struct Snapshot {pub records:Vec<CpuidRecord>,pub selected:CacheSelection}

pub fn collect_with(mut read:impl FnMut(u32,u32)->CpuidRegisters)->Result<Snapshot,Error>{
    let mut records=Vec::new();
    loop {
        match select_cache(&records) {
            Ok(selected)=>return Ok(Snapshot{records,selected}),
            Err(Error::MissingCpuid{leaf,subleaf})=>{
                let registers=read(leaf,subleaf);
                records.push(CpuidRecord{leaf,subleaf,registers});
            }
            Err(error)=>return Err(error),
        }
    }
}

#[cfg(any(target_arch="x86_64",target_arch="x86"))]
pub fn collect_host()->Result<Snapshot,Error>{
    #[cfg(target_arch="x86_64")] use std::arch::x86_64::__cpuid_count;
    #[cfg(target_arch="x86")] use std::arch::x86::__cpuid_count;
    collect_with(|leaf,subleaf|{

        let r=__cpuid_count(leaf,subleaf);
        CpuidRegisters{eax:r.eax,ebx:r.ebx,ecx:r.ecx,edx:r.edx}
    })
}

