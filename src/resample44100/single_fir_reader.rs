use super::fir_phase0::Phase0Fir;
use super::halfband_down2::HalfbandDown2;
use super::fractional_profile_data::{FALSE_PROFILES, TRUE_PROFILES};
use super::fractional_table::{generic_auto_table, rational_22_rows, rational_28_rows,
    FractionalMath, FractionalMathNames};
use super::generic_fractional::GenericFractional;
use super::halfband_up2::{HalfbandUp2, ProfileVariant};
use super::linear_phase_spectrum::LinearPhaseSpectrum;
use super::ooura_table_f64::Trig64;
use super::ordinary_44100_chain::OrdinaryMathNames;
use super::rational_fir::{RationalFir, RationalKernel};
use super::time_taps::{phase0_supported_taps, WideDesignMath};
use std::collections::VecDeque;

#[derive(Clone, Copy)]
pub enum SourceRate {
    Hz4000,
    Hz6000,
    Hz8000,
    Hz11025,
    Hz12000,
    Hz16000,
    Hz22050,
    Hz24000,
    Hz32000,
    Hz48001,
    Hz64000,
    Hz72000,
    Hz88200,
    Hz96000,
    Hz144000,
    Hz176400,
    Hz192000,
    Hz288000,
    Hz352800,
    Hz384000,
}

impl SourceRate {
    pub fn sample_rate(self) -> u32 {
        match self {
            Self::Hz4000 => 4_000,
            Self::Hz6000 => 6_000,
            Self::Hz8000 => 8_000,
            Self::Hz11025 => 11_025,
            Self::Hz12000 => 12_000,
            Self::Hz16000 => 16_000,
            Self::Hz22050 => 22_050,
            Self::Hz24000 => 24_000,
            Self::Hz32000 => 32_000,
            Self::Hz48001 => 48_001,
            Self::Hz64000 => 64_000,
            Self::Hz72000 => 72_000,
            Self::Hz88200 => 88_200,
            Self::Hz96000 => 96_000,
            Self::Hz144000 => 144_000,
            Self::Hz176400 => 176_400,
            Self::Hz192000 => 192_000,
            Self::Hz288000 => 288_000,
            Self::Hz352800 => 352_800,
            Self::Hz384000 => 384_000,
        }
    }

    fn filter_parameters(self) -> (usize, usize, f64, f64) {
        match self {
            Self::Hz4000 | Self::Hz8000 => (3, 1, 1.0 / 3.0, 3.0),
            Self::Hz6000 | Self::Hz12000 => (2, 1, 0.5, 2.0),
            Self::Hz11025 => (2, 1, 0.5, 2.0),
            Self::Hz16000 => (3, 1, 1.0 / 3.0, 3.0),
            Self::Hz22050 => (2, 1, 0.5, 2.0),
            Self::Hz24000 => (2, 1, 0.5, 2.0),
            Self::Hz32000 => (3, 2, 1.0 / 3.0, 3.0),
            Self::Hz48001 => (2, 1, (0.5 * 48000.0) / 48001.0, 2.0),
            Self::Hz64000 => (3, 4, 0.25, 3.0),
            Self::Hz72000 => (2, 3, 1.0 / 3.0, 2.0),
            Self::Hz88200 => (2, 1, (0.5 * 48000.0) / 88200.0, 2.0),
            Self::Hz96000 => (1, 2, 0.5, 1.0),
            Self::Hz144000 => (1, 3, 1.0 / 3.0, 1.0),
            Self::Hz176400 => (1, 1, 48000.0 / 176400.0, 1.0),
            Self::Hz192000 => (1, 2, 0.5, 0.5),
            Self::Hz288000 => (1, 3, 1.0 / 3.0, 0.5),
            Self::Hz352800 => (1, 1, 96000.0 / 352800.0, 0.5),
            Self::Hz384000 => (1, 2, 0.5, 0.25),
        }
    }

    fn halfband_stages(self) -> (u32, ProfileVariant) {
        match self {
            Self::Hz4000 => (2, ProfileVariant::True),
            Self::Hz6000 => (2, ProfileVariant::False),
            Self::Hz8000 => (1, ProfileVariant::True),
            Self::Hz11025 => (1, ProfileVariant::False),
            Self::Hz12000 => (1, ProfileVariant::False),
            _ => (0, ProfileVariant::False),
        }
    }

    fn rational_phases(self) -> Option<usize> {
        match self {
            Self::Hz11025 => Some(80),
            Self::Hz22050 => Some(160),
            Self::Hz88200 => Some(40),
            Self::Hz176400 => Some(40),
            Self::Hz352800 => Some(40),
            _ => None,
        }
    }

    fn down_stages(self) -> (u32, ProfileVariant) {
        match self {
            Self::Hz192000 => (1, ProfileVariant::False),
            Self::Hz288000 => (1, ProfileVariant::True),
            Self::Hz352800 => (1, ProfileVariant::True),
            Self::Hz384000 => (2, ProfileVariant::False),
            _ => (0, ProfileVariant::False),
        }
    }

    fn post_filter_transition(self) -> Option<f64> {
        match self {
            Self::Hz11025 => {
                let mut relative_rate = 4.0;
                relative_rate *= f64::from(self.sample_rate());
                relative_rate /= 48000.0;
                let mut transition = 1.0 - relative_rate;
                transition /= f64::from_bits(0x3f91eb851eb851ec);
                Some(f64::from_bits(0x4046800000000000).min(transition))
            }
            _ => None,
        }
    }

    fn rational_kernel(self) -> RationalKernel {
        match self {
            Self::Hz176400 | Self::Hz352800 => RationalKernel::True22,
            _ => RationalKernel::False28,
        }
    }

    fn generic_rates(self) -> Option<(f64, f64, bool)> {
        match self {
            Self::Hz48001 => Some((96002.0, 48000.0, false)),
            _ => None,
        }
    }
}

pub trait SingleFirMath: WideDesignMath + Trig64 + FractionalMath {}
impl<T: WideDesignMath + Trig64 + FractionalMath> SingleFirMath for T {}

pub fn short_import_output_count(
    source_count: usize,
    rate: SourceRate,
) -> Result<usize, &'static str> {
    let source_rate = rate.sample_rate() as usize;
    if source_count == 0 || source_count > source_rate * 300 {
        return Err("own nonempty decoded input duration<=300s scope required");
    }
    let duration = source_count as f64 / source_rate as f64;
    Ok((duration * 48000.0).trunc() as usize)
}

pub struct SingleFirReader<'a> {
    source: &'a [f32],
    source_rate: SourceRate,
    downbands: Vec<HalfbandDown2>,
    filter: Phase0Fir,
    halfbands: Vec<HalfbandUp2>,
    rational: Option<RationalFir>,
    generic: Option<GenericFractional>,
    post_filter: Option<Phase0Fir>,
    fifo: VecDeque<f32>,
    source_position: usize,
    prepared_source_capacity: usize,
}

impl<'a> SingleFirReader<'a> {
    pub fn new<M: SingleFirMath>(
        source: &'a [f32],
        rate: SourceRate,
        names: OrdinaryMathNames,
        math: &mut M,
    ) -> Result<Self, &'static str> {
        short_import_output_count(source.len(), rate)?;
        if source.iter().any(|x| !x.is_finite()) {
            return Err("finite decoded channel0 input required");
        }
        if [names.exp, names.cos, names.log, names.sin, names.pow]
            .iter().any(|name| name.is_empty())
        {
            return Err("explicit named source CRT/trigonometric services required");
        }

        let ratio = f64::from(rate.sample_rate()) / 48000.0;
        let prepared_bits = (512.0 * ratio + 6755399441055744.0_f64).to_bits();
        let prepared_source_capacity = (prepared_bits as u32) as usize + 64;
        let attenuation = f64::from_bits(0x4069dd1eb851eb85);
        let (down_count, down_variant) = rate.down_stages();
        let mut latency = 0.0;
        let mut downbands = Vec::with_capacity(down_count as usize);
        for index in (0..down_count).rev() {
            let stage = HalfbandDown2::ordinary(index, down_variant, latency)?;
            latency = stage.fractional_latency();
            downbands.push(stage);
        }
        let (up, down, cutoff, gain) = rate.filter_parameters();
        let taps = phase0_supported_taps(cutoff, 2.0, attenuation, math)?;
        let spectrum = LinearPhaseSpectrum::from_taps(&taps.taps, gain, math)?;
        let filter = Phase0Fir::from_generated_spectrum(spectrum, up, down, latency)?;
        latency = filter.layout.fractional_latency;
        let rational = if let Some(phases) = rate.rational_phases() {
            let services = FractionalMathNames { exp: names.exp, sin: names.sin, pow: names.pow };
            let kernel = rate.rational_kernel();
            let coefficients = match kernel {
                RationalKernel::False28 =>
                    rational_28_rows(phases, FALSE_PROFILES[10][2], services, math)?,
                RationalKernel::True22 =>
                    rational_22_rows(phases, TRUE_PROFILES[8][2], services, math)?,
            };
            let stage = RationalFir::source_147(coefficients, phases, kernel, latency)?;
            latency = stage.unrepresented_latency_fraction;
            Some(stage)
        } else {
            None
        };
        let generic = if let Some((source_rate, target_rate, variant)) = rate.generic_rates() {
            let table = generic_auto_table(attenuation, variant, math)?;
            let stage = GenericFractional::from_source_table(
                table.table, source_rate, target_rate, latency,
            )?;
            latency = 0.0;
            Some(stage)
        } else {
            None
        };
        let post_filter = if let Some(transition) = rate.post_filter_transition() {
            let taps = phase0_supported_taps(0.5, transition, attenuation, math)?;
            let spectrum = LinearPhaseSpectrum::from_taps(&taps.taps, 2.0, math)?;
            let stage = Phase0Fir::from_generated_spectrum(spectrum, 2, 1, latency)?;
            latency = stage.layout.fractional_latency;
            Some(stage)
        } else {
            None
        };
        let (stage_count, variant) = rate.halfband_stages();
        let mut halfbands = Vec::with_capacity(stage_count as usize);
        for index in 0..stage_count {
            let stage = HalfbandUp2::ordinary(index, variant, latency)?;
            latency = stage.fractional_latency();
            halfbands.push(stage);
        }
        let mut value = Self {
            source, source_rate: rate, downbands, filter, halfbands, rational, generic,
            post_filter,
            fifo: VecDeque::new(),
            source_position: 0, prepared_source_capacity,
        };

        let mut extra = 0usize;
        loop {
            if !value.process_f64(&[0.0], math)?.is_empty() {
                break;
            }
            extra += 1;
            if extra > 1 << 20 {
                return Err("own constructor startup-probe resource bound exceeded");
            }
        }
        for stage in &mut value.downbands { stage.reset(); }
        value.filter.reset()?;
        if let Some(stage) = &mut value.rational { stage.reset(); }
        if let Some(stage) = &mut value.generic { stage.reset(); }
        if let Some(stage) = &mut value.post_filter { stage.reset()?; }
        for stage in &mut value.halfbands { stage.reset(); }
        Ok(value)
    }

    fn process_f64<M: SingleFirMath>(&mut self, input: &[f64], math: &mut M)
        -> Result<Vec<f64>, &'static str>
    {
        let mut output = if self.downbands.is_empty() {
            self.filter.process(input, math)?
        } else {
            let mut reduced = self.downbands[0].process(input)?;
            for stage in self.downbands.iter_mut().skip(1) {
                reduced = stage.process(&reduced)?;
            }
            self.filter.process(&reduced, math)?
        };
        if let Some(stage) = &mut self.rational {
            let mut final_output = Vec::new();
            stage.process_into(&output, &mut final_output)?;
            output = final_output;
        }
        if let Some(stage) = &mut self.generic {
            let mut final_output = Vec::new();
            stage.process_into(&output, &mut final_output)?;
            output = final_output;
        }
        if let Some(stage) = &mut self.post_filter { output = stage.process(&output, math)?; }
        for stage in &mut self.halfbands { output = stage.process(&output)?; }
        Ok(output)
    }

    pub fn read_block<M: SingleFirMath>(
        &mut self,
        count: usize,
        math: &mut M,
    ) -> Result<Vec<f32>, &'static str> {
        if count > 512 {
            return Err("own ordinary prepare512 output-block scope required");
        }
        while self.fifo.len() < count {
            let missing = count - self.fifo.len();
            let mut requested = missing as f64;
            requested *= f64::from(self.source_rate.sample_rate()) / 48000.0;
            requested += 64.0;
            let requested = requested.trunc() as usize;
            if requested == 0 || requested > self.prepared_source_capacity {
                return Err("source prepared-capacity/progress invariant failed");
            }
            let begin = self.source_position.min(self.source.len());
            let available = requested.min(self.source.len() - begin);
            let mut widened = vec![0.0; requested];
            for (target, source) in widened[..available].iter_mut()
                .zip(&self.source[begin..begin + available])
            {
                *target = *source as f64;
            }
            self.source_position += requested;
            let produced = self.process_f64(&widened, math)?;
            self.fifo.extend(produced.into_iter().map(|x| x as f32));
        }
        Ok(self.fifo.drain(..count).collect())
    }
}

pub fn convert_short_channel0<M: SingleFirMath>(
    source: &[f32],
    rate: SourceRate,
    names: OrdinaryMathNames,
    math: &mut M,
) -> Result<Vec<f32>, &'static str> {
    let count = short_import_output_count(source.len(), rate)?;
    let mut reader = SingleFirReader::new(source, rate, names, math)?;
    let mut output = Vec::with_capacity(count);
    while output.len() < count {
        let requested = (count - output.len()).min(512);
        output.extend(reader.read_block(requested, math)?);
    }
    Ok(output)
}
