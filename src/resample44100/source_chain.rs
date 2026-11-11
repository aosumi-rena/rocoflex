use super::fir_phase0::Phase0Fir;
use super::fractional_table::{generic_auto_table, source_rational_rows, FractionalMathNames};
use super::generic_fractional::GenericFractional;
use super::halfband_down2::HalfbandDown2;
use super::halfband_up2::{HalfbandUp2, ProfileVariant};
use super::linear_phase_spectrum::LinearPhaseSpectrum;
use super::ordinary_44100_chain::OrdinaryMathNames;
use super::ordinary_selector::{ordinary_plan, source_rational, StagePlan};
use super::rational_fir::{RationalFir, RationalKernel};
use super::single_fir_reader::SingleFirMath;
use super::time_taps::phase0_supported_taps;

enum Stage {
    Fir(Phase0Fir),
    Up2(HalfbandUp2),
    Down2(HalfbandDown2),
    Rational(RationalFir),
    Generic(GenericFractional),
}

impl Stage {
    fn fractional_latency(&self) -> f64 {
        match self {
            Self::Fir(value) => value.layout.fractional_latency,
            Self::Up2(value) => value.fractional_latency(),
            Self::Down2(value) => value.fractional_latency(),
            Self::Rational(value) => value.unrepresented_latency_fraction,
            Self::Generic(_) => 0.0,
        }
    }

    fn reset(&mut self) -> Result<(), &'static str> {
        match self {
            Self::Fir(value) => value.reset()?,
            Self::Up2(value) => value.reset(),
            Self::Down2(value) => value.reset(),
            Self::Rational(value) => value.reset(),
            Self::Generic(value) => value.reset(),
        }
        Ok(())
    }

    fn process<M: SingleFirMath>(
        &mut self,
        input: &[f64],
        math: &mut M,
    ) -> Result<Vec<f64>, &'static str> {
        match self {
            Self::Fir(value) => value.process(input, math),
            Self::Up2(value) => value.process(input),
            Self::Down2(value) => value.process(input),
            Self::Rational(value) => {
                let mut output = Vec::new();
                value.process_into(input, &mut output)?;
                Ok(output)
            }
            Self::Generic(value) => {
                let mut output = Vec::new();
                value.process_into(input, &mut output)?;
                Ok(output)
            }
        }
    }
}

pub struct OrdinaryChain {
    stages: Vec<Stage>,
    failure: Option<&'static str>,
    pub startup_probe_extra_samples: usize,
}

impl OrdinaryChain {
    pub fn new<M: SingleFirMath>(
        source_rate: u32,
        names: OrdinaryMathNames,
        math: &mut M,
    ) -> Result<Self, &'static str> {
        if [names.exp, names.cos, names.log, names.sin, names.pow]
            .iter().any(|name| name.is_empty())
        {
            return Err("explicit named source CRT/trigonometric services required");
        }
        let attenuation = f64::from_bits(0x4069dd1eb851eb85);
        let plan = ordinary_plan(source_rate)?;
        let mut stages = Vec::with_capacity(plan.len());
        let mut latency = 0.0;
        for item in plan {
            let stage = match item {
                StagePlan::Fir { up, down, cutoff, gain, transition } => {
                    let taps = phase0_supported_taps(cutoff, transition, attenuation, math)?;
                    let spectrum = LinearPhaseSpectrum::from_taps(&taps.taps, gain, math)?;
                    Stage::Fir(Phase0Fir::from_generated_spectrum(
                        spectrum, up as usize, down as usize, latency,
                    )?)
                }
                StagePlan::Up2 { index, variant } =>
                    Stage::Up2(HalfbandUp2::ordinary(index, variant, latency)?),
                StagePlan::Down2 { index, variant } =>
                    Stage::Down2(HalfbandDown2::ordinary(index, variant, latency)?),
                StagePlan::Fractional { source, target, variant } => {
                    let variant = matches!(variant, ProfileVariant::True);
                    if let Some((numerator, phases)) = source_rational(source, target) {
                        let services = FractionalMathNames {
                            exp: names.exp, sin: names.sin, pow: names.pow,
                        };
                        let coefficients = source_rational_rows(phases, variant, services, math)?;
                        let kernel = if variant { RationalKernel::True22 }
                            else { RationalKernel::False28 };
                        Stage::Rational(RationalFir::from_source_table(
                            coefficients, numerator, phases, kernel, latency,
                        )?)
                    } else {
                        let table = generic_auto_table(attenuation, variant, math)?;
                        Stage::Generic(GenericFractional::from_source_table(
                            table.table, source, target, latency,
                        )?)
                    }
                }
            };
            latency = stage.fractional_latency();
            stages.push(stage);
        }
        let mut value = Self {
            stages, failure: None, startup_probe_extra_samples: 0,
        };
        loop {
            if !value.process(&[0.0], math)?.is_empty() { break; }
            value.startup_probe_extra_samples += 1;
            if value.startup_probe_extra_samples > 1 << 20 {
                return Err("own constructor startup-probe resource bound exceeded");
            }
        }
        value.reset()?;
        Ok(value)
    }

    pub fn reset(&mut self) -> Result<(), &'static str> {
        if self.failure.is_some() {
            return Err("own failed source chain requires full reconstruction");
        }
        self.failure = Some("own source chain reset interrupted before completion");
        for stage in &mut self.stages {
            if let Err(error) = stage.reset() {
                self.failure = Some(error);
                return Err(error);
            }
        }
        self.failure = None;
        Ok(())
    }

    pub fn failure(&self) -> Option<&'static str> {
        self.failure
    }

    pub fn requires_reconstruction(&self) -> bool {
        self.failure.is_some()
    }

    pub fn process<M: SingleFirMath>(
        &mut self,
        input: &[f64],
        math: &mut M,
    ) -> Result<Vec<f64>, &'static str> {
        if let Some(error) = self.failure { return Err(error); }
        if input.iter().any(|x| !x.is_finite()) {
            return Err("finite source chain input required");
        }
        let mut output = Vec::new();
        for (index, stage) in self.stages.iter_mut().enumerate() {
            let source = if index == 0 { input } else { &output };
            self.failure = Some("own source stage interrupted before completion");
            let next = match stage.process(source, math) {
                Ok(next) => next,
                Err(error) => {

                    self.failure = Some(error);
                    return Err(error);
                }
            };
            output = next;
        }
        if self.stages.is_empty() { output.extend_from_slice(input); }
        self.failure = None;
        Ok(output)
    }
}
