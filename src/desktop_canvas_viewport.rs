use eframe::egui::{Pos2, Rect};
use super::layout;
use crate::source_scalar::{ExplicitExceptionState, PowerError, ScalarF32Architecture};

#[path = "desktop_probe_parameter_publication.rs"]
mod publication;

struct Pending {
    event: Pos2,
    rect: Rect,
    zoom: f64,
    world: Option<[f64; 2]>,
    parameters: [Option<publication::Completed>; 2],
}

enum Cause {
    InvalidViewport,
    Controls(layout::context::provider::Error),
    State(PowerError),
    Parameter { axis: usize, failure: publication::Failed },
    Nonfinite,
}

pub(super) struct Failure { pending: Pending, cause: Cause }
pub(super) struct Completed { pending: Pending, position: [f32; 2] }

impl Failure {
    pub fn message(&self) -> &'static str { "The probe position could not be calculated." }
}
impl Completed {
    pub fn position(&self) -> [f32; 2] { self.position }
}

impl Pending {
    fn calculate(&mut self) -> Result<[f32; 2], Cause> {
        if [self.event.x, self.event.y, self.rect.min.x, self.rect.min.y,
                self.rect.max.x, self.rect.max.y].iter().any(|v| !v.is_finite())
            || !self.zoom.is_finite() || self.zoom <= 0.0
        {
            return Err(Cause::InvalidViewport);
        }
        let width = self.rect.width();
        let height = self.rect.height();
        let zoom = self.zoom as f32;
        if !width.is_finite() || !height.is_finite() || width <= 0.0 || height <= 0.0
            || !zoom.is_finite() || zoom <= 0.0
        {
            return Err(Cause::InvalidViewport);
        }
        let half = width.min(height) * 0.5;
        let center = self.rect.center();
        if !half.is_finite() || half <= 0.0
            || !center.x.is_finite() || !center.y.is_finite()
        {
            return Err(Cause::InvalidViewport);
        }

        let x = self.event.x.clamp(self.rect.left(), self.rect.right());
        let y = self.event.y.clamp(self.rect.top(), self.rect.bottom());
        let world = [
            ((f64::from(x) - f64::from(center.x)) / f64::from(half)) / f64::from(zoom),
            ((f64::from(center.y) - f64::from(y)) / f64::from(half)) / f64::from(zoom),
        ];
        self.world = Some(world);
        if world.iter().any(|v| !v.is_finite()) { return Err(Cause::Nonfinite); }
        let mxcsr = layout::context::provider::check_current_thread().map_err(Cause::Controls)?;
        let mut state = ExplicitExceptionState::from_explicit_parts(mxcsr, 0).map_err(Cause::State)?;
        let mut position = [0.0; 2];
        for axis in 0..2 {
            let completed = publication::calculate(world[axis].to_bits(), state,
                ScalarF32Architecture::IntelSdm)
                .map_err(|failure| Cause::Parameter { axis, failure })?;
            state = completed.state();
            position[axis] = f32::from_bits(completed.bits());
            self.parameters[axis] = Some(completed);
            if !position[axis].is_finite() { return Err(Cause::Nonfinite); }
        }
        Ok(position)
    }
}

pub(super) fn project(event: Pos2, rect: Rect, zoom: f64) -> Result<Completed, Failure> {
    let mut pending = Pending { event, rect, zoom, world: None, parameters: [None, None] };
    match pending.calculate() {
        Ok(position) => Ok(Completed { pending, position }),
        Err(cause) => Err(Failure { pending, cause }),
    }
}
