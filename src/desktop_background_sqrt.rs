pub(super) fn evaluate(operand: u64, saved_mxcsr: u32) -> Result<(u64, u32), ()> {
    if saved_mxcsr & 0xffc0 != 0x1f80 { return Err(()); }
    let mut value = f64::from_bits(operand);
    if !value.is_finite() || value < 0.0 { return Err(()); }
    #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
    {
        let clean = saved_mxcsr & !0x3f;
        let mut completed = 0u32;
        unsafe {
            std::arch::asm!(
                "ldmxcsr [{clean}]",
                "sqrtsd {value}, {value}",
                "stmxcsr [{completed}]",
                "ldmxcsr [{saved}]",
                clean = in(reg) &clean, completed = in(reg) &mut completed,
                saved = in(reg) &saved_mxcsr, value = inout(xmm_reg) value,
                options(nostack, preserves_flags),
            );
        }
        Ok((value.to_bits(), completed & 0x3f))
    }
    #[cfg(not(any(target_arch = "x86_64", target_arch = "x86")))]
    { Err(()) }
}
