use super::{empty_background, layout, source_math};
use empty_background::literal::public_numeric as camera;
use crate::source_scalar::{ExplicitExceptionState, PowerError, ScalarF32Architecture,
    SourceF64ValueError, SourceF64ValueMath};

#[derive(Clone, Copy, PartialEq, Eq)]
struct Key { pair: [u32; 2], controls: Option<u32> }
struct Input { pair: [u32; 2], mxcsr: u32, errno: i32, architecture: ScalarF32Architecture }
enum BindingError { State(PowerError), Backend(SourceF64ValueError) }
enum Cause { Camera(camera::Failure<SourceF64ValueError>), Narrow(SourceF64ValueError), InvalidExtent }
struct Pending { input: Input, math: SourceF64ValueMath, wide: Option<u64>, narrowed: Option<u32> }
enum State {
    Controls { pair: [u32; 2], error: layout::context::provider::Error },
    Binding { input: Input, error: BindingError },
    Failed { pending: Pending, cause: Cause },
    Complete { pending: Pending, bits: u32 },
}
pub(super) struct Cached { key: Key, state: State }
impl Cached {

    pub fn update(slot: &mut Option<Self>, pair: [f32; 2]) -> Option<f64> {
        let pair = pair.map(f32::to_bits);
        let observed = layout::context::provider::check_current_thread();
        let controls = match &observed {
            Ok(word) => Some(*word & !0x3f),
            Err(layout::context::provider::Error::FloatingPointEnvironment { observed }) => Some(*observed & !0x3f),
            Err(_) => None,
        };
        let key = Key { pair, controls };
        if slot.as_ref().is_none_or(|cached| cached.key != key) {
            let state = match observed {
                Err(error) => State::Controls { pair, error },
                Ok(mxcsr) => Self::calculate(Input { pair, mxcsr, errno: 0, architecture: ScalarF32Architecture::IntelSdm }),
            };
            *slot = Some(Self { key, state });
        }
        match &slot.as_ref()?.state {
            State::Complete { bits, .. } => Some(f64::from(f32::from_bits(*bits))),
            _ => None,
        }
    }
    fn calculate(input: Input) -> State {

        let entry = match ExplicitExceptionState::from_explicit_parts(input.mxcsr, input.errno) {
            Ok(entry) => entry,
            Err(error) => return State::Binding { input, error: BindingError::State(error) },
        };
        let math = match SourceF64ValueMath::bind(input.architecture, entry) {
            Ok(math) => math,
            Err(error) => return State::Binding { input, error: BindingError::Backend(error) },
        };
        let mut pending = Pending { input, math, wide: None, narrowed: None };
        let wide = match camera::source_zoom_f64_with_math(pending.input.pair.map(Some), &mut pending.math) {
            Ok(wide) => wide,
            Err(error) => return State::Failed { pending, cause: Cause::Camera(error) },
        };
        pending.wide = Some(wide);

        let bits = match pending.math.narrow_f64_bits(wide) {
            Ok(bits) => bits,
            Err(error) => return State::Failed { pending, cause: Cause::Narrow(error) },
        };
        pending.narrowed = Some(bits);

        if bits & 0x8000_0000 != 0 || bits & 0x7fff_ffff == 0 || bits & 0x7f80_0000 == 0x7f80_0000 {
            return State::Failed { pending, cause: Cause::InvalidExtent };
        }
        State::Complete { pending, bits }
    }
}
