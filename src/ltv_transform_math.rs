use crate::{ltv_first_call::SourceTransformMathProfile,
    ltv_transform_log::{SourcePositiveIntegerTransformLog,TransformLogError},
    ltv_transform_rounding::SourceTransformMathWithBoundCeil};

pub type SourceTransformValueMath = SourceTransformMathWithBoundCeil<SourcePositiveIntegerTransformLog>;

pub struct BindingFailure {
    pub profile: SourceTransformMathProfile,
    pub cause: TransformLogError,
}

pub fn from_explicit_profile(profile: SourceTransformMathProfile)
    -> Result<SourceTransformValueMath, BindingFailure>
{
    let logarithm = SourcePositiveIntegerTransformLog::from_explicit_profile(profile)
        .map_err(|cause| BindingFailure { profile, cause })?;
    Ok(SourceTransformMathWithBoundCeil::from_explicit_logarithm(logarithm))
}
