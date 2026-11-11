use super::{empty_background::literal::{prefix, geometry, radial}, layout,
    source_math::BackgroundMath};
use crate::source_scalar as literal;
use literal::{ExplicitExceptionState, ScalarF32Architecture, SourceExp597c90,
    SourceExp597c90Progress, SourceExpDispatchError, SourceExpDispatchOutcome,
    SourceExpEntry, SourceExpRangeError, SourceExp596150Arguments};

pub(super) struct RangeBindings {
    pub actual_name_address: u64,
    pub decode_source_pointer: fn(u64, u64) -> u64,
    pub verify_source_cookie: fn(u64, u64) -> bool,
}

pub(super) struct RangeStopped<E> {
    pub cursor: SourceExp597c90,
    pub error: E,

    pub known_advanced: Option<ExplicitExceptionState>,
}

pub(super) trait RangeServices {
    type Error;

    fn complete_597c90(&mut self, cursor: SourceExp597c90)
        -> Result<SourceExp597c90, RangeStopped<Self::Error>>;
}

pub(super) enum Failure<E> {
    Controls(layout::context::provider::Error),
    UpperLane(prefix::ExpRequest),
    Dispatch { error: SourceExpDispatchError, advanced: ExplicitExceptionState },
    RangeEntry { error: SourceExpRangeError, entry: SourceExpEntry },
    RangeService { expected: SourceExp596150Arguments, stopped: RangeStopped<E> },
    MismatchedReturn { expected: SourceExp596150Arguments, cursor: SourceExp597c90 },
    IncompleteReturn(SourceExp597c90),
}

pub(super) struct LastCall {
    pub request: prefix::ExpRequest,
    pub value: Option<u32>,
}

#[must_use = "retain the callback context, pending range cursor and exact error"]
pub(super) struct SourceRadial<C: RangeServices> {
    math: BackgroundMath,
    state: ExplicitExceptionState,
    bindings: RangeBindings,
    context: C,
    failure: Option<Failure<C::Error>>,
    last: Option<LastCall>,
    completed_range: Option<SourceExp597c90>,
}

#[must_use = "resolve all original context, cursor and service-error owners explicitly"]
pub(super) struct Retained<C: RangeServices> {
    pub math: BackgroundMath,

    pub known_prefix_state: ExplicitExceptionState,
    pub bindings: RangeBindings,
    pub context: C,
    pub failure: Option<Failure<C::Error>>,
    pub last: Option<LastCall>,
    pub completed_range: Option<SourceExp597c90>,
}

impl<C: RangeServices> SourceRadial<C> {

    pub fn from_explicit_context(math: BackgroundMath, bindings: RangeBindings, context: C) -> Self {
        let state = math.exception_state();
        Self { math, state, bindings, context, failure: None,
            last: None, completed_range: None }
    }

    pub fn context(&self) -> &C { &self.context }
    pub fn math(&self) -> &BackgroundMath { &self.math }

    pub fn failure(&self) -> Option<&Failure<C::Error>> { self.failure.as_ref() }
    pub fn last_call(&self) -> Option<&LastCall> { self.last.as_ref() }

    pub fn known_prefix_state(&self) -> ExplicitExceptionState { self.state }
    pub fn completed_range(&self) -> Option<&SourceExp597c90> { self.completed_range.as_ref() }

    pub fn into_owned_prefix(self) -> Retained<C> {
        Retained { math: self.math, known_prefix_state: self.state,
            bindings: self.bindings, context: self.context, failure: self.failure,
            last: self.last, completed_range: self.completed_range }
    }

    fn check_controls(&mut self) -> Result<(), ()> {
        if self.failure.is_some() || self.math.failure().is_some() { return Err(()); }
        match layout::context::provider::check_current_thread() {
            Ok(_) => Ok(()),
            Err(error) => { self.failure = Some(Failure::Controls(error)); Err(()) }
        }
    }

    fn evaluate(&mut self, request: prefix::ExpRequest) -> Result<f32, ()> {
        if self.failure.is_some() || self.math.failure().is_some() { return Err(()); }
        self.last = Some(LastCall { request, value: None });
        self.check_controls()?;
        self.state = self.math.exception_state();
        if request.upper_f64 != literal::SOURCE_RADIAL_EXP_UPPER_F64 {
            self.failure = Some(Failure::UpperLane(request)); return Err(());
        }
        let outcome = match literal::selected_exp_with_explicit_upper_lane(
            self.math.source_word(), ScalarF32Architecture::IntelSdm,
            request.argument, request.upper_f64, self.state,
        ) {
            Ok(value) => value,
            Err(error) => {
                let advanced = error.known_advanced_state(self.state);
                self.state = advanced;
                self.failure = Some(Failure::Dispatch { error, advanced }); return Err(());
            }
        };
        let bits = match outcome {
            SourceExpDispatchOutcome::Complete(value) => {
                self.state = value.exception_state; value.bits
            }
            SourceExpDispatchOutcome::RangeRequired(entry) => {
                self.state = entry.exception_state;
                let cursor: SourceExp597c90 = match SourceExp597c90::begin(entry, self.bindings.actual_name_address,
                    self.bindings.decode_source_pointer, self.bindings.verify_source_cookie) {
                    Ok(cursor) => cursor,
                    Err(error) => {
                        self.state = error.known_advanced_state(self.state);
                        self.failure = Some(Failure::RangeEntry { error, entry }); return Err(());
                    }
                };
                let expected = cursor.arguments();
                let cursor = match self.context.complete_597c90(cursor) {
                    Ok(cursor) => cursor,
                    Err(stopped) => {
                        if stopped.cursor.arguments() == expected {
                            self.state = stopped.known_advanced.unwrap_or(stopped.cursor.exception_state());
                        }

                        self.failure = Some(Failure::RangeService { expected, stopped }); return Err(());
                    }
                };
                if cursor.arguments() != expected {
                    self.failure = Some(Failure::MismatchedReturn { expected, cursor }); return Err(());
                }
                match cursor.progress() {
                    SourceExp597c90Progress::Complete(completed) => {
                        self.state = completed.advanced;
                        let bits = completed.bits;
                        self.completed_range = Some(cursor);
                        bits
                    }
                    SourceExp597c90Progress::Request(_) => {
                        self.state = cursor.exception_state();
                        self.failure = Some(Failure::IncompleteReturn(cursor)); return Err(());
                    }
                }
            }
        };
        if let Some(last) = &mut self.last { last.value = Some(bits); }

        self.math.install_advanced(self.state)?;
        self.check_controls()?;
        Ok(f32::from_bits(bits))
    }

    fn math_call<T>(&mut self, call: impl FnOnce(&mut BackgroundMath) -> Result<T, ()>)
        -> Result<T, ()> {
        if self.failure.is_some() || self.math.failure().is_some() { return Err(()); }
        let value = call(&mut self.math)?;
        self.state = self.math.exception_state();
        Ok(value)
    }
}

impl<C: RangeServices> prefix::ExpServices for SourceRadial<C> {
    fn radial_exp(&mut self, request: prefix::ExpRequest) -> Result<f32, ()> { self.evaluate(request) }
}

impl<C: RangeServices> radial::SqrtServices for SourceRadial<C> {
    fn sqrt_f64_bits(&mut self, operand: u64) -> Result<u64, ()> {
        self.math_call(|math| radial::SqrtServices::sqrt_f64_bits(math, operand))
    }
}
impl<C: RangeServices> geometry::MathServices for SourceRadial<C> {
    fn atan2(&mut self, numerator: f32, denominator: f32) -> Result<f32, ()> {
        self.math_call(|math| geometry::MathServices::atan2(math, numerator, denominator))
    }
    fn ring_sin_cos(&mut self, theta: f32) -> Result<[f32; 2], ()> {
        self.math_call(|math| geometry::MathServices::ring_sin_cos(math, theta))
    }
    fn projection_sin(&mut self, angle: f32) -> Result<f32, ()> {
        self.math_call(|math| geometry::MathServices::projection_sin(math, angle))
    }
    fn projection_cos(&mut self, angle: f32) -> Result<f32, ()> {
        self.math_call(|math| geometry::MathServices::projection_cos(math, angle))
    }
    fn norm(&mut self, first: f32, second: f32) -> Result<f32, ()> {
        self.math_call(|math| geometry::MathServices::norm(math, first, second))
    }
}
