mod logarithm;
mod polar;
mod power;
#[allow(dead_code)]
mod uniform;

pub use polar::Parameters;
pub use uniform::GaussianPairCache as PartnerCache;
use logarithm::ScalarPositiveLog;
use polar::{GaussianMath, UniformSource};
use power::ScalarPowerServices;
use uniform::SourceUniform32;

#[derive(Clone, Copy, Debug)]
pub struct ScalarProfile(u32);

impl ScalarProfile {
    pub fn from_source_word(word: u32) -> Result<Self, FailureReason> {
        if word <= 1 {
            Ok(Self(word))
        } else {
            Err(FailureReason::UnsupportedSourceProfile(word))
        }
    }

    pub fn source_word(self) -> u32 { self.0 }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MathDomain {
    Logarithm,
    Exponent,
    Scaling,
    ExceptionalSquareRoot,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FailureReason {
    UnsupportedSourceProfile(u32),
    UnsupportedPlatform,
    UnsupportedFloatEnvironment { mxcsr: u32 },
    Math(MathDomain),
    Core(String),
    NonFiniteOutput,
}

#[derive(Clone, Debug)]
pub struct State {
    uniform: SourceUniform32,
    cache: PartnerCache,
    consumed: usize,
}

#[derive(Debug)]
pub struct Draw {
    pub value: f32,
    pub state: State,
}

#[derive(Debug)]
pub struct DrawFailure {
    pub reason: FailureReason,
    pub advanced: State,
}

impl State {
    pub fn from_explicit_parts(word: u32, cache: PartnerCache, consumed: usize) -> Self {
        Self { uniform: SourceUniform32::from_state_word(word), cache, consumed }
    }

    pub fn rng_word(&self) -> u32 { self.uniform.state_word() }
    pub fn partner_cache(&self) -> PartnerCache { self.cache }
    pub fn consumed_uniforms(&self) -> usize { self.consumed }

    pub fn draw(
        mut self,
        parameters: Parameters,
        cache_allowed: bool,
        profile: ScalarProfile,
        attempt_limit: usize,
    ) -> Result<Draw, DrawFailure> {
        let mut math = match ScalarMath::bind(profile) {
            Ok(math) => math,
            Err(reason) => return Err(DrawFailure { reason, advanced: self }),
        };
        let mut uniform = UniformBorrow {
            rng: &mut self.uniform,
            consumed: &mut self.consumed,
        };
        let sample = polar::next(
            &mut self.cache, &mut uniform, parameters, cache_allowed,
            &mut math, attempt_limit,
        );
        if let Some(domain) = math.failure {
            return Err(DrawFailure { reason: FailureReason::Math(domain), advanced: self });
        }
        match sample {
            Ok(value) if value.is_finite() => Ok(Draw { value, state: self }),
            Ok(_) => Err(DrawFailure { reason: FailureReason::NonFiniteOutput, advanced: self }),
            Err(error) => Err(DrawFailure { reason: FailureReason::Core(error), advanced: self }),
        }
    }
}

pub fn draw_standard_values(
    profile: ScalarProfile, attempt_limit: usize, uniform_word: &mut u32,
    cache: &mut PartnerCache, consumed: &mut usize,
) -> Result<u32, FailureReason> {
    let state = State::from_explicit_parts(*uniform_word, *cache, *consumed);
    let outcome = state.draw(Parameters { mean: 0.0, scale: 1.0 }, true, profile, attempt_limit);
    let (sample, advanced) = match outcome {
        Ok(draw) => (Ok(draw.value.to_bits()), draw.state),
        Err(failure) => (Err(failure.reason), failure.advanced),
    };
    *uniform_word = advanced.rng_word();
    *cache = advanced.partner_cache();
    *consumed = advanced.consumed_uniforms();
    sample
}

struct UniformBorrow<'a> {
    rng: &'a mut SourceUniform32,
    consumed: &'a mut usize,
}

impl UniformSource for UniformBorrow<'_> {
    fn next_uniform(&mut self) -> Result<f32, String> {
        let next = self.consumed.checked_add(1)
            .ok_or_else(|| String::from("Uniform consumption count overflow"))?;
        let value = self.rng.next_uniform_rne();
        *self.consumed = next;
        Ok(value)
    }
}

struct ScalarMath {
    log: ScalarPositiveLog,
    power: ScalarPowerServices,
    failure: Option<MathDomain>,
}

impl ScalarMath {
    fn bind(profile: ScalarProfile) -> Result<Self, FailureReason> {
        let controls = current_controls()?;
        let log = ScalarPositiveLog::bind(profile.0, controls)
            .map_err(|_| FailureReason::UnsupportedFloatEnvironment { mxcsr: controls })?;
        let power = ScalarPowerServices::bind(controls)
            .map_err(|_| FailureReason::UnsupportedFloatEnvironment { mxcsr: controls })?;
        Ok(Self { log, power, failure: None })
    }

    fn fail(&mut self, domain: MathDomain) -> f32 {
        if self.failure.is_none() { self.failure = Some(domain); }

        f32::from_bits(0x7fc0_0000)
    }
}

impl GaussianMath for ScalarMath {
    fn log_radius(&mut self, value: f32) -> f32 {
        match self.log.evaluate(value) {
            Ok(value) => value,
            Err(_) => self.fail(MathDomain::Logarithm),
        }
    }
    fn binary_exponent(&mut self, value: f32) -> i32 {
        match self.power.exponent(value) {
            Ok(value) => value,
            Err(_) => { self.fail(MathDomain::Exponent); 0 }
        }
    }
    fn scale_power_two(&mut self, value: f32, exponent: i32) -> f32 {
        match self.power.scale(value, exponent) {
            Ok(value) => value,
            Err(_) => self.fail(MathDomain::Scaling),
        }
    }
    fn exceptional_sqrt(&mut self, _value: f32) -> f32 {
        self.fail(MathDomain::ExceptionalSquareRoot)
    }
}

fn current_controls() -> Result<u32, FailureReason> {
    #[cfg(all(windows, target_arch = "x86_64"))]
    {
        let mut mxcsr = 0u32;
        unsafe {
            std::arch::asm!("stmxcsr [{p}]", p = in(reg) &mut mxcsr,
                options(nostack, preserves_flags));
        }
        if mxcsr & 0xffc0 != 0x1f80 {
            Err(FailureReason::UnsupportedFloatEnvironment { mxcsr })
        } else {
            Ok(mxcsr)
        }
    }
    #[cfg(not(all(windows, target_arch = "x86_64")))]
    { Err(FailureReason::UnsupportedPlatform) }
}
