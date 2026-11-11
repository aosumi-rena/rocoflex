use super::{layout, source_inverse, source_math};

pub(super) enum Phase { Admission, Local, DisplayMap, Numeric, Publication }
pub(super) enum Cause {
    Controls(layout::context::provider::Error),
    Nonfinite,
    Extent,
    Numeric,
}
enum Numeric { Complete(source_inverse::Completed), Failed(source_inverse::Failure) }

pub(super) struct Prepared {
    pub event: [f32; 2],
    pub origin: [f32; 2],
    pub extent: [f32; 2],
    pub current: [f32; 2],
    pub admission_mxcsr: Option<u32>,
    pub local: Option<[f32; 2]>,
    pub display_scale: Option<f32>,
    pub display_offset: Option<[f32; 2]>,
    pub canonical: Option<[f32; 2]>,
    pub rounded: Option<[i32; 2]>,
    pub clamped: Option<[i32; 2]>,
    pub normalized: Option<[f64; 2]>,
    pub inverse: Option<[f64; 2]>,
    pub position: Option<[f32; 2]>,
    pub phase: Phase,
    numeric: Option<Numeric>,
}

#[must_use = "retain the exact event, inverse prefix and control/refusal cause"]
pub(super) struct Failure { pub pending: Prepared, pub cause: Cause }
pub(super) struct Completed { pending: Prepared, position: [f32; 2] }
impl Completed { pub fn position(&self) -> [f32; 2] { self.position } }
impl Failure {
    pub fn message(&self) -> &'static str { "The probe position could not be calculated." }
}

impl Prepared {
    fn calculate(&mut self) -> Result<[f32; 2], Cause> {
        let mxcsr = layout::context::provider::check_current_thread().map_err(Cause::Controls)?;
        self.admission_mxcsr = Some(mxcsr);
        if self.event.iter().chain(&self.origin).chain(&self.extent).chain(&self.current).any(|v| !v.is_finite()) {
            return Err(Cause::Nonfinite);
        }
        if self.extent.iter().any(|v| *v <= 0.0) { return Err(Cause::Extent); }
        self.phase = Phase::Local;

        let local = [self.event[0] - self.origin[0], self.event[1] - self.origin[1]];
        self.local = Some(local);
        self.phase = Phase::DisplayMap;

        let scale = 560.0 / self.extent[0].min(self.extent[1]);
        self.display_scale = Some(scale);
        let offset = [430.0 - (self.extent[0] * scale) * 0.5,
            280.0 - (self.extent[1] * scale) * 0.5];
        self.display_offset = Some(offset);
        let canonical = [local[0] * scale + offset[0], local[1] * scale + offset[1]];
        self.canonical = Some(canonical);
        if !scale.is_finite() || canonical.iter().any(|v| !v.is_finite()) {
            return Err(Cause::Nonfinite);
        }
        self.phase = Phase::Numeric;
        let numeric_mxcsr = layout::context::provider::check_current_thread().map_err(Cause::Controls)?;

        let completed = match source_inverse::calculate(canonical, self.current, numeric_mxcsr, 0,
            crate::source_scalar::ScalarF32Architecture::IntelSdm)
        {
            Ok(completed) => completed,
            Err(failure) => { self.numeric = Some(Numeric::Failed(failure)); return Err(Cause::Numeric); }
        };
        self.rounded = completed.rounded();
        self.clamped = completed.clamped();
        self.normalized = completed.normalized();
        self.inverse = completed.inverse();
        let position = completed.position();
        self.numeric = Some(Numeric::Complete(completed));
        self.phase = Phase::Publication;
        self.position = Some(position);
        layout::context::provider::check_current_thread().map_err(Cause::Controls)?;
        Ok(position)
    }
}

pub(super) fn project(event: [f32; 2], origin: [f32; 2], extent: [f32; 2], current: [f32; 2])
    -> Result<Completed, Failure> {
    let mut pending = Prepared { event, origin, extent, current, admission_mxcsr: None, local: None,
        display_scale: None, display_offset: None, canonical: None, rounded: None,
        clamped: None, normalized: None, inverse: None, position: None, phase: Phase::Admission, numeric: None };
    match pending.calculate() {
        Ok(position) => Ok(Completed { pending, position }),
        Err(cause) => Err(Failure { pending, cause }),
    }
}
