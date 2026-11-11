#[derive(Clone, Copy, Debug)]
pub enum VectorEnvironment { NearestEvenMaskedGradual }
#[derive(Debug)]
pub enum VectorError {
    UnsupportedPlatform, FloatEnvironment(u32), Width, Extent,
    NonfiniteRead, NonfiniteResult, MissingPrimitive(u64), NullPrimitive,
    DuplicatePrimitive(u64), WrapperClass,
}

pub(crate) fn check_environment(_environment: VectorEnvironment) -> Result<(), VectorError> {
    #[cfg(all(target_arch="x86_64", target_os="windows"))]
    {
        let mut word = 0u32;
        unsafe {
            core::arch::asm!("stmxcsr [{address}]", address = in(reg) &mut word,
                options(nostack, preserves_flags));
        }
        if word & 0xffc0 != 0x1f80 { return Err(VectorError::FloatEnvironment(word)); }
        Ok(())
    }
    #[cfg(not(all(target_arch="x86_64", target_os="windows")))]
    { Err(VectorError::UnsupportedPlatform) }
}

pub(crate) fn check_current_vector_environment()->Result<(),VectorError>{
    check_environment(VectorEnvironment::NearestEvenMaskedGradual)
}
