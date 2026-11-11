use super::finite_power::EntryComparison;
use super::scalar_f64_operand::F64Operand;
use super::shared_scalar_f32::comparison::ordered_sign_magnitude_relation;

pub(super) fn f64_comparison_candidate(left: u64, right: u64, quiet: bool, daz: bool) -> (EntryComparison, u8) {
    let a = F64Operand::from_bits(left);
    let b = F64Operand::from_bits(right);

    if a.nan() || b.nan() {
        return (EntryComparison::Unordered, u8::from(!quiet || a.signaling() || b.signaling()));
    }
    let a = a.with_daz(daz);
    let b = b.with_daz(daz);
    let raised = if a.subnormal() || b.subnormal() { 2 } else { 0 };
    (ordered_sign_magnitude_relation(a.magnitude(), a.negative(), b.magnitude(), b.negative()).into(), raised)
}
