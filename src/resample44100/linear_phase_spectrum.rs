use super::ooura_table_f64::{RealDft, Trig64};

pub struct LinearPhaseSpectrum {
    pub integer_latency: usize,
    pub fft_len: usize,
    pub normalization: f64,
    pub packed: Vec<f64>,
}

impl LinearPhaseSpectrum {

    pub fn from_taps<M: Trig64>(taps: &[f64], gain: f64, math: &mut M)
        -> Result<Self, &'static str>
    {
        if taps.is_empty() || taps.len() % 2 != 1 || !gain.is_finite() || gain <= 0.0 {
            return Err("expected finite positive gain and odd source taps");
        }
        if taps.iter().any(|v| !v.is_finite()) {
            return Err("finite source-tap scope required");
        }
        if taps.iter().zip(taps.iter().rev()).any(|(a,b)| a.to_bits() != b.to_bits()) {
            return Err("source phase0 requires symmetric generated taps");
        }
        let m = taps.len()-1;
        let bit_length = (usize::BITS-m.leading_zeros()).max(1);
        let n = 2usize.checked_shl(bit_length).ok_or("source FFT size overflow")?;
        let mut fft = RealDft::new(n)?;
        let half = m/2;
        let mut sum = 0.0;

        for &tap in taps { sum += tap; }
        if !sum.is_finite() || sum == 0.0 { return Err("degenerate source-tap sum"); }
        let inverse_scale = 2.0 / n as f64;
        let scale = (gain * inverse_scale) / sum;
        let mut packed = vec![0.0; n];
        for i in 0..=half { packed[i] = taps[half+i] * scale; }
        for i in 1..=half { packed[n-i] = packed[i]; }
        fft.transform(&mut packed, true, math)?;

        for pair in packed[2..].chunks_exact_mut(2) { pair[1] = pair[0]; }
        Ok(Self { integer_latency:half, fft_len:n, normalization:scale, packed })
    }
}
