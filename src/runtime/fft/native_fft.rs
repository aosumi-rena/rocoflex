//! Real forward FFT adapted from Takuya Ooura's General Purpose FFT Package.
//! Copyright Takuya OOURA, 1996-2001. See ../licenses/Ooura.txt.
#![allow(unused_assignments, unused_parens, clippy::excessive_precision)]

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TwiddleSite { CftMdl1, CftMdl2, CftB1st, RftFsub, RftBsub }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TwiddleOperation { Cosine57ddb0, Sine586960 }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TwiddleScale { One, Half }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TwiddleRequest {
    pub site: TwiddleSite, pub refresh: bool, pub operation: TwiddleOperation,
    pub angle_f32_bits: u32, pub scale: TwiddleScale,
}
pub trait TwiddleMath {
    type Error;
    fn evaluate(&mut self, request: TwiddleRequest) -> Result<u32, Self::Error>;
}
pub enum TwiddleFftError<E> { UnsupportedSize { length: usize }, Math(E) }
struct HostTwiddle;
impl TwiddleMath for HostTwiddle {
    type Error = std::convert::Infallible;
    fn evaluate(&mut self, request: TwiddleRequest) -> Result<u32, Self::Error> {
        let angle = f32::from_bits(request.angle_f32_bits) as f64;
        let value = match request.operation {
            TwiddleOperation::Cosine57ddb0 => angle.cos(),
            TwiddleOperation::Sine586960 => angle.sin(),
        };
        let value = match request.scale { TwiddleScale::One => value, TwiddleScale::Half => 0.5_f64 * value };
        Ok((value as f32).to_bits())
    }
}
fn twiddle<M: TwiddleMath>(math: &mut M, site: TwiddleSite, refresh: bool,
    operation: TwiddleOperation, angle: f32, scale: TwiddleScale) -> Result<f32, M::Error>
{
    math.evaluate(TwiddleRequest { site, refresh, operation, angle_f32_bits: angle.to_bits(), scale })
        .map(f32::from_bits)
}

pub fn cdft_in_place(a: &mut [f32], positive_exponent: bool) -> crate::Result<()> {
    if a.len()<4 || a.len()>8192 || !a.len().is_power_of_two()
        || a.iter().any(|value| !value.is_finite()) {
        return Err("complex FFT requires 4..8192 finite power-of-two interleaved lanes".into());
    }
    let result=if positive_exponent {
        cftfsub(a.len() as i32,a,&mut HostTwiddle)
    } else {
        cftbsub(a.len() as i32,a,&mut HostTwiddle)
    };
    match result { Ok(())=>{}, Err(never)=>match never {} }
    if a.iter().any(|value| !value.is_finite()) {
        return Err("nonfinite complex FFT output".into());
    }
    Ok(())
}

pub fn rdft_forward(a: &mut [f32]) -> crate::Result<()> {
    match rdft_forward_with_twiddle(a, &mut HostTwiddle) {
        Ok(()) => Ok(()),
        Err(TwiddleFftError::UnsupportedSize { .. }) => Err("unsupported source RDFT size".into()),
        Err(TwiddleFftError::Math(never)) => match never {},
    }
}

pub fn rdft_forward_with_twiddle<M: TwiddleMath>(a: &mut [f32], math: &mut M)
    -> Result<(), TwiddleFftError<M::Error>>
{
    if ![512, 1024, 2048].contains(&a.len()) {
        return Err(TwiddleFftError::UnsupportedSize { length: a.len() });
    }
    let n = a.len() as i32;
    cftfsub(n, a, math).map_err(TwiddleFftError::Math)?;
    rftfsub(n, a, math).map_err(TwiddleFftError::Math)?;
    let xi = a[0] - a[1];
    a[0] += a[1];
    a[1] = xi;
    Ok(())
}

fn cftfsub<M: TwiddleMath>(n: i32, a: &mut [f32], math: &mut M) -> Result<(), M::Error> {
    if (n > 8i32) {
        if (n > 32i32) {
            cftmdl1(n, a, math)?;
            if (n > 512i32) {
                cftrec4(n, a, math)?;
            } else {
                if (n > 128i32) {
                    cftleaf(n, 1i32, a, math)?;
                } else {
                    cftfx41(n, a);
                }
            }
            bitrv2(n, a);
        } else {
            if (n == 32i32) {
                cftf161(a);
                bitrv216(a);
            } else {
                cftf081(a);
                bitrv208(a);
            }
        }
    } else {
        if (n == 8i32) {
            cftf040(a);
        } else {
            if (n == 4i32) {
                cftx020(a);
            }
        }
    }
    Ok(())
}

fn bitrv2(n: i32, a: &mut [f32]) {
    let mut j0: i32 = 0i32;
    let mut k0: i32 = 0i32;
    let mut j1: i32 = 0i32;
    let mut k1: i32 = 0i32;
    let mut l: i32 = 0i32;
    let mut m: i32 = 0i32;
    let mut i: i32 = 0i32;
    let mut j: i32 = 0i32;
    let mut k: i32 = 0i32;
    let mut nh: i32 = 0i32;
    let mut xr: f32 = 0f32;
    let mut xi: f32 = 0f32;
    let mut yr: f32 = 0f32;
    let mut yi: f32 = 0f32;
    m = 4i32;
    {
        l = (n >> 2i32);
        while (l > 8i32) {
            m = (m << 1i32);
            l = (l >> 2i32);
        }
    }
    nh = (n >> 1i32);
    if (l == 8i32) {
        j0 = 0i32;
        {
            k0 = 0i32;
            while (k0 < m) {
                k = k0;
                {
                    j = j0;
                    while (j < (j0 + k0)) {
                        xr = a[(j) as usize];
                        xi = a[(j + 1i32) as usize];
                        yr = a[(k) as usize];
                        yi = a[(k + 1i32) as usize];
                        a[(j) as usize] = yr;
                        a[(j + 1i32) as usize] = yi;
                        a[(k) as usize] = xr;
                        a[(k + 1i32) as usize] = xi;
                        j1 = (j + m);
                        k1 = (k + (2i32 * m));
                        xr = a[(j1) as usize];
                        xi = a[(j1 + 1i32) as usize];
                        yr = a[(k1) as usize];
                        yi = a[(k1 + 1i32) as usize];
                        a[(j1) as usize] = yr;
                        a[(j1 + 1i32) as usize] = yi;
                        a[(k1) as usize] = xr;
                        a[(k1 + 1i32) as usize] = xi;
                        j1 = (j1 + m);
                        k1 = (k1 - m);
                        xr = a[(j1) as usize];
                        xi = a[(j1 + 1i32) as usize];
                        yr = a[(k1) as usize];
                        yi = a[(k1 + 1i32) as usize];
                        a[(j1) as usize] = yr;
                        a[(j1 + 1i32) as usize] = yi;
                        a[(k1) as usize] = xr;
                        a[(k1 + 1i32) as usize] = xi;
                        j1 = (j1 + m);
                        k1 = (k1 + (2i32 * m));
                        xr = a[(j1) as usize];
                        xi = a[(j1 + 1i32) as usize];
                        yr = a[(k1) as usize];
                        yi = a[(k1 + 1i32) as usize];
                        a[(j1) as usize] = yr;
                        a[(j1 + 1i32) as usize] = yi;
                        a[(k1) as usize] = xr;
                        a[(k1 + 1i32) as usize] = xi;
                        j1 = (j1 + nh);
                        k1 = (k1 + 2i32);
                        xr = a[(j1) as usize];
                        xi = a[(j1 + 1i32) as usize];
                        yr = a[(k1) as usize];
                        yi = a[(k1 + 1i32) as usize];
                        a[(j1) as usize] = yr;
                        a[(j1 + 1i32) as usize] = yi;
                        a[(k1) as usize] = xr;
                        a[(k1 + 1i32) as usize] = xi;
                        j1 = (j1 - m);
                        k1 = (k1 - (2i32 * m));
                        xr = a[(j1) as usize];
                        xi = a[(j1 + 1i32) as usize];
                        yr = a[(k1) as usize];
                        yi = a[(k1 + 1i32) as usize];
                        a[(j1) as usize] = yr;
                        a[(j1 + 1i32) as usize] = yi;
                        a[(k1) as usize] = xr;
                        a[(k1 + 1i32) as usize] = xi;
                        j1 = (j1 - m);
                        k1 = (k1 + m);
                        xr = a[(j1) as usize];
                        xi = a[(j1 + 1i32) as usize];
                        yr = a[(k1) as usize];
                        yi = a[(k1 + 1i32) as usize];
                        a[(j1) as usize] = yr;
                        a[(j1 + 1i32) as usize] = yi;
                        a[(k1) as usize] = xr;
                        a[(k1 + 1i32) as usize] = xi;
                        j1 = (j1 - m);
                        k1 = (k1 - (2i32 * m));
                        xr = a[(j1) as usize];
                        xi = a[(j1 + 1i32) as usize];
                        yr = a[(k1) as usize];
                        yi = a[(k1 + 1i32) as usize];
                        a[(j1) as usize] = yr;
                        a[(j1 + 1i32) as usize] = yi;
                        a[(k1) as usize] = xr;
                        a[(k1 + 1i32) as usize] = xi;
                        j1 = (j1 + 2i32);
                        k1 = (k1 + nh);
                        xr = a[(j1) as usize];
                        xi = a[(j1 + 1i32) as usize];
                        yr = a[(k1) as usize];
                        yi = a[(k1 + 1i32) as usize];
                        a[(j1) as usize] = yr;
                        a[(j1 + 1i32) as usize] = yi;
                        a[(k1) as usize] = xr;
                        a[(k1 + 1i32) as usize] = xi;
                        j1 = (j1 + m);
                        k1 = (k1 + (2i32 * m));
                        xr = a[(j1) as usize];
                        xi = a[(j1 + 1i32) as usize];
                        yr = a[(k1) as usize];
                        yi = a[(k1 + 1i32) as usize];
                        a[(j1) as usize] = yr;
                        a[(j1 + 1i32) as usize] = yi;
                        a[(k1) as usize] = xr;
                        a[(k1 + 1i32) as usize] = xi;
                        j1 = (j1 + m);
                        k1 = (k1 - m);
                        xr = a[(j1) as usize];
                        xi = a[(j1 + 1i32) as usize];
                        yr = a[(k1) as usize];
                        yi = a[(k1 + 1i32) as usize];
                        a[(j1) as usize] = yr;
                        a[(j1 + 1i32) as usize] = yi;
                        a[(k1) as usize] = xr;
                        a[(k1 + 1i32) as usize] = xi;
                        j1 = (j1 + m);
                        k1 = (k1 + (2i32 * m));
                        xr = a[(j1) as usize];
                        xi = a[(j1 + 1i32) as usize];
                        yr = a[(k1) as usize];
                        yi = a[(k1 + 1i32) as usize];
                        a[(j1) as usize] = yr;
                        a[(j1 + 1i32) as usize] = yi;
                        a[(k1) as usize] = xr;
                        a[(k1 + 1i32) as usize] = xi;
                        j1 = (j1 - nh);
                        k1 = (k1 - 2i32);
                        xr = a[(j1) as usize];
                        xi = a[(j1 + 1i32) as usize];
                        yr = a[(k1) as usize];
                        yi = a[(k1 + 1i32) as usize];
                        a[(j1) as usize] = yr;
                        a[(j1 + 1i32) as usize] = yi;
                        a[(k1) as usize] = xr;
                        a[(k1 + 1i32) as usize] = xi;
                        j1 = (j1 - m);
                        k1 = (k1 - (2i32 * m));
                        xr = a[(j1) as usize];
                        xi = a[(j1 + 1i32) as usize];
                        yr = a[(k1) as usize];
                        yi = a[(k1 + 1i32) as usize];
                        a[(j1) as usize] = yr;
                        a[(j1 + 1i32) as usize] = yi;
                        a[(k1) as usize] = xr;
                        a[(k1 + 1i32) as usize] = xi;
                        j1 = (j1 - m);
                        k1 = (k1 + m);
                        xr = a[(j1) as usize];
                        xi = a[(j1 + 1i32) as usize];
                        yr = a[(k1) as usize];
                        yi = a[(k1 + 1i32) as usize];
                        a[(j1) as usize] = yr;
                        a[(j1 + 1i32) as usize] = yi;
                        a[(k1) as usize] = xr;
                        a[(k1 + 1i32) as usize] = xi;
                        j1 = (j1 - m);
                        k1 = (k1 - (2i32 * m));
                        xr = a[(j1) as usize];
                        xi = a[(j1 + 1i32) as usize];
                        yr = a[(k1) as usize];
                        yi = a[(k1 + 1i32) as usize];
                        a[(j1) as usize] = yr;
                        a[(j1 + 1i32) as usize] = yi;
                        a[(k1) as usize] = xr;
                        a[(k1 + 1i32) as usize] = xi;
                        {
                            i = (nh >> 1i32);
                            while (i > {
                                k = (k ^ i);
                                k
                            }) {
                                i = (i >> 1i32);
                            }
                        }
                        j = (j + 4i32);
                    }
                }
                k1 = (j0 + k0);
                j1 = (k1 + 2i32);
                k1 = (k1 + nh);
                xr = a[(j1) as usize];
                xi = a[(j1 + 1i32) as usize];
                yr = a[(k1) as usize];
                yi = a[(k1 + 1i32) as usize];
                a[(j1) as usize] = yr;
                a[(j1 + 1i32) as usize] = yi;
                a[(k1) as usize] = xr;
                a[(k1 + 1i32) as usize] = xi;
                j1 = (j1 + m);
                k1 = (k1 + (2i32 * m));
                xr = a[(j1) as usize];
                xi = a[(j1 + 1i32) as usize];
                yr = a[(k1) as usize];
                yi = a[(k1 + 1i32) as usize];
                a[(j1) as usize] = yr;
                a[(j1 + 1i32) as usize] = yi;
                a[(k1) as usize] = xr;
                a[(k1 + 1i32) as usize] = xi;
                j1 = (j1 + m);
                k1 = (k1 - m);
                xr = a[(j1) as usize];
                xi = a[(j1 + 1i32) as usize];
                yr = a[(k1) as usize];
                yi = a[(k1 + 1i32) as usize];
                a[(j1) as usize] = yr;
                a[(j1 + 1i32) as usize] = yi;
                a[(k1) as usize] = xr;
                a[(k1 + 1i32) as usize] = xi;
                j1 = (j1 - 2i32);
                k1 = (k1 - nh);
                xr = a[(j1) as usize];
                xi = a[(j1 + 1i32) as usize];
                yr = a[(k1) as usize];
                yi = a[(k1 + 1i32) as usize];
                a[(j1) as usize] = yr;
                a[(j1 + 1i32) as usize] = yi;
                a[(k1) as usize] = xr;
                a[(k1 + 1i32) as usize] = xi;
                j1 = (j1 + (nh + 2i32));
                k1 = (k1 + (nh + 2i32));
                xr = a[(j1) as usize];
                xi = a[(j1 + 1i32) as usize];
                yr = a[(k1) as usize];
                yi = a[(k1 + 1i32) as usize];
                a[(j1) as usize] = yr;
                a[(j1 + 1i32) as usize] = yi;
                a[(k1) as usize] = xr;
                a[(k1 + 1i32) as usize] = xi;
                j1 = (j1 - (nh - m));
                k1 = (k1 + ((2i32 * m) - 2i32));
                xr = a[(j1) as usize];
                xi = a[(j1 + 1i32) as usize];
                yr = a[(k1) as usize];
                yi = a[(k1 + 1i32) as usize];
                a[(j1) as usize] = yr;
                a[(j1 + 1i32) as usize] = yi;
                a[(k1) as usize] = xr;
                a[(k1 + 1i32) as usize] = xi;
                {
                    i = (nh >> 1i32);
                    while (i > {
                        j0 = (j0 ^ i);
                        j0
                    }) {
                        i = (i >> 1i32);
                    }
                }
                k0 = (k0 + 4i32);
            }
        }
    } else {
        j0 = 0i32;
        {
            k0 = 0i32;
            while (k0 < m) {
                k = k0;
                {
                    j = j0;
                    while (j < (j0 + k0)) {
                        xr = a[(j) as usize];
                        xi = a[(j + 1i32) as usize];
                        yr = a[(k) as usize];
                        yi = a[(k + 1i32) as usize];
                        a[(j) as usize] = yr;
                        a[(j + 1i32) as usize] = yi;
                        a[(k) as usize] = xr;
                        a[(k + 1i32) as usize] = xi;
                        j1 = (j + m);
                        k1 = (k + m);
                        xr = a[(j1) as usize];
                        xi = a[(j1 + 1i32) as usize];
                        yr = a[(k1) as usize];
                        yi = a[(k1 + 1i32) as usize];
                        a[(j1) as usize] = yr;
                        a[(j1 + 1i32) as usize] = yi;
                        a[(k1) as usize] = xr;
                        a[(k1 + 1i32) as usize] = xi;
                        j1 = (j1 + nh);
                        k1 = (k1 + 2i32);
                        xr = a[(j1) as usize];
                        xi = a[(j1 + 1i32) as usize];
                        yr = a[(k1) as usize];
                        yi = a[(k1 + 1i32) as usize];
                        a[(j1) as usize] = yr;
                        a[(j1 + 1i32) as usize] = yi;
                        a[(k1) as usize] = xr;
                        a[(k1 + 1i32) as usize] = xi;
                        j1 = (j1 - m);
                        k1 = (k1 - m);
                        xr = a[(j1) as usize];
                        xi = a[(j1 + 1i32) as usize];
                        yr = a[(k1) as usize];
                        yi = a[(k1 + 1i32) as usize];
                        a[(j1) as usize] = yr;
                        a[(j1 + 1i32) as usize] = yi;
                        a[(k1) as usize] = xr;
                        a[(k1 + 1i32) as usize] = xi;
                        j1 = (j1 + 2i32);
                        k1 = (k1 + nh);
                        xr = a[(j1) as usize];
                        xi = a[(j1 + 1i32) as usize];
                        yr = a[(k1) as usize];
                        yi = a[(k1 + 1i32) as usize];
                        a[(j1) as usize] = yr;
                        a[(j1 + 1i32) as usize] = yi;
                        a[(k1) as usize] = xr;
                        a[(k1 + 1i32) as usize] = xi;
                        j1 = (j1 + m);
                        k1 = (k1 + m);
                        xr = a[(j1) as usize];
                        xi = a[(j1 + 1i32) as usize];
                        yr = a[(k1) as usize];
                        yi = a[(k1 + 1i32) as usize];
                        a[(j1) as usize] = yr;
                        a[(j1 + 1i32) as usize] = yi;
                        a[(k1) as usize] = xr;
                        a[(k1 + 1i32) as usize] = xi;
                        j1 = (j1 - nh);
                        k1 = (k1 - 2i32);
                        xr = a[(j1) as usize];
                        xi = a[(j1 + 1i32) as usize];
                        yr = a[(k1) as usize];
                        yi = a[(k1 + 1i32) as usize];
                        a[(j1) as usize] = yr;
                        a[(j1 + 1i32) as usize] = yi;
                        a[(k1) as usize] = xr;
                        a[(k1 + 1i32) as usize] = xi;
                        j1 = (j1 - m);
                        k1 = (k1 - m);
                        xr = a[(j1) as usize];
                        xi = a[(j1 + 1i32) as usize];
                        yr = a[(k1) as usize];
                        yi = a[(k1 + 1i32) as usize];
                        a[(j1) as usize] = yr;
                        a[(j1 + 1i32) as usize] = yi;
                        a[(k1) as usize] = xr;
                        a[(k1 + 1i32) as usize] = xi;
                        {
                            i = (nh >> 1i32);
                            while (i > {
                                k = (k ^ i);
                                k
                            }) {
                                i = (i >> 1i32);
                            }
                        }
                        j = (j + 4i32);
                    }
                }
                k1 = (j0 + k0);
                j1 = (k1 + 2i32);
                k1 = (k1 + nh);
                xr = a[(j1) as usize];
                xi = a[(j1 + 1i32) as usize];
                yr = a[(k1) as usize];
                yi = a[(k1 + 1i32) as usize];
                a[(j1) as usize] = yr;
                a[(j1 + 1i32) as usize] = yi;
                a[(k1) as usize] = xr;
                a[(k1 + 1i32) as usize] = xi;
                j1 = (j1 + m);
                k1 = (k1 + m);
                xr = a[(j1) as usize];
                xi = a[(j1 + 1i32) as usize];
                yr = a[(k1) as usize];
                yi = a[(k1 + 1i32) as usize];
                a[(j1) as usize] = yr;
                a[(j1 + 1i32) as usize] = yi;
                a[(k1) as usize] = xr;
                a[(k1 + 1i32) as usize] = xi;
                {
                    i = (nh >> 1i32);
                    while (i > {
                        j0 = (j0 ^ i);
                        j0
                    }) {
                        i = (i >> 1i32);
                    }
                }
                k0 = (k0 + 4i32);
            }
        }
    }
}

fn bitrv216(a: &mut [f32]) {
    let mut x1r: f32 = 0f32;
    let mut x1i: f32 = 0f32;
    let mut x2r: f32 = 0f32;
    let mut x2i: f32 = 0f32;
    let mut x3r: f32 = 0f32;
    let mut x3i: f32 = 0f32;
    let mut x4r: f32 = 0f32;
    let mut x4i: f32 = 0f32;
    let mut x5r: f32 = 0f32;
    let mut x5i: f32 = 0f32;
    let mut x7r: f32 = 0f32;
    let mut x7i: f32 = 0f32;
    let mut x8r: f32 = 0f32;
    let mut x8i: f32 = 0f32;
    let mut x10r: f32 = 0f32;
    let mut x10i: f32 = 0f32;
    let mut x11r: f32 = 0f32;
    let mut x11i: f32 = 0f32;
    let mut x12r: f32 = 0f32;
    let mut x12i: f32 = 0f32;
    let mut x13r: f32 = 0f32;
    let mut x13i: f32 = 0f32;
    let mut x14r: f32 = 0f32;
    let mut x14i: f32 = 0f32;
    x1r = a[(2i32) as usize];
    x1i = a[(3i32) as usize];
    x2r = a[(4i32) as usize];
    x2i = a[(5i32) as usize];
    x3r = a[(6i32) as usize];
    x3i = a[(7i32) as usize];
    x4r = a[(8i32) as usize];
    x4i = a[(9i32) as usize];
    x5r = a[(10i32) as usize];
    x5i = a[(11i32) as usize];
    x7r = a[(14i32) as usize];
    x7i = a[(15i32) as usize];
    x8r = a[(16i32) as usize];
    x8i = a[(17i32) as usize];
    x10r = a[(20i32) as usize];
    x10i = a[(21i32) as usize];
    x11r = a[(22i32) as usize];
    x11i = a[(23i32) as usize];
    x12r = a[(24i32) as usize];
    x12i = a[(25i32) as usize];
    x13r = a[(26i32) as usize];
    x13i = a[(27i32) as usize];
    x14r = a[(28i32) as usize];
    x14i = a[(29i32) as usize];
    a[(2i32) as usize] = x8r;
    a[(3i32) as usize] = x8i;
    a[(4i32) as usize] = x4r;
    a[(5i32) as usize] = x4i;
    a[(6i32) as usize] = x12r;
    a[(7i32) as usize] = x12i;
    a[(8i32) as usize] = x2r;
    a[(9i32) as usize] = x2i;
    a[(10i32) as usize] = x10r;
    a[(11i32) as usize] = x10i;
    a[(14i32) as usize] = x14r;
    a[(15i32) as usize] = x14i;
    a[(16i32) as usize] = x1r;
    a[(17i32) as usize] = x1i;
    a[(20i32) as usize] = x5r;
    a[(21i32) as usize] = x5i;
    a[(22i32) as usize] = x13r;
    a[(23i32) as usize] = x13i;
    a[(24i32) as usize] = x3r;
    a[(25i32) as usize] = x3i;
    a[(26i32) as usize] = x11r;
    a[(27i32) as usize] = x11i;
    a[(28i32) as usize] = x7r;
    a[(29i32) as usize] = x7i;
}

fn bitrv208(a: &mut [f32]) {
    let mut x1r: f32 = 0f32;
    let mut x1i: f32 = 0f32;
    let mut x3r: f32 = 0f32;
    let mut x3i: f32 = 0f32;
    let mut x4r: f32 = 0f32;
    let mut x4i: f32 = 0f32;
    let mut x6r: f32 = 0f32;
    let mut x6i: f32 = 0f32;
    x1r = a[(2i32) as usize];
    x1i = a[(3i32) as usize];
    x3r = a[(6i32) as usize];
    x3i = a[(7i32) as usize];
    x4r = a[(8i32) as usize];
    x4i = a[(9i32) as usize];
    x6r = a[(12i32) as usize];
    x6i = a[(13i32) as usize];
    a[(2i32) as usize] = x4r;
    a[(3i32) as usize] = x4i;
    a[(6i32) as usize] = x6r;
    a[(7i32) as usize] = x6i;
    a[(8i32) as usize] = x1r;
    a[(9i32) as usize] = x1i;
    a[(12i32) as usize] = x3r;
    a[(13i32) as usize] = x3i;
}

fn cftrec4<M: TwiddleMath>(n: i32, a: &mut [f32], math: &mut M) -> Result<(), M::Error> {
    let mut isplt: i32 = 0i32;
    let mut j: i32 = 0i32;
    let mut k: i32 = 0i32;
    let mut m: i32 = 0i32;
    m = n;
    while (m > 512i32) {
        m = (m >> 2i32);
        cftmdl1(m, &mut a[(n - m) as usize..], math)?;
    }
    cftleaf(m, 1i32, &mut a[(n - m) as usize..], math)?;
    k = 0i32;
    {
        j = (n - m);
        while (j > 0i32) {
            k += 1;
            isplt = cfttree(m, j, k, a, math)?;
            cftleaf(m, isplt, &mut a[(j - m) as usize..], math)?;
            j = (j - m);
        }
    }
    Ok(())
}

fn cfttree<M: TwiddleMath>(n: i32, j: i32, k: i32, a: &mut [f32], math: &mut M) -> Result<i32, M::Error> {
    let mut i: i32 = 0i32;
    let mut isplt: i32 = 0i32;
    let mut m: i32 = 0i32;
    if ((k & 3i32) != 0i32) {
        isplt = (k & 1i32);
        if (isplt != 0i32) {
            cftmdl1(n, &mut a[(j - n) as usize..], math)?;
        } else {
            cftmdl2(n, &mut a[(j - n) as usize..], math)?;
        }
    } else {
        m = n;
        {
            i = k;
            while ((i & 3i32) == 0i32) {
                m = (m << 2i32);
                i = (i >> 2i32);
            }
        }
        isplt = (i & 1i32);
        if (isplt != 0i32) {
            while (m > 128i32) {
                cftmdl1(m, &mut a[(j - m) as usize..], math)?;
                m = (m >> 2i32);
            }
        } else {
            while (m > 128i32) {
                cftmdl2(m, &mut a[(j - m) as usize..], math)?;
                m = (m >> 2i32);
            }
        }
    }
    return Ok(isplt);
}

fn cftleaf<M: TwiddleMath>(n: i32, isplt: i32, a: &mut [f32], math: &mut M) -> Result<(), M::Error> {
    if (n == 512i32) {
        cftmdl1(128i32, a, math)?;
        cftf161(a);
        cftf162(&mut a[(32i32) as usize..]);
        cftf161(&mut a[(64i32) as usize..]);
        cftf161(&mut a[(96i32) as usize..]);
        cftmdl2(128i32, &mut a[(128i32) as usize..], math)?;
        cftf161(&mut a[(128i32) as usize..]);
        cftf162(&mut a[(160i32) as usize..]);
        cftf161(&mut a[(192i32) as usize..]);
        cftf162(&mut a[(224i32) as usize..]);
        cftmdl1(128i32, &mut a[(256i32) as usize..], math)?;
        cftf161(&mut a[(256i32) as usize..]);
        cftf162(&mut a[(288i32) as usize..]);
        cftf161(&mut a[(320i32) as usize..]);
        cftf161(&mut a[(352i32) as usize..]);
        if (isplt != 0i32) {
            cftmdl1(128i32, &mut a[(384i32) as usize..], math)?;
            cftf161(&mut a[(480i32) as usize..]);
        } else {
            cftmdl2(128i32, &mut a[(384i32) as usize..], math)?;
            cftf162(&mut a[(480i32) as usize..]);
        }
        cftf161(&mut a[(384i32) as usize..]);
        cftf162(&mut a[(416i32) as usize..]);
        cftf161(&mut a[(448i32) as usize..]);
    } else {
        cftmdl1(64i32, a, math)?;
        cftf081(a);
        cftf082(&mut a[(16i32) as usize..]);
        cftf081(&mut a[(32i32) as usize..]);
        cftf081(&mut a[(48i32) as usize..]);
        cftmdl2(64i32, &mut a[(64i32) as usize..], math)?;
        cftf081(&mut a[(64i32) as usize..]);
        cftf082(&mut a[(80i32) as usize..]);
        cftf081(&mut a[(96i32) as usize..]);
        cftf082(&mut a[(112i32) as usize..]);
        cftmdl1(64i32, &mut a[(128i32) as usize..], math)?;
        cftf081(&mut a[(128i32) as usize..]);
        cftf082(&mut a[(144i32) as usize..]);
        cftf081(&mut a[(160i32) as usize..]);
        cftf081(&mut a[(176i32) as usize..]);
        if (isplt != 0i32) {
            cftmdl1(64i32, &mut a[(192i32) as usize..], math)?;
            cftf081(&mut a[(240i32) as usize..]);
        } else {
            cftmdl2(64i32, &mut a[(192i32) as usize..], math)?;
            cftf082(&mut a[(240i32) as usize..]);
        }
        cftf081(&mut a[(192i32) as usize..]);
        cftf082(&mut a[(208i32) as usize..]);
        cftf081(&mut a[(224i32) as usize..]);
    }
    Ok(())
}

fn cftmdl1<M: TwiddleMath>(n: i32, a: &mut [f32], math: &mut M) -> Result<(), M::Error> {
    let mut i: i32 = 0i32;
    let mut i0: i32 = 0i32;
    let mut j: i32 = 0i32;
    let mut j0: i32 = 0i32;
    let mut j1: i32 = 0i32;
    let mut j2: i32 = 0i32;
    let mut j3: i32 = 0i32;
    let mut m: i32 = 0i32;
    let mut mh: i32 = 0i32;
    let mut ew: f32 = 0f32;
    let mut w1r: f32 = 0f32;
    let mut w1i: f32 = 0f32;
    let mut wk1r: f32 = 0f32;
    let mut wk1i: f32 = 0f32;
    let mut wk3r: f32 = 0f32;
    let mut wk3i: f32 = 0f32;
    let mut wd1r: f32 = 0f32;
    let mut wd1i: f32 = 0f32;
    let mut wd3r: f32 = 0f32;
    let mut wd3i: f32 = 0f32;
    let mut ss1: f32 = 0f32;
    let mut ss3: f32 = 0f32;
    let mut x0r: f32 = 0f32;
    let mut x0i: f32 = 0f32;
    let mut x1r: f32 = 0f32;
    let mut x1i: f32 = 0f32;
    let mut x2r: f32 = 0f32;
    let mut x2i: f32 = 0f32;
    let mut x3r: f32 = 0f32;
    let mut x3i: f32 = 0f32;
    mh = (n >> 3i32);
    m = (2i32 * mh);
    j1 = m;
    j2 = (j1 + m);
    j3 = (j2 + m);
    x0r = (a[(0i32) as usize] + a[(j2) as usize]);
    x0i = (a[(1i32) as usize] + a[(j2 + 1i32) as usize]);
    x1r = (a[(0i32) as usize] - a[(j2) as usize]);
    x1i = (a[(1i32) as usize] - a[(j2 + 1i32) as usize]);
    x2r = (a[(j1) as usize] + a[(j3) as usize]);
    x2i = (a[(j1 + 1i32) as usize] + a[(j3 + 1i32) as usize]);
    x3r = (a[(j1) as usize] - a[(j3) as usize]);
    x3i = (a[(j1 + 1i32) as usize] - a[(j3 + 1i32) as usize]);
    a[(0i32) as usize] = (x0r + x2r);
    a[(1i32) as usize] = (x0i + x2i);
    a[(j1) as usize] = (x0r - x2r);
    a[(j1 + 1i32) as usize] = (x0i - x2i);
    a[(j2) as usize] = (x1r - x3i);
    a[(j2 + 1i32) as usize] = (x1i + x3r);
    a[(j3) as usize] = (x1r + x3i);
    a[(j3 + 1i32) as usize] = (x1i - x3r);
    wd1r = (1i32 as f32);
    wd1i = (0i32 as f32);
    wd3r = (1i32 as f32);
    wd3i = (0i32 as f32);
    ew = ((1.570796326794896619231321691639751442098584699687f64 / (m as f64)) as f32);
    w1r = twiddle(math, TwiddleSite::CftMdl1, false, TwiddleOperation::Cosine57ddb0, 2_f32 * ew, TwiddleScale::One)?;
    w1i = twiddle(math, TwiddleSite::CftMdl1, false, TwiddleOperation::Sine586960, 2_f32 * ew, TwiddleScale::One)?;
    wk1r = w1r;
    wk1i = w1i;
    ss1 = ((2i32 as f32) * w1i);
    wk3i = (((2i32 as f32) * ss1) * wk1r);
    wk3r = (wk1r - (wk3i * wk1i));
    wk3i = (wk1i - (wk3i * wk1r));
    ss3 = ((2i32 as f32) * wk3i);
    i = 0i32;
    {
        loop {
            i0 = (i + (4i32 * 32i32));
            if (i0 > (mh - 4i32)) {
                i0 = (mh - 4i32);
            }
            {
                j = (i + 2i32);
                while (j < i0) {
                    wd1r = (wd1r - (ss1 * wk1i));
                    wd1i = (wd1i + (ss1 * wk1r));
                    wd3r = (wd3r - (ss3 * wk3i));
                    wd3i = (wd3i + (ss3 * wk3r));
                    j1 = (j + m);
                    j2 = (j1 + m);
                    j3 = (j2 + m);
                    x0r = (a[(j) as usize] + a[(j2) as usize]);
                    x0i = (a[(j + 1i32) as usize] + a[(j2 + 1i32) as usize]);
                    x1r = (a[(j) as usize] - a[(j2) as usize]);
                    x1i = (a[(j + 1i32) as usize] - a[(j2 + 1i32) as usize]);
                    x2r = (a[(j1) as usize] + a[(j3) as usize]);
                    x2i = (a[(j1 + 1i32) as usize] + a[(j3 + 1i32) as usize]);
                    x3r = (a[(j1) as usize] - a[(j3) as usize]);
                    x3i = (a[(j1 + 1i32) as usize] - a[(j3 + 1i32) as usize]);
                    a[(j) as usize] = (x0r + x2r);
                    a[(j + 1i32) as usize] = (x0i + x2i);
                    a[(j1) as usize] = (x0r - x2r);
                    a[(j1 + 1i32) as usize] = (x0i - x2i);
                    x0r = (x1r - x3i);
                    x0i = (x1i + x3r);
                    a[(j2) as usize] = ((wk1r * x0r) - (wk1i * x0i));
                    a[(j2 + 1i32) as usize] = ((wk1r * x0i) + (wk1i * x0r));
                    x0r = (x1r + x3i);
                    x0i = (x1i - x3r);
                    a[(j3) as usize] = ((wk3r * x0r) + (wk3i * x0i));
                    a[(j3 + 1i32) as usize] = ((wk3r * x0i) - (wk3i * x0r));
                    x0r = (a[(j + 2i32) as usize] + a[(j2 + 2i32) as usize]);
                    x0i = (a[(j + 3i32) as usize] + a[(j2 + 3i32) as usize]);
                    x1r = (a[(j + 2i32) as usize] - a[(j2 + 2i32) as usize]);
                    x1i = (a[(j + 3i32) as usize] - a[(j2 + 3i32) as usize]);
                    x2r = (a[(j1 + 2i32) as usize] + a[(j3 + 2i32) as usize]);
                    x2i = (a[(j1 + 3i32) as usize] + a[(j3 + 3i32) as usize]);
                    x3r = (a[(j1 + 2i32) as usize] - a[(j3 + 2i32) as usize]);
                    x3i = (a[(j1 + 3i32) as usize] - a[(j3 + 3i32) as usize]);
                    a[(j + 2i32) as usize] = (x0r + x2r);
                    a[(j + 3i32) as usize] = (x0i + x2i);
                    a[(j1 + 2i32) as usize] = (x0r - x2r);
                    a[(j1 + 3i32) as usize] = (x0i - x2i);
                    x0r = (x1r - x3i);
                    x0i = (x1i + x3r);
                    a[(j2 + 2i32) as usize] = ((wd1r * x0r) - (wd1i * x0i));
                    a[(j2 + 3i32) as usize] = ((wd1r * x0i) + (wd1i * x0r));
                    x0r = (x1r + x3i);
                    x0i = (x1i - x3r);
                    a[(j3 + 2i32) as usize] = ((wd3r * x0r) + (wd3i * x0i));
                    a[(j3 + 3i32) as usize] = ((wd3r * x0i) - (wd3i * x0r));
                    j0 = (m - j);
                    j1 = (j0 + m);
                    j2 = (j1 + m);
                    j3 = (j2 + m);
                    x0r = (a[(j0) as usize] + a[(j2) as usize]);
                    x0i = (a[(j0 + 1i32) as usize] + a[(j2 + 1i32) as usize]);
                    x1r = (a[(j0) as usize] - a[(j2) as usize]);
                    x1i = (a[(j0 + 1i32) as usize] - a[(j2 + 1i32) as usize]);
                    x2r = (a[(j1) as usize] + a[(j3) as usize]);
                    x2i = (a[(j1 + 1i32) as usize] + a[(j3 + 1i32) as usize]);
                    x3r = (a[(j1) as usize] - a[(j3) as usize]);
                    x3i = (a[(j1 + 1i32) as usize] - a[(j3 + 1i32) as usize]);
                    a[(j0) as usize] = (x0r + x2r);
                    a[(j0 + 1i32) as usize] = (x0i + x2i);
                    a[(j1) as usize] = (x0r - x2r);
                    a[(j1 + 1i32) as usize] = (x0i - x2i);
                    x0r = (x1r - x3i);
                    x0i = (x1i + x3r);
                    a[(j2) as usize] = ((wk1i * x0r) - (wk1r * x0i));
                    a[(j2 + 1i32) as usize] = ((wk1i * x0i) + (wk1r * x0r));
                    x0r = (x1r + x3i);
                    x0i = (x1i - x3r);
                    a[(j3) as usize] = ((wk3i * x0r) + (wk3r * x0i));
                    a[(j3 + 1i32) as usize] = ((wk3i * x0i) - (wk3r * x0r));
                    x0r = (a[(j0 - 2i32) as usize] + a[(j2 - 2i32) as usize]);
                    x0i = (a[(j0 - 1i32) as usize] + a[(j2 - 1i32) as usize]);
                    x1r = (a[(j0 - 2i32) as usize] - a[(j2 - 2i32) as usize]);
                    x1i = (a[(j0 - 1i32) as usize] - a[(j2 - 1i32) as usize]);
                    x2r = (a[(j1 - 2i32) as usize] + a[(j3 - 2i32) as usize]);
                    x2i = (a[(j1 - 1i32) as usize] + a[(j3 - 1i32) as usize]);
                    x3r = (a[(j1 - 2i32) as usize] - a[(j3 - 2i32) as usize]);
                    x3i = (a[(j1 - 1i32) as usize] - a[(j3 - 1i32) as usize]);
                    a[(j0 - 2i32) as usize] = (x0r + x2r);
                    a[(j0 - 1i32) as usize] = (x0i + x2i);
                    a[(j1 - 2i32) as usize] = (x0r - x2r);
                    a[(j1 - 1i32) as usize] = (x0i - x2i);
                    x0r = (x1r - x3i);
                    x0i = (x1i + x3r);
                    a[(j2 - 2i32) as usize] = ((wd1i * x0r) - (wd1r * x0i));
                    a[(j2 - 1i32) as usize] = ((wd1i * x0i) + (wd1r * x0r));
                    x0r = (x1r + x3i);
                    x0i = (x1i - x3r);
                    a[(j3 - 2i32) as usize] = ((wd3i * x0r) + (wd3r * x0i));
                    a[(j3 - 1i32) as usize] = ((wd3i * x0i) - (wd3r * x0r));
                    wk1r = (wk1r - (ss1 * wd1i));
                    wk1i = (wk1i + (ss1 * wd1r));
                    wk3r = (wk3r - (ss3 * wd3i));
                    wk3i = (wk3i + (ss3 * wd3r));
                    j = (j + 4i32);
                }
            }
            if (i0 == (mh - 4i32)) {
                break;
            }
            wd1r = twiddle(math, TwiddleSite::CftMdl1, true, TwiddleOperation::Cosine57ddb0, ew * (i0 as f32), TwiddleScale::One)?;
            wd1i = twiddle(math, TwiddleSite::CftMdl1, true, TwiddleOperation::Sine586960, ew * (i0 as f32), TwiddleScale::One)?;
            wd3i = (((4i32 as f32) * wd1i) * wd1r);
            wd3r = (wd1r - (wd3i * wd1i));
            wd3i = (wd1i - (wd3i * wd1r));
            wk1r = ((w1r * wd1r) - (w1i * wd1i));
            wk1i = ((w1r * wd1i) + (w1i * wd1r));
            wk3i = (((4i32 as f32) * wk1i) * wk1r);
            wk3r = (wk1r - (wk3i * wk1i));
            wk3i = (wk1i - (wk3i * wk1r));
            i = i0;
        }
    }
    wd1r = (0.707106781186547524400844362104849039284835937688f64 as f32);
    j0 = mh;
    j1 = (j0 + m);
    j2 = (j1 + m);
    j3 = (j2 + m);
    x0r = (a[(j0 - 2i32) as usize] + a[(j2 - 2i32) as usize]);
    x0i = (a[(j0 - 1i32) as usize] + a[(j2 - 1i32) as usize]);
    x1r = (a[(j0 - 2i32) as usize] - a[(j2 - 2i32) as usize]);
    x1i = (a[(j0 - 1i32) as usize] - a[(j2 - 1i32) as usize]);
    x2r = (a[(j1 - 2i32) as usize] + a[(j3 - 2i32) as usize]);
    x2i = (a[(j1 - 1i32) as usize] + a[(j3 - 1i32) as usize]);
    x3r = (a[(j1 - 2i32) as usize] - a[(j3 - 2i32) as usize]);
    x3i = (a[(j1 - 1i32) as usize] - a[(j3 - 1i32) as usize]);
    a[(j0 - 2i32) as usize] = (x0r + x2r);
    a[(j0 - 1i32) as usize] = (x0i + x2i);
    a[(j1 - 2i32) as usize] = (x0r - x2r);
    a[(j1 - 1i32) as usize] = (x0i - x2i);
    x0r = (x1r - x3i);
    x0i = (x1i + x3r);
    a[(j2 - 2i32) as usize] = ((wk1r * x0r) - (wk1i * x0i));
    a[(j2 - 1i32) as usize] = ((wk1r * x0i) + (wk1i * x0r));
    x0r = (x1r + x3i);
    x0i = (x1i - x3r);
    a[(j3 - 2i32) as usize] = ((wk3r * x0r) + (wk3i * x0i));
    a[(j3 - 1i32) as usize] = ((wk3r * x0i) - (wk3i * x0r));
    x0r = (a[(j0) as usize] + a[(j2) as usize]);
    x0i = (a[(j0 + 1i32) as usize] + a[(j2 + 1i32) as usize]);
    x1r = (a[(j0) as usize] - a[(j2) as usize]);
    x1i = (a[(j0 + 1i32) as usize] - a[(j2 + 1i32) as usize]);
    x2r = (a[(j1) as usize] + a[(j3) as usize]);
    x2i = (a[(j1 + 1i32) as usize] + a[(j3 + 1i32) as usize]);
    x3r = (a[(j1) as usize] - a[(j3) as usize]);
    x3i = (a[(j1 + 1i32) as usize] - a[(j3 + 1i32) as usize]);
    a[(j0) as usize] = (x0r + x2r);
    a[(j0 + 1i32) as usize] = (x0i + x2i);
    a[(j1) as usize] = (x0r - x2r);
    a[(j1 + 1i32) as usize] = (x0i - x2i);
    x0r = (x1r - x3i);
    x0i = (x1i + x3r);
    a[(j2) as usize] = (wd1r * (x0r - x0i));
    a[(j2 + 1i32) as usize] = (wd1r * (x0i + x0r));
    x0r = (x1r + x3i);
    x0i = (x1i - x3r);
    a[(j3) as usize] = ((-wd1r) * (x0r + x0i));
    a[(j3 + 1i32) as usize] = ((-wd1r) * (x0i - x0r));
    x0r = (a[(j0 + 2i32) as usize] + a[(j2 + 2i32) as usize]);
    x0i = (a[(j0 + 3i32) as usize] + a[(j2 + 3i32) as usize]);
    x1r = (a[(j0 + 2i32) as usize] - a[(j2 + 2i32) as usize]);
    x1i = (a[(j0 + 3i32) as usize] - a[(j2 + 3i32) as usize]);
    x2r = (a[(j1 + 2i32) as usize] + a[(j3 + 2i32) as usize]);
    x2i = (a[(j1 + 3i32) as usize] + a[(j3 + 3i32) as usize]);
    x3r = (a[(j1 + 2i32) as usize] - a[(j3 + 2i32) as usize]);
    x3i = (a[(j1 + 3i32) as usize] - a[(j3 + 3i32) as usize]);
    a[(j0 + 2i32) as usize] = (x0r + x2r);
    a[(j0 + 3i32) as usize] = (x0i + x2i);
    a[(j1 + 2i32) as usize] = (x0r - x2r);
    a[(j1 + 3i32) as usize] = (x0i - x2i);
    x0r = (x1r - x3i);
    x0i = (x1i + x3r);
    a[(j2 + 2i32) as usize] = ((wk1i * x0r) - (wk1r * x0i));
    a[(j2 + 3i32) as usize] = ((wk1i * x0i) + (wk1r * x0r));
    x0r = (x1r + x3i);
    x0i = (x1i - x3r);
    a[(j3 + 2i32) as usize] = ((wk3i * x0r) + (wk3r * x0i));
    a[(j3 + 3i32) as usize] = ((wk3i * x0i) - (wk3r * x0r));
    Ok(())
}

fn cftmdl2<M: TwiddleMath>(n: i32, a: &mut [f32], math: &mut M) -> Result<(), M::Error> {
    let mut i: i32 = 0i32;
    let mut i0: i32 = 0i32;
    let mut j: i32 = 0i32;
    let mut j0: i32 = 0i32;
    let mut j1: i32 = 0i32;
    let mut j2: i32 = 0i32;
    let mut j3: i32 = 0i32;
    let mut m: i32 = 0i32;
    let mut mh: i32 = 0i32;
    let mut ew: f32 = 0f32;
    let mut w1r: f32 = 0f32;
    let mut w1i: f32 = 0f32;
    let mut wn4r: f32 = 0f32;
    let mut wk1r: f32 = 0f32;
    let mut wk1i: f32 = 0f32;
    let mut wk3r: f32 = 0f32;
    let mut wk3i: f32 = 0f32;
    let mut wl1r: f32 = 0f32;
    let mut wl1i: f32 = 0f32;
    let mut wl3r: f32 = 0f32;
    let mut wl3i: f32 = 0f32;
    let mut wd1r: f32 = 0f32;
    let mut wd1i: f32 = 0f32;
    let mut wd3r: f32 = 0f32;
    let mut wd3i: f32 = 0f32;
    let mut we1r: f32 = 0f32;
    let mut we1i: f32 = 0f32;
    let mut we3r: f32 = 0f32;
    let mut we3i: f32 = 0f32;
    let mut ss1: f32 = 0f32;
    let mut ss3: f32 = 0f32;
    let mut x0r: f32 = 0f32;
    let mut x0i: f32 = 0f32;
    let mut x1r: f32 = 0f32;
    let mut x1i: f32 = 0f32;
    let mut x2r: f32 = 0f32;
    let mut x2i: f32 = 0f32;
    let mut x3r: f32 = 0f32;
    let mut x3i: f32 = 0f32;
    let mut y0r: f32 = 0f32;
    let mut y0i: f32 = 0f32;
    let mut y2r: f32 = 0f32;
    let mut y2i: f32 = 0f32;
    mh = (n >> 3i32);
    m = (2i32 * mh);
    wn4r = (0.707106781186547524400844362104849039284835937688f64 as f32);
    j1 = m;
    j2 = (j1 + m);
    j3 = (j2 + m);
    x0r = (a[(0i32) as usize] - a[(j2 + 1i32) as usize]);
    x0i = (a[(1i32) as usize] + a[(j2) as usize]);
    x1r = (a[(0i32) as usize] + a[(j2 + 1i32) as usize]);
    x1i = (a[(1i32) as usize] - a[(j2) as usize]);
    x2r = (a[(j1) as usize] - a[(j3 + 1i32) as usize]);
    x2i = (a[(j1 + 1i32) as usize] + a[(j3) as usize]);
    x3r = (a[(j1) as usize] + a[(j3 + 1i32) as usize]);
    x3i = (a[(j1 + 1i32) as usize] - a[(j3) as usize]);
    y0r = (wn4r * (x2r - x2i));
    y0i = (wn4r * (x2i + x2r));
    a[(0i32) as usize] = (x0r + y0r);
    a[(1i32) as usize] = (x0i + y0i);
    a[(j1) as usize] = (x0r - y0r);
    a[(j1 + 1i32) as usize] = (x0i - y0i);
    y0r = (wn4r * (x3r - x3i));
    y0i = (wn4r * (x3i + x3r));
    a[(j2) as usize] = (x1r - y0i);
    a[(j2 + 1i32) as usize] = (x1i + y0r);
    a[(j3) as usize] = (x1r + y0i);
    a[(j3 + 1i32) as usize] = (x1i - y0r);
    wl1r = (1i32 as f32);
    wl1i = (0i32 as f32);
    wl3r = (1i32 as f32);
    wl3i = (0i32 as f32);
    we1r = wn4r;
    we1i = wn4r;
    we3r = (-wn4r);
    we3i = (-wn4r);
    ew = ((1.570796326794896619231321691639751442098584699687f64 / ((2i32 * m) as f64)) as f32);
    w1r = twiddle(math, TwiddleSite::CftMdl2, false, TwiddleOperation::Cosine57ddb0, 2_f32 * ew, TwiddleScale::One)?;
    w1i = twiddle(math, TwiddleSite::CftMdl2, false, TwiddleOperation::Sine586960, 2_f32 * ew, TwiddleScale::One)?;
    wk1r = w1r;
    wk1i = w1i;
    wd1r = (wn4r * (w1r - w1i));
    wd1i = (wn4r * (w1i + w1r));
    ss1 = ((2i32 as f32) * w1i);
    wk3i = (((2i32 as f32) * ss1) * wk1r);
    wk3r = (wk1r - (wk3i * wk1i));
    wk3i = (wk1i - (wk3i * wk1r));
    ss3 = ((2i32 as f32) * wk3i);
    wd3r = ((-wn4r) * (wk3r - wk3i));
    wd3i = ((-wn4r) * (wk3i + wk3r));
    i = 0i32;
    {
        loop {
            i0 = (i + (4i32 * 32i32));
            if (i0 > (mh - 4i32)) {
                i0 = (mh - 4i32);
            }
            {
                j = (i + 2i32);
                while (j < i0) {
                    wl1r = (wl1r - (ss1 * wk1i));
                    wl1i = (wl1i + (ss1 * wk1r));
                    wl3r = (wl3r - (ss3 * wk3i));
                    wl3i = (wl3i + (ss3 * wk3r));
                    we1r = (we1r - (ss1 * wd1i));
                    we1i = (we1i + (ss1 * wd1r));
                    we3r = (we3r - (ss3 * wd3i));
                    we3i = (we3i + (ss3 * wd3r));
                    j1 = (j + m);
                    j2 = (j1 + m);
                    j3 = (j2 + m);
                    x0r = (a[(j) as usize] - a[(j2 + 1i32) as usize]);
                    x0i = (a[(j + 1i32) as usize] + a[(j2) as usize]);
                    x1r = (a[(j) as usize] + a[(j2 + 1i32) as usize]);
                    x1i = (a[(j + 1i32) as usize] - a[(j2) as usize]);
                    x2r = (a[(j1) as usize] - a[(j3 + 1i32) as usize]);
                    x2i = (a[(j1 + 1i32) as usize] + a[(j3) as usize]);
                    x3r = (a[(j1) as usize] + a[(j3 + 1i32) as usize]);
                    x3i = (a[(j1 + 1i32) as usize] - a[(j3) as usize]);
                    y0r = ((wk1r * x0r) - (wk1i * x0i));
                    y0i = ((wk1r * x0i) + (wk1i * x0r));
                    y2r = ((wd1r * x2r) - (wd1i * x2i));
                    y2i = ((wd1r * x2i) + (wd1i * x2r));
                    a[(j) as usize] = (y0r + y2r);
                    a[(j + 1i32) as usize] = (y0i + y2i);
                    a[(j1) as usize] = (y0r - y2r);
                    a[(j1 + 1i32) as usize] = (y0i - y2i);
                    y0r = ((wk3r * x1r) + (wk3i * x1i));
                    y0i = ((wk3r * x1i) - (wk3i * x1r));
                    y2r = ((wd3r * x3r) + (wd3i * x3i));
                    y2i = ((wd3r * x3i) - (wd3i * x3r));
                    a[(j2) as usize] = (y0r + y2r);
                    a[(j2 + 1i32) as usize] = (y0i + y2i);
                    a[(j3) as usize] = (y0r - y2r);
                    a[(j3 + 1i32) as usize] = (y0i - y2i);
                    x0r = (a[(j + 2i32) as usize] - a[(j2 + 3i32) as usize]);
                    x0i = (a[(j + 3i32) as usize] + a[(j2 + 2i32) as usize]);
                    x1r = (a[(j + 2i32) as usize] + a[(j2 + 3i32) as usize]);
                    x1i = (a[(j + 3i32) as usize] - a[(j2 + 2i32) as usize]);
                    x2r = (a[(j1 + 2i32) as usize] - a[(j3 + 3i32) as usize]);
                    x2i = (a[(j1 + 3i32) as usize] + a[(j3 + 2i32) as usize]);
                    x3r = (a[(j1 + 2i32) as usize] + a[(j3 + 3i32) as usize]);
                    x3i = (a[(j1 + 3i32) as usize] - a[(j3 + 2i32) as usize]);
                    y0r = ((wl1r * x0r) - (wl1i * x0i));
                    y0i = ((wl1r * x0i) + (wl1i * x0r));
                    y2r = ((we1r * x2r) - (we1i * x2i));
                    y2i = ((we1r * x2i) + (we1i * x2r));
                    a[(j + 2i32) as usize] = (y0r + y2r);
                    a[(j + 3i32) as usize] = (y0i + y2i);
                    a[(j1 + 2i32) as usize] = (y0r - y2r);
                    a[(j1 + 3i32) as usize] = (y0i - y2i);
                    y0r = ((wl3r * x1r) + (wl3i * x1i));
                    y0i = ((wl3r * x1i) - (wl3i * x1r));
                    y2r = ((we3r * x3r) + (we3i * x3i));
                    y2i = ((we3r * x3i) - (we3i * x3r));
                    a[(j2 + 2i32) as usize] = (y0r + y2r);
                    a[(j2 + 3i32) as usize] = (y0i + y2i);
                    a[(j3 + 2i32) as usize] = (y0r - y2r);
                    a[(j3 + 3i32) as usize] = (y0i - y2i);
                    j0 = (m - j);
                    j1 = (j0 + m);
                    j2 = (j1 + m);
                    j3 = (j2 + m);
                    x0r = (a[(j0) as usize] - a[(j2 + 1i32) as usize]);
                    x0i = (a[(j0 + 1i32) as usize] + a[(j2) as usize]);
                    x1r = (a[(j0) as usize] + a[(j2 + 1i32) as usize]);
                    x1i = (a[(j0 + 1i32) as usize] - a[(j2) as usize]);
                    x2r = (a[(j1) as usize] - a[(j3 + 1i32) as usize]);
                    x2i = (a[(j1 + 1i32) as usize] + a[(j3) as usize]);
                    x3r = (a[(j1) as usize] + a[(j3 + 1i32) as usize]);
                    x3i = (a[(j1 + 1i32) as usize] - a[(j3) as usize]);
                    y0r = ((wd1i * x0r) - (wd1r * x0i));
                    y0i = ((wd1i * x0i) + (wd1r * x0r));
                    y2r = ((wk1i * x2r) - (wk1r * x2i));
                    y2i = ((wk1i * x2i) + (wk1r * x2r));
                    a[(j0) as usize] = (y0r + y2r);
                    a[(j0 + 1i32) as usize] = (y0i + y2i);
                    a[(j1) as usize] = (y0r - y2r);
                    a[(j1 + 1i32) as usize] = (y0i - y2i);
                    y0r = ((wd3i * x1r) + (wd3r * x1i));
                    y0i = ((wd3i * x1i) - (wd3r * x1r));
                    y2r = ((wk3i * x3r) + (wk3r * x3i));
                    y2i = ((wk3i * x3i) - (wk3r * x3r));
                    a[(j2) as usize] = (y0r + y2r);
                    a[(j2 + 1i32) as usize] = (y0i + y2i);
                    a[(j3) as usize] = (y0r - y2r);
                    a[(j3 + 1i32) as usize] = (y0i - y2i);
                    x0r = (a[(j0 - 2i32) as usize] - a[(j2 - 1i32) as usize]);
                    x0i = (a[(j0 - 1i32) as usize] + a[(j2 - 2i32) as usize]);
                    x1r = (a[(j0 - 2i32) as usize] + a[(j2 - 1i32) as usize]);
                    x1i = (a[(j0 - 1i32) as usize] - a[(j2 - 2i32) as usize]);
                    x2r = (a[(j1 - 2i32) as usize] - a[(j3 - 1i32) as usize]);
                    x2i = (a[(j1 - 1i32) as usize] + a[(j3 - 2i32) as usize]);
                    x3r = (a[(j1 - 2i32) as usize] + a[(j3 - 1i32) as usize]);
                    x3i = (a[(j1 - 1i32) as usize] - a[(j3 - 2i32) as usize]);
                    y0r = ((we1i * x0r) - (we1r * x0i));
                    y0i = ((we1i * x0i) + (we1r * x0r));
                    y2r = ((wl1i * x2r) - (wl1r * x2i));
                    y2i = ((wl1i * x2i) + (wl1r * x2r));
                    a[(j0 - 2i32) as usize] = (y0r + y2r);
                    a[(j0 - 1i32) as usize] = (y0i + y2i);
                    a[(j1 - 2i32) as usize] = (y0r - y2r);
                    a[(j1 - 1i32) as usize] = (y0i - y2i);
                    y0r = ((we3i * x1r) + (we3r * x1i));
                    y0i = ((we3i * x1i) - (we3r * x1r));
                    y2r = ((wl3i * x3r) + (wl3r * x3i));
                    y2i = ((wl3i * x3i) - (wl3r * x3r));
                    a[(j2 - 2i32) as usize] = (y0r + y2r);
                    a[(j2 - 1i32) as usize] = (y0i + y2i);
                    a[(j3 - 2i32) as usize] = (y0r - y2r);
                    a[(j3 - 1i32) as usize] = (y0i - y2i);
                    wk1r = (wk1r - (ss1 * wl1i));
                    wk1i = (wk1i + (ss1 * wl1r));
                    wk3r = (wk3r - (ss3 * wl3i));
                    wk3i = (wk3i + (ss3 * wl3r));
                    wd1r = (wd1r - (ss1 * we1i));
                    wd1i = (wd1i + (ss1 * we1r));
                    wd3r = (wd3r - (ss3 * we3i));
                    wd3i = (wd3i + (ss3 * we3r));
                    j = (j + 4i32);
                }
            }
            if (i0 == (mh - 4i32)) {
                break;
            }
            wl1r = twiddle(math, TwiddleSite::CftMdl2, true, TwiddleOperation::Cosine57ddb0, ew * (i0 as f32), TwiddleScale::One)?;
            wl1i = twiddle(math, TwiddleSite::CftMdl2, true, TwiddleOperation::Sine586960, ew * (i0 as f32), TwiddleScale::One)?;
            wl3i = (((4i32 as f32) * wl1i) * wl1r);
            wl3r = (wl1r - (wl3i * wl1i));
            wl3i = (wl1i - (wl3i * wl1r));
            we1r = (wn4r * (wl1r - wl1i));
            we1i = (wn4r * (wl1i + wl1r));
            we3r = ((-wn4r) * (wl3r - wl3i));
            we3i = ((-wn4r) * (wl3i + wl3r));
            wk1r = ((w1r * wl1r) - (w1i * wl1i));
            wk1i = ((w1r * wl1i) + (w1i * wl1r));
            wk3i = (((4i32 as f32) * wk1i) * wk1r);
            wk3r = (wk1r - (wk3i * wk1i));
            wk3i = (wk1i - (wk3i * wk1r));
            wd1r = (wn4r * (wk1r - wk1i));
            wd1i = (wn4r * (wk1i + wk1r));
            wd3r = ((-wn4r) * (wk3r - wk3i));
            wd3i = ((-wn4r) * (wk3i + wk3r));
            i = i0;
        }
    }
    wl1r = (0.923879532511286756128183189396788286822416625863f64 as f32);
    wl1i = (0.382683432365089771728459984030398866761344562485f64 as f32);
    j0 = mh;
    j1 = (j0 + m);
    j2 = (j1 + m);
    j3 = (j2 + m);
    x0r = (a[(j0 - 2i32) as usize] - a[(j2 - 1i32) as usize]);
    x0i = (a[(j0 - 1i32) as usize] + a[(j2 - 2i32) as usize]);
    x1r = (a[(j0 - 2i32) as usize] + a[(j2 - 1i32) as usize]);
    x1i = (a[(j0 - 1i32) as usize] - a[(j2 - 2i32) as usize]);
    x2r = (a[(j1 - 2i32) as usize] - a[(j3 - 1i32) as usize]);
    x2i = (a[(j1 - 1i32) as usize] + a[(j3 - 2i32) as usize]);
    x3r = (a[(j1 - 2i32) as usize] + a[(j3 - 1i32) as usize]);
    x3i = (a[(j1 - 1i32) as usize] - a[(j3 - 2i32) as usize]);
    y0r = ((wk1r * x0r) - (wk1i * x0i));
    y0i = ((wk1r * x0i) + (wk1i * x0r));
    y2r = ((wd1r * x2r) - (wd1i * x2i));
    y2i = ((wd1r * x2i) + (wd1i * x2r));
    a[(j0 - 2i32) as usize] = (y0r + y2r);
    a[(j0 - 1i32) as usize] = (y0i + y2i);
    a[(j1 - 2i32) as usize] = (y0r - y2r);
    a[(j1 - 1i32) as usize] = (y0i - y2i);
    y0r = ((wk3r * x1r) + (wk3i * x1i));
    y0i = ((wk3r * x1i) - (wk3i * x1r));
    y2r = ((wd3r * x3r) + (wd3i * x3i));
    y2i = ((wd3r * x3i) - (wd3i * x3r));
    a[(j2 - 2i32) as usize] = (y0r + y2r);
    a[(j2 - 1i32) as usize] = (y0i + y2i);
    a[(j3 - 2i32) as usize] = (y0r - y2r);
    a[(j3 - 1i32) as usize] = (y0i - y2i);
    x0r = (a[(j0) as usize] - a[(j2 + 1i32) as usize]);
    x0i = (a[(j0 + 1i32) as usize] + a[(j2) as usize]);
    x1r = (a[(j0) as usize] + a[(j2 + 1i32) as usize]);
    x1i = (a[(j0 + 1i32) as usize] - a[(j2) as usize]);
    x2r = (a[(j1) as usize] - a[(j3 + 1i32) as usize]);
    x2i = (a[(j1 + 1i32) as usize] + a[(j3) as usize]);
    x3r = (a[(j1) as usize] + a[(j3 + 1i32) as usize]);
    x3i = (a[(j1 + 1i32) as usize] - a[(j3) as usize]);
    y0r = ((wl1r * x0r) - (wl1i * x0i));
    y0i = ((wl1r * x0i) + (wl1i * x0r));
    y2r = ((wl1i * x2r) - (wl1r * x2i));
    y2i = ((wl1i * x2i) + (wl1r * x2r));
    a[(j0) as usize] = (y0r + y2r);
    a[(j0 + 1i32) as usize] = (y0i + y2i);
    a[(j1) as usize] = (y0r - y2r);
    a[(j1 + 1i32) as usize] = (y0i - y2i);
    y0r = ((wl1i * x1r) - (wl1r * x1i));
    y0i = ((wl1i * x1i) + (wl1r * x1r));
    y2r = ((wl1r * x3r) - (wl1i * x3i));
    y2i = ((wl1r * x3i) + (wl1i * x3r));
    a[(j2) as usize] = (y0r - y2r);
    a[(j2 + 1i32) as usize] = (y0i - y2i);
    a[(j3) as usize] = (y0r + y2r);
    a[(j3 + 1i32) as usize] = (y0i + y2i);
    x0r = (a[(j0 + 2i32) as usize] - a[(j2 + 3i32) as usize]);
    x0i = (a[(j0 + 3i32) as usize] + a[(j2 + 2i32) as usize]);
    x1r = (a[(j0 + 2i32) as usize] + a[(j2 + 3i32) as usize]);
    x1i = (a[(j0 + 3i32) as usize] - a[(j2 + 2i32) as usize]);
    x2r = (a[(j1 + 2i32) as usize] - a[(j3 + 3i32) as usize]);
    x2i = (a[(j1 + 3i32) as usize] + a[(j3 + 2i32) as usize]);
    x3r = (a[(j1 + 2i32) as usize] + a[(j3 + 3i32) as usize]);
    x3i = (a[(j1 + 3i32) as usize] - a[(j3 + 2i32) as usize]);
    y0r = ((wd1i * x0r) - (wd1r * x0i));
    y0i = ((wd1i * x0i) + (wd1r * x0r));
    y2r = ((wk1i * x2r) - (wk1r * x2i));
    y2i = ((wk1i * x2i) + (wk1r * x2r));
    a[(j0 + 2i32) as usize] = (y0r + y2r);
    a[(j0 + 3i32) as usize] = (y0i + y2i);
    a[(j1 + 2i32) as usize] = (y0r - y2r);
    a[(j1 + 3i32) as usize] = (y0i - y2i);
    y0r = ((wd3i * x1r) + (wd3r * x1i));
    y0i = ((wd3i * x1i) - (wd3r * x1r));
    y2r = ((wk3i * x3r) + (wk3r * x3i));
    y2i = ((wk3i * x3i) - (wk3r * x3r));
    a[(j2 + 2i32) as usize] = (y0r + y2r);
    a[(j2 + 3i32) as usize] = (y0i + y2i);
    a[(j3 + 2i32) as usize] = (y0r - y2r);
    a[(j3 + 3i32) as usize] = (y0i - y2i);
    Ok(())
}

fn cftfx41(n: i32, a: &mut [f32]) {
    if (n == 128i32) {
        cftf161(a);
        cftf162(&mut a[(32i32) as usize..]);
        cftf161(&mut a[(64i32) as usize..]);
        cftf161(&mut a[(96i32) as usize..]);
    } else {
        cftf081(a);
        cftf082(&mut a[(16i32) as usize..]);
        cftf081(&mut a[(32i32) as usize..]);
        cftf081(&mut a[(48i32) as usize..]);
    }
}

fn cftf161(a: &mut [f32]) {
    let mut wn4r: f32 = 0f32;
    let mut wk1r: f32 = 0f32;
    let mut wk1i: f32 = 0f32;
    let mut x0r: f32 = 0f32;
    let mut x0i: f32 = 0f32;
    let mut x1r: f32 = 0f32;
    let mut x1i: f32 = 0f32;
    let mut x2r: f32 = 0f32;
    let mut x2i: f32 = 0f32;
    let mut x3r: f32 = 0f32;
    let mut x3i: f32 = 0f32;
    let mut y0r: f32 = 0f32;
    let mut y0i: f32 = 0f32;
    let mut y1r: f32 = 0f32;
    let mut y1i: f32 = 0f32;
    let mut y2r: f32 = 0f32;
    let mut y2i: f32 = 0f32;
    let mut y3r: f32 = 0f32;
    let mut y3i: f32 = 0f32;
    let mut y4r: f32 = 0f32;
    let mut y4i: f32 = 0f32;
    let mut y5r: f32 = 0f32;
    let mut y5i: f32 = 0f32;
    let mut y6r: f32 = 0f32;
    let mut y6i: f32 = 0f32;
    let mut y7r: f32 = 0f32;
    let mut y7i: f32 = 0f32;
    let mut y8r: f32 = 0f32;
    let mut y8i: f32 = 0f32;
    let mut y9r: f32 = 0f32;
    let mut y9i: f32 = 0f32;
    let mut y10r: f32 = 0f32;
    let mut y10i: f32 = 0f32;
    let mut y11r: f32 = 0f32;
    let mut y11i: f32 = 0f32;
    let mut y12r: f32 = 0f32;
    let mut y12i: f32 = 0f32;
    let mut y13r: f32 = 0f32;
    let mut y13i: f32 = 0f32;
    let mut y14r: f32 = 0f32;
    let mut y14i: f32 = 0f32;
    let mut y15r: f32 = 0f32;
    let mut y15i: f32 = 0f32;
    wn4r = (0.707106781186547524400844362104849039284835937688f64 as f32);
    wk1r = (0.923879532511286756128183189396788286822416625863f64 as f32);
    wk1i = (0.382683432365089771728459984030398866761344562485f64 as f32);
    x0r = (a[(0i32) as usize] + a[(16i32) as usize]);
    x0i = (a[(1i32) as usize] + a[(17i32) as usize]);
    x1r = (a[(0i32) as usize] - a[(16i32) as usize]);
    x1i = (a[(1i32) as usize] - a[(17i32) as usize]);
    x2r = (a[(8i32) as usize] + a[(24i32) as usize]);
    x2i = (a[(9i32) as usize] + a[(25i32) as usize]);
    x3r = (a[(8i32) as usize] - a[(24i32) as usize]);
    x3i = (a[(9i32) as usize] - a[(25i32) as usize]);
    y0r = (x0r + x2r);
    y0i = (x0i + x2i);
    y4r = (x0r - x2r);
    y4i = (x0i - x2i);
    y8r = (x1r - x3i);
    y8i = (x1i + x3r);
    y12r = (x1r + x3i);
    y12i = (x1i - x3r);
    x0r = (a[(2i32) as usize] + a[(18i32) as usize]);
    x0i = (a[(3i32) as usize] + a[(19i32) as usize]);
    x1r = (a[(2i32) as usize] - a[(18i32) as usize]);
    x1i = (a[(3i32) as usize] - a[(19i32) as usize]);
    x2r = (a[(10i32) as usize] + a[(26i32) as usize]);
    x2i = (a[(11i32) as usize] + a[(27i32) as usize]);
    x3r = (a[(10i32) as usize] - a[(26i32) as usize]);
    x3i = (a[(11i32) as usize] - a[(27i32) as usize]);
    y1r = (x0r + x2r);
    y1i = (x0i + x2i);
    y5r = (x0r - x2r);
    y5i = (x0i - x2i);
    x0r = (x1r - x3i);
    x0i = (x1i + x3r);
    y9r = ((wk1r * x0r) - (wk1i * x0i));
    y9i = ((wk1r * x0i) + (wk1i * x0r));
    x0r = (x1r + x3i);
    x0i = (x1i - x3r);
    y13r = ((wk1i * x0r) - (wk1r * x0i));
    y13i = ((wk1i * x0i) + (wk1r * x0r));
    x0r = (a[(4i32) as usize] + a[(20i32) as usize]);
    x0i = (a[(5i32) as usize] + a[(21i32) as usize]);
    x1r = (a[(4i32) as usize] - a[(20i32) as usize]);
    x1i = (a[(5i32) as usize] - a[(21i32) as usize]);
    x2r = (a[(12i32) as usize] + a[(28i32) as usize]);
    x2i = (a[(13i32) as usize] + a[(29i32) as usize]);
    x3r = (a[(12i32) as usize] - a[(28i32) as usize]);
    x3i = (a[(13i32) as usize] - a[(29i32) as usize]);
    y2r = (x0r + x2r);
    y2i = (x0i + x2i);
    y6r = (x0r - x2r);
    y6i = (x0i - x2i);
    x0r = (x1r - x3i);
    x0i = (x1i + x3r);
    y10r = (wn4r * (x0r - x0i));
    y10i = (wn4r * (x0i + x0r));
    x0r = (x1r + x3i);
    x0i = (x1i - x3r);
    y14r = (wn4r * (x0r + x0i));
    y14i = (wn4r * (x0i - x0r));
    x0r = (a[(6i32) as usize] + a[(22i32) as usize]);
    x0i = (a[(7i32) as usize] + a[(23i32) as usize]);
    x1r = (a[(6i32) as usize] - a[(22i32) as usize]);
    x1i = (a[(7i32) as usize] - a[(23i32) as usize]);
    x2r = (a[(14i32) as usize] + a[(30i32) as usize]);
    x2i = (a[(15i32) as usize] + a[(31i32) as usize]);
    x3r = (a[(14i32) as usize] - a[(30i32) as usize]);
    x3i = (a[(15i32) as usize] - a[(31i32) as usize]);
    y3r = (x0r + x2r);
    y3i = (x0i + x2i);
    y7r = (x0r - x2r);
    y7i = (x0i - x2i);
    x0r = (x1r - x3i);
    x0i = (x1i + x3r);
    y11r = ((wk1i * x0r) - (wk1r * x0i));
    y11i = ((wk1i * x0i) + (wk1r * x0r));
    x0r = (x1r + x3i);
    x0i = (x1i - x3r);
    y15r = ((wk1r * x0r) - (wk1i * x0i));
    y15i = ((wk1r * x0i) + (wk1i * x0r));
    x0r = (y12r - y14r);
    x0i = (y12i - y14i);
    x1r = (y12r + y14r);
    x1i = (y12i + y14i);
    x2r = (y13r - y15r);
    x2i = (y13i - y15i);
    x3r = (y13r + y15r);
    x3i = (y13i + y15i);
    a[(24i32) as usize] = (x0r + x2r);
    a[(25i32) as usize] = (x0i + x2i);
    a[(26i32) as usize] = (x0r - x2r);
    a[(27i32) as usize] = (x0i - x2i);
    a[(28i32) as usize] = (x1r - x3i);
    a[(29i32) as usize] = (x1i + x3r);
    a[(30i32) as usize] = (x1r + x3i);
    a[(31i32) as usize] = (x1i - x3r);
    x0r = (y8r + y10r);
    x0i = (y8i + y10i);
    x1r = (y8r - y10r);
    x1i = (y8i - y10i);
    x2r = (y9r + y11r);
    x2i = (y9i + y11i);
    x3r = (y9r - y11r);
    x3i = (y9i - y11i);
    a[(16i32) as usize] = (x0r + x2r);
    a[(17i32) as usize] = (x0i + x2i);
    a[(18i32) as usize] = (x0r - x2r);
    a[(19i32) as usize] = (x0i - x2i);
    a[(20i32) as usize] = (x1r - x3i);
    a[(21i32) as usize] = (x1i + x3r);
    a[(22i32) as usize] = (x1r + x3i);
    a[(23i32) as usize] = (x1i - x3r);
    x0r = (y5r - y7i);
    x0i = (y5i + y7r);
    x2r = (wn4r * (x0r - x0i));
    x2i = (wn4r * (x0i + x0r));
    x0r = (y5r + y7i);
    x0i = (y5i - y7r);
    x3r = (wn4r * (x0r - x0i));
    x3i = (wn4r * (x0i + x0r));
    x0r = (y4r - y6i);
    x0i = (y4i + y6r);
    x1r = (y4r + y6i);
    x1i = (y4i - y6r);
    a[(8i32) as usize] = (x0r + x2r);
    a[(9i32) as usize] = (x0i + x2i);
    a[(10i32) as usize] = (x0r - x2r);
    a[(11i32) as usize] = (x0i - x2i);
    a[(12i32) as usize] = (x1r - x3i);
    a[(13i32) as usize] = (x1i + x3r);
    a[(14i32) as usize] = (x1r + x3i);
    a[(15i32) as usize] = (x1i - x3r);
    x0r = (y0r + y2r);
    x0i = (y0i + y2i);
    x1r = (y0r - y2r);
    x1i = (y0i - y2i);
    x2r = (y1r + y3r);
    x2i = (y1i + y3i);
    x3r = (y1r - y3r);
    x3i = (y1i - y3i);
    a[(0i32) as usize] = (x0r + x2r);
    a[(1i32) as usize] = (x0i + x2i);
    a[(2i32) as usize] = (x0r - x2r);
    a[(3i32) as usize] = (x0i - x2i);
    a[(4i32) as usize] = (x1r - x3i);
    a[(5i32) as usize] = (x1i + x3r);
    a[(6i32) as usize] = (x1r + x3i);
    a[(7i32) as usize] = (x1i - x3r);
}

fn cftf162(a: &mut [f32]) {
    let mut wn4r: f32 = 0f32;
    let mut wk1r: f32 = 0f32;
    let mut wk1i: f32 = 0f32;
    let mut wk2r: f32 = 0f32;
    let mut wk2i: f32 = 0f32;
    let mut wk3r: f32 = 0f32;
    let mut wk3i: f32 = 0f32;
    let mut x0r: f32 = 0f32;
    let mut x0i: f32 = 0f32;
    let mut x1r: f32 = 0f32;
    let mut x1i: f32 = 0f32;
    let mut x2r: f32 = 0f32;
    let mut x2i: f32 = 0f32;
    let mut y0r: f32 = 0f32;
    let mut y0i: f32 = 0f32;
    let mut y1r: f32 = 0f32;
    let mut y1i: f32 = 0f32;
    let mut y2r: f32 = 0f32;
    let mut y2i: f32 = 0f32;
    let mut y3r: f32 = 0f32;
    let mut y3i: f32 = 0f32;
    let mut y4r: f32 = 0f32;
    let mut y4i: f32 = 0f32;
    let mut y5r: f32 = 0f32;
    let mut y5i: f32 = 0f32;
    let mut y6r: f32 = 0f32;
    let mut y6i: f32 = 0f32;
    let mut y7r: f32 = 0f32;
    let mut y7i: f32 = 0f32;
    let mut y8r: f32 = 0f32;
    let mut y8i: f32 = 0f32;
    let mut y9r: f32 = 0f32;
    let mut y9i: f32 = 0f32;
    let mut y10r: f32 = 0f32;
    let mut y10i: f32 = 0f32;
    let mut y11r: f32 = 0f32;
    let mut y11i: f32 = 0f32;
    let mut y12r: f32 = 0f32;
    let mut y12i: f32 = 0f32;
    let mut y13r: f32 = 0f32;
    let mut y13i: f32 = 0f32;
    let mut y14r: f32 = 0f32;
    let mut y14i: f32 = 0f32;
    let mut y15r: f32 = 0f32;
    let mut y15i: f32 = 0f32;
    wn4r = (0.707106781186547524400844362104849039284835937688f64 as f32);
    wk1r = (0.980785280403230449126182236134239036973933730893f64 as f32);
    wk1i = (0.195090322016128267848284868477022240927691617751f64 as f32);
    wk2r = (0.923879532511286756128183189396788286822416625863f64 as f32);
    wk2i = (0.382683432365089771728459984030398866761344562485f64 as f32);
    wk3r = (0.831469612302545237078788377617905756738560811987f64 as f32);
    wk3i = (0.555570233019602224742830813948532874374937190754f64 as f32);
    x1r = (a[(0i32) as usize] - a[(17i32) as usize]);
    x1i = (a[(1i32) as usize] + a[(16i32) as usize]);
    x0r = (a[(8i32) as usize] - a[(25i32) as usize]);
    x0i = (a[(9i32) as usize] + a[(24i32) as usize]);
    x2r = (wn4r * (x0r - x0i));
    x2i = (wn4r * (x0i + x0r));
    y0r = (x1r + x2r);
    y0i = (x1i + x2i);
    y4r = (x1r - x2r);
    y4i = (x1i - x2i);
    x1r = (a[(0i32) as usize] + a[(17i32) as usize]);
    x1i = (a[(1i32) as usize] - a[(16i32) as usize]);
    x0r = (a[(8i32) as usize] + a[(25i32) as usize]);
    x0i = (a[(9i32) as usize] - a[(24i32) as usize]);
    x2r = (wn4r * (x0r - x0i));
    x2i = (wn4r * (x0i + x0r));
    y8r = (x1r - x2i);
    y8i = (x1i + x2r);
    y12r = (x1r + x2i);
    y12i = (x1i - x2r);
    x0r = (a[(2i32) as usize] - a[(19i32) as usize]);
    x0i = (a[(3i32) as usize] + a[(18i32) as usize]);
    x1r = ((wk1r * x0r) - (wk1i * x0i));
    x1i = ((wk1r * x0i) + (wk1i * x0r));
    x0r = (a[(10i32) as usize] - a[(27i32) as usize]);
    x0i = (a[(11i32) as usize] + a[(26i32) as usize]);
    x2r = ((wk3i * x0r) - (wk3r * x0i));
    x2i = ((wk3i * x0i) + (wk3r * x0r));
    y1r = (x1r + x2r);
    y1i = (x1i + x2i);
    y5r = (x1r - x2r);
    y5i = (x1i - x2i);
    x0r = (a[(2i32) as usize] + a[(19i32) as usize]);
    x0i = (a[(3i32) as usize] - a[(18i32) as usize]);
    x1r = ((wk3r * x0r) - (wk3i * x0i));
    x1i = ((wk3r * x0i) + (wk3i * x0r));
    x0r = (a[(10i32) as usize] + a[(27i32) as usize]);
    x0i = (a[(11i32) as usize] - a[(26i32) as usize]);
    x2r = ((wk1r * x0r) + (wk1i * x0i));
    x2i = ((wk1r * x0i) - (wk1i * x0r));
    y9r = (x1r - x2r);
    y9i = (x1i - x2i);
    y13r = (x1r + x2r);
    y13i = (x1i + x2i);
    x0r = (a[(4i32) as usize] - a[(21i32) as usize]);
    x0i = (a[(5i32) as usize] + a[(20i32) as usize]);
    x1r = ((wk2r * x0r) - (wk2i * x0i));
    x1i = ((wk2r * x0i) + (wk2i * x0r));
    x0r = (a[(12i32) as usize] - a[(29i32) as usize]);
    x0i = (a[(13i32) as usize] + a[(28i32) as usize]);
    x2r = ((wk2i * x0r) - (wk2r * x0i));
    x2i = ((wk2i * x0i) + (wk2r * x0r));
    y2r = (x1r + x2r);
    y2i = (x1i + x2i);
    y6r = (x1r - x2r);
    y6i = (x1i - x2i);
    x0r = (a[(4i32) as usize] + a[(21i32) as usize]);
    x0i = (a[(5i32) as usize] - a[(20i32) as usize]);
    x1r = ((wk2i * x0r) - (wk2r * x0i));
    x1i = ((wk2i * x0i) + (wk2r * x0r));
    x0r = (a[(12i32) as usize] + a[(29i32) as usize]);
    x0i = (a[(13i32) as usize] - a[(28i32) as usize]);
    x2r = ((wk2r * x0r) - (wk2i * x0i));
    x2i = ((wk2r * x0i) + (wk2i * x0r));
    y10r = (x1r - x2r);
    y10i = (x1i - x2i);
    y14r = (x1r + x2r);
    y14i = (x1i + x2i);
    x0r = (a[(6i32) as usize] - a[(23i32) as usize]);
    x0i = (a[(7i32) as usize] + a[(22i32) as usize]);
    x1r = ((wk3r * x0r) - (wk3i * x0i));
    x1i = ((wk3r * x0i) + (wk3i * x0r));
    x0r = (a[(14i32) as usize] - a[(31i32) as usize]);
    x0i = (a[(15i32) as usize] + a[(30i32) as usize]);
    x2r = ((wk1i * x0r) - (wk1r * x0i));
    x2i = ((wk1i * x0i) + (wk1r * x0r));
    y3r = (x1r + x2r);
    y3i = (x1i + x2i);
    y7r = (x1r - x2r);
    y7i = (x1i - x2i);
    x0r = (a[(6i32) as usize] + a[(23i32) as usize]);
    x0i = (a[(7i32) as usize] - a[(22i32) as usize]);
    x1r = ((wk1i * x0r) + (wk1r * x0i));
    x1i = ((wk1i * x0i) - (wk1r * x0r));
    x0r = (a[(14i32) as usize] + a[(31i32) as usize]);
    x0i = (a[(15i32) as usize] - a[(30i32) as usize]);
    x2r = ((wk3i * x0r) - (wk3r * x0i));
    x2i = ((wk3i * x0i) + (wk3r * x0r));
    y11r = (x1r + x2r);
    y11i = (x1i + x2i);
    y15r = (x1r - x2r);
    y15i = (x1i - x2i);
    x1r = (y0r + y2r);
    x1i = (y0i + y2i);
    x2r = (y1r + y3r);
    x2i = (y1i + y3i);
    a[(0i32) as usize] = (x1r + x2r);
    a[(1i32) as usize] = (x1i + x2i);
    a[(2i32) as usize] = (x1r - x2r);
    a[(3i32) as usize] = (x1i - x2i);
    x1r = (y0r - y2r);
    x1i = (y0i - y2i);
    x2r = (y1r - y3r);
    x2i = (y1i - y3i);
    a[(4i32) as usize] = (x1r - x2i);
    a[(5i32) as usize] = (x1i + x2r);
    a[(6i32) as usize] = (x1r + x2i);
    a[(7i32) as usize] = (x1i - x2r);
    x1r = (y4r - y6i);
    x1i = (y4i + y6r);
    x0r = (y5r - y7i);
    x0i = (y5i + y7r);
    x2r = (wn4r * (x0r - x0i));
    x2i = (wn4r * (x0i + x0r));
    a[(8i32) as usize] = (x1r + x2r);
    a[(9i32) as usize] = (x1i + x2i);
    a[(10i32) as usize] = (x1r - x2r);
    a[(11i32) as usize] = (x1i - x2i);
    x1r = (y4r + y6i);
    x1i = (y4i - y6r);
    x0r = (y5r + y7i);
    x0i = (y5i - y7r);
    x2r = (wn4r * (x0r - x0i));
    x2i = (wn4r * (x0i + x0r));
    a[(12i32) as usize] = (x1r - x2i);
    a[(13i32) as usize] = (x1i + x2r);
    a[(14i32) as usize] = (x1r + x2i);
    a[(15i32) as usize] = (x1i - x2r);
    x1r = (y8r + y10r);
    x1i = (y8i + y10i);
    x2r = (y9r - y11r);
    x2i = (y9i - y11i);
    a[(16i32) as usize] = (x1r + x2r);
    a[(17i32) as usize] = (x1i + x2i);
    a[(18i32) as usize] = (x1r - x2r);
    a[(19i32) as usize] = (x1i - x2i);
    x1r = (y8r - y10r);
    x1i = (y8i - y10i);
    x2r = (y9r + y11r);
    x2i = (y9i + y11i);
    a[(20i32) as usize] = (x1r - x2i);
    a[(21i32) as usize] = (x1i + x2r);
    a[(22i32) as usize] = (x1r + x2i);
    a[(23i32) as usize] = (x1i - x2r);
    x1r = (y12r - y14i);
    x1i = (y12i + y14r);
    x0r = (y13r + y15i);
    x0i = (y13i - y15r);
    x2r = (wn4r * (x0r - x0i));
    x2i = (wn4r * (x0i + x0r));
    a[(24i32) as usize] = (x1r + x2r);
    a[(25i32) as usize] = (x1i + x2i);
    a[(26i32) as usize] = (x1r - x2r);
    a[(27i32) as usize] = (x1i - x2i);
    x1r = (y12r + y14i);
    x1i = (y12i - y14r);
    x0r = (y13r - y15i);
    x0i = (y13i + y15r);
    x2r = (wn4r * (x0r - x0i));
    x2i = (wn4r * (x0i + x0r));
    a[(28i32) as usize] = (x1r - x2i);
    a[(29i32) as usize] = (x1i + x2r);
    a[(30i32) as usize] = (x1r + x2i);
    a[(31i32) as usize] = (x1i - x2r);
}

fn cftf081(a: &mut [f32]) {
    let mut wn4r: f32 = 0f32;
    let mut x0r: f32 = 0f32;
    let mut x0i: f32 = 0f32;
    let mut x1r: f32 = 0f32;
    let mut x1i: f32 = 0f32;
    let mut x2r: f32 = 0f32;
    let mut x2i: f32 = 0f32;
    let mut x3r: f32 = 0f32;
    let mut x3i: f32 = 0f32;
    let mut y0r: f32 = 0f32;
    let mut y0i: f32 = 0f32;
    let mut y1r: f32 = 0f32;
    let mut y1i: f32 = 0f32;
    let mut y2r: f32 = 0f32;
    let mut y2i: f32 = 0f32;
    let mut y3r: f32 = 0f32;
    let mut y3i: f32 = 0f32;
    let mut y4r: f32 = 0f32;
    let mut y4i: f32 = 0f32;
    let mut y5r: f32 = 0f32;
    let mut y5i: f32 = 0f32;
    let mut y6r: f32 = 0f32;
    let mut y6i: f32 = 0f32;
    let mut y7r: f32 = 0f32;
    let mut y7i: f32 = 0f32;
    wn4r = (0.707106781186547524400844362104849039284835937688f64 as f32);
    x0r = (a[(0i32) as usize] + a[(8i32) as usize]);
    x0i = (a[(1i32) as usize] + a[(9i32) as usize]);
    x1r = (a[(0i32) as usize] - a[(8i32) as usize]);
    x1i = (a[(1i32) as usize] - a[(9i32) as usize]);
    x2r = (a[(4i32) as usize] + a[(12i32) as usize]);
    x2i = (a[(5i32) as usize] + a[(13i32) as usize]);
    x3r = (a[(4i32) as usize] - a[(12i32) as usize]);
    x3i = (a[(5i32) as usize] - a[(13i32) as usize]);
    y0r = (x0r + x2r);
    y0i = (x0i + x2i);
    y2r = (x0r - x2r);
    y2i = (x0i - x2i);
    y1r = (x1r - x3i);
    y1i = (x1i + x3r);
    y3r = (x1r + x3i);
    y3i = (x1i - x3r);
    x0r = (a[(2i32) as usize] + a[(10i32) as usize]);
    x0i = (a[(3i32) as usize] + a[(11i32) as usize]);
    x1r = (a[(2i32) as usize] - a[(10i32) as usize]);
    x1i = (a[(3i32) as usize] - a[(11i32) as usize]);
    x2r = (a[(6i32) as usize] + a[(14i32) as usize]);
    x2i = (a[(7i32) as usize] + a[(15i32) as usize]);
    x3r = (a[(6i32) as usize] - a[(14i32) as usize]);
    x3i = (a[(7i32) as usize] - a[(15i32) as usize]);
    y4r = (x0r + x2r);
    y4i = (x0i + x2i);
    y6r = (x0r - x2r);
    y6i = (x0i - x2i);
    x0r = (x1r - x3i);
    x0i = (x1i + x3r);
    x2r = (x1r + x3i);
    x2i = (x1i - x3r);
    y5r = (wn4r * (x0r - x0i));
    y5i = (wn4r * (x0r + x0i));
    y7r = (wn4r * (x2r - x2i));
    y7i = (wn4r * (x2r + x2i));
    a[(8i32) as usize] = (y1r + y5r);
    a[(9i32) as usize] = (y1i + y5i);
    a[(10i32) as usize] = (y1r - y5r);
    a[(11i32) as usize] = (y1i - y5i);
    a[(12i32) as usize] = (y3r - y7i);
    a[(13i32) as usize] = (y3i + y7r);
    a[(14i32) as usize] = (y3r + y7i);
    a[(15i32) as usize] = (y3i - y7r);
    a[(0i32) as usize] = (y0r + y4r);
    a[(1i32) as usize] = (y0i + y4i);
    a[(2i32) as usize] = (y0r - y4r);
    a[(3i32) as usize] = (y0i - y4i);
    a[(4i32) as usize] = (y2r - y6i);
    a[(5i32) as usize] = (y2i + y6r);
    a[(6i32) as usize] = (y2r + y6i);
    a[(7i32) as usize] = (y2i - y6r);
}

fn cftf082(a: &mut [f32]) {
    let mut wn4r: f32 = 0f32;
    let mut wk1r: f32 = 0f32;
    let mut wk1i: f32 = 0f32;
    let mut x0r: f32 = 0f32;
    let mut x0i: f32 = 0f32;
    let mut x1r: f32 = 0f32;
    let mut x1i: f32 = 0f32;
    let mut y0r: f32 = 0f32;
    let mut y0i: f32 = 0f32;
    let mut y1r: f32 = 0f32;
    let mut y1i: f32 = 0f32;
    let mut y2r: f32 = 0f32;
    let mut y2i: f32 = 0f32;
    let mut y3r: f32 = 0f32;
    let mut y3i: f32 = 0f32;
    let mut y4r: f32 = 0f32;
    let mut y4i: f32 = 0f32;
    let mut y5r: f32 = 0f32;
    let mut y5i: f32 = 0f32;
    let mut y6r: f32 = 0f32;
    let mut y6i: f32 = 0f32;
    let mut y7r: f32 = 0f32;
    let mut y7i: f32 = 0f32;
    wn4r = (0.707106781186547524400844362104849039284835937688f64 as f32);
    wk1r = (0.923879532511286756128183189396788286822416625863f64 as f32);
    wk1i = (0.382683432365089771728459984030398866761344562485f64 as f32);
    y0r = (a[(0i32) as usize] - a[(9i32) as usize]);
    y0i = (a[(1i32) as usize] + a[(8i32) as usize]);
    y1r = (a[(0i32) as usize] + a[(9i32) as usize]);
    y1i = (a[(1i32) as usize] - a[(8i32) as usize]);
    x0r = (a[(4i32) as usize] - a[(13i32) as usize]);
    x0i = (a[(5i32) as usize] + a[(12i32) as usize]);
    y2r = (wn4r * (x0r - x0i));
    y2i = (wn4r * (x0i + x0r));
    x0r = (a[(4i32) as usize] + a[(13i32) as usize]);
    x0i = (a[(5i32) as usize] - a[(12i32) as usize]);
    y3r = (wn4r * (x0r - x0i));
    y3i = (wn4r * (x0i + x0r));
    x0r = (a[(2i32) as usize] - a[(11i32) as usize]);
    x0i = (a[(3i32) as usize] + a[(10i32) as usize]);
    y4r = ((wk1r * x0r) - (wk1i * x0i));
    y4i = ((wk1r * x0i) + (wk1i * x0r));
    x0r = (a[(2i32) as usize] + a[(11i32) as usize]);
    x0i = (a[(3i32) as usize] - a[(10i32) as usize]);
    y5r = ((wk1i * x0r) - (wk1r * x0i));
    y5i = ((wk1i * x0i) + (wk1r * x0r));
    x0r = (a[(6i32) as usize] - a[(15i32) as usize]);
    x0i = (a[(7i32) as usize] + a[(14i32) as usize]);
    y6r = ((wk1i * x0r) - (wk1r * x0i));
    y6i = ((wk1i * x0i) + (wk1r * x0r));
    x0r = (a[(6i32) as usize] + a[(15i32) as usize]);
    x0i = (a[(7i32) as usize] - a[(14i32) as usize]);
    y7r = ((wk1r * x0r) - (wk1i * x0i));
    y7i = ((wk1r * x0i) + (wk1i * x0r));
    x0r = (y0r + y2r);
    x0i = (y0i + y2i);
    x1r = (y4r + y6r);
    x1i = (y4i + y6i);
    a[(0i32) as usize] = (x0r + x1r);
    a[(1i32) as usize] = (x0i + x1i);
    a[(2i32) as usize] = (x0r - x1r);
    a[(3i32) as usize] = (x0i - x1i);
    x0r = (y0r - y2r);
    x0i = (y0i - y2i);
    x1r = (y4r - y6r);
    x1i = (y4i - y6i);
    a[(4i32) as usize] = (x0r - x1i);
    a[(5i32) as usize] = (x0i + x1r);
    a[(6i32) as usize] = (x0r + x1i);
    a[(7i32) as usize] = (x0i - x1r);
    x0r = (y1r - y3i);
    x0i = (y1i + y3r);
    x1r = (y5r - y7r);
    x1i = (y5i - y7i);
    a[(8i32) as usize] = (x0r + x1r);
    a[(9i32) as usize] = (x0i + x1i);
    a[(10i32) as usize] = (x0r - x1r);
    a[(11i32) as usize] = (x0i - x1i);
    x0r = (y1r + y3i);
    x0i = (y1i - y3r);
    x1r = (y5r + y7r);
    x1i = (y5i + y7i);
    a[(12i32) as usize] = (x0r - x1i);
    a[(13i32) as usize] = (x0i + x1r);
    a[(14i32) as usize] = (x0r + x1i);
    a[(15i32) as usize] = (x0i - x1r);
}

fn cftf040(a: &mut [f32]) {
    let mut x0r: f32 = 0f32;
    let mut x0i: f32 = 0f32;
    let mut x1r: f32 = 0f32;
    let mut x1i: f32 = 0f32;
    let mut x2r: f32 = 0f32;
    let mut x2i: f32 = 0f32;
    let mut x3r: f32 = 0f32;
    let mut x3i: f32 = 0f32;
    x0r = (a[(0i32) as usize] + a[(4i32) as usize]);
    x0i = (a[(1i32) as usize] + a[(5i32) as usize]);
    x1r = (a[(0i32) as usize] - a[(4i32) as usize]);
    x1i = (a[(1i32) as usize] - a[(5i32) as usize]);
    x2r = (a[(2i32) as usize] + a[(6i32) as usize]);
    x2i = (a[(3i32) as usize] + a[(7i32) as usize]);
    x3r = (a[(2i32) as usize] - a[(6i32) as usize]);
    x3i = (a[(3i32) as usize] - a[(7i32) as usize]);
    a[(0i32) as usize] = (x0r + x2r);
    a[(1i32) as usize] = (x0i + x2i);
    a[(2i32) as usize] = (x1r - x3i);
    a[(3i32) as usize] = (x1i + x3r);
    a[(4i32) as usize] = (x0r - x2r);
    a[(5i32) as usize] = (x0i - x2i);
    a[(6i32) as usize] = (x1r + x3i);
    a[(7i32) as usize] = (x1i - x3r);
}

fn cftx020(a: &mut [f32]) {
    let mut x0r: f32 = 0f32;
    let mut x0i: f32 = 0f32;
    x0r = (a[(0i32) as usize] - a[(2i32) as usize]);
    x0i = (a[(1i32) as usize] - a[(3i32) as usize]);
    a[(0i32) as usize] = (a[(0i32) as usize] + a[(2i32) as usize]);
    a[(1i32) as usize] = (a[(1i32) as usize] + a[(3i32) as usize]);
    a[(2i32) as usize] = x0r;
    a[(3i32) as usize] = x0i;
}

fn rftfsub<M: TwiddleMath>(n: i32, a: &mut [f32], math: &mut M) -> Result<(), M::Error> {
    let mut i: i32 = 0i32;
    let mut i0: i32 = 0i32;
    let mut j: i32 = 0i32;
    let mut k: i32 = 0i32;
    let mut ec: f32 = 0f32;
    let mut w1r: f32 = 0f32;
    let mut w1i: f32 = 0f32;
    let mut wkr: f32 = 0f32;
    let mut wki: f32 = 0f32;
    let mut wdr: f32 = 0f32;
    let mut wdi: f32 = 0f32;
    let mut ss: f32 = 0f32;
    let mut xr: f32 = 0f32;
    let mut xi: f32 = 0f32;
    let mut yr: f32 = 0f32;
    let mut yi: f32 = 0f32;
    ec = ((((2i32 as f64) * 1.570796326794896619231321691639751442098584699687f64) / (n as f64))
        as f32);
    wkr = (0i32 as f32);
    wki = (0i32 as f32);
    wdi = twiddle(math, TwiddleSite::RftFsub, false, TwiddleOperation::Cosine57ddb0, ec, TwiddleScale::One)?;
    wdr = twiddle(math, TwiddleSite::RftFsub, false, TwiddleOperation::Sine586960, ec, TwiddleScale::One)?;
    wdi = (wdi * wdr);
    wdr = (wdr * wdr);
    w1r = ((1i32 as f32) - ((2i32 as f32) * wdr));
    w1i = ((2i32 as f32) * wdi);
    ss = ((2i32 as f32) * w1i);
    i = (n >> 1i32);
    {
        loop {
            i0 = (i - (4i32 * 64i32));
            if (i0 < 4i32) {
                i0 = 4i32;
            }
            {
                j = (i - 4i32);
                while (j >= i0) {
                    k = (n - j);
                    xr = (a[(j + 2i32) as usize] - a[(k - 2i32) as usize]);
                    xi = (a[(j + 3i32) as usize] + a[(k - 1i32) as usize]);
                    yr = ((wdr * xr) - (wdi * xi));
                    yi = ((wdr * xi) + (wdi * xr));
                    a[(j + 2i32) as usize] = (a[(j + 2i32) as usize] - yr);
                    a[(j + 3i32) as usize] = (a[(j + 3i32) as usize] - yi);
                    a[(k - 2i32) as usize] = (a[(k - 2i32) as usize] + yr);
                    a[(k - 1i32) as usize] = (a[(k - 1i32) as usize] - yi);
                    wkr = (wkr + (ss * wdi));
                    wki = (((wki as f64) + ((ss as f64) * (0.5f64 - (wdr as f64)))) as f32);
                    xr = (a[(j) as usize] - a[(k) as usize]);
                    xi = (a[(j + 1i32) as usize] + a[(k + 1i32) as usize]);
                    yr = ((wkr * xr) - (wki * xi));
                    yi = ((wkr * xi) + (wki * xr));
                    a[(j) as usize] = (a[(j) as usize] - yr);
                    a[(j + 1i32) as usize] = (a[(j + 1i32) as usize] - yi);
                    a[(k) as usize] = (a[(k) as usize] + yr);
                    a[(k + 1i32) as usize] = (a[(k + 1i32) as usize] - yi);
                    wdr = (wdr + (ss * wki));
                    wdi = (((wdi as f64) + ((ss as f64) * (0.5f64 - (wkr as f64)))) as f32);
                    j = (j - 4i32);
                }
            }
            if (i0 == 4i32) {
                break;
            }
            wkr = twiddle(math, TwiddleSite::RftFsub, true, TwiddleOperation::Sine586960, ec * (i0 as f32), TwiddleScale::Half)?;
            wki = twiddle(math, TwiddleSite::RftFsub, true, TwiddleOperation::Cosine57ddb0, ec * (i0 as f32), TwiddleScale::Half)?;
            let positive = w1i * wki;
            let shifted = positive + 0.5_f32;
            let negative = w1r * wkr;
            wdr = shifted - negative;
            wdi = ((wkr * w1i) + (wki * w1r));
            wkr = ((0.5f64 - (wkr as f64)) as f32);
            i = i0;
        }
    }
    xr = (a[(2i32) as usize] - a[(n - 2i32) as usize]);
    xi = (a[(3i32) as usize] + a[(n - 1i32) as usize]);
    yr = ((wdr * xr) - (wdi * xi));
    yi = ((wdr * xi) + (wdi * xr));
    a[(2i32) as usize] = (a[(2i32) as usize] - yr);
    a[(3i32) as usize] = (a[(3i32) as usize] - yi);
    a[(n - 2i32) as usize] = (a[(n - 2i32) as usize] + yr);
    a[(n - 1i32) as usize] = (a[(n - 1i32) as usize] - yi);
    Ok(())
}

pub fn rdft_inverse(a:&mut[f32])->crate::Result<()> {
    if a.len()!=512 {return Err("inverse source scope is512 samples".into());}
    match rdft_inverse_body(a, &mut HostTwiddle) { Ok(()) => Ok(()), Err(never) => match never {} }
}

pub fn rdft_inverse_supported(a:&mut[f32])->crate::Result<()> {
    match rdft_inverse_supported_with_twiddle(a, &mut HostTwiddle) {
        Ok(()) => Ok(()),
        Err(TwiddleFftError::UnsupportedSize { .. }) => Err("unsupported source inverse RDFT size".into()),
        Err(TwiddleFftError::Math(never)) => match never {},
    }
}
pub fn rdft_inverse_supported_with_twiddle<M: TwiddleMath>(a: &mut [f32], math: &mut M)
    -> Result<(), TwiddleFftError<M::Error>>
{
    if ![512,1024,2048].contains(&a.len()) {
        return Err(TwiddleFftError::UnsupportedSize { length: a.len() });
    }
    rdft_inverse_body(a, math).map_err(TwiddleFftError::Math)
}
fn rdft_inverse_body<M: TwiddleMath>(a:&mut[f32], math: &mut M) -> Result<(), M::Error> {
    let n=a.len() as i32;
    let xi=(a[0]-a[1])*0.5_f32;
    a[1]=xi;
    a[0]-=xi;
    rftbsub(n,a, math)?;
    cftbsub(n,a, math)?;
    Ok(())
}
fn cftbsub<M: TwiddleMath>(n: i32, a: &mut [f32], math: &mut M) -> Result<(), M::Error> {
    if (n > 8i32) {
        if (n > 32i32) {
            cftb1st(n, a, math)?;
            if (n > 512i32) {
                cftrec4(n, a, math)?;
            } else {
                if (n > 128i32) {
                    cftleaf(n, 1i32, a, math)?;
                } else {
                    cftfx41(n, a);
                }
            }
            bitrv2conj(n, a);
        } else {
            if (n == 32i32) {
                cftf161(a);
                bitrv216neg(a);
            } else {
                cftf081(a);
                bitrv208neg(a);
            }
        }
    } else {
        if (n == 8i32) {
            cftb040(a);
        } else {
            if (n == 4i32) {
                cftx020(a);
            }
        }
    }
    Ok(())
}

fn bitrv2conj(n: i32, a: &mut [f32]) {
    let mut j0: i32 = 0i32;
    let mut k0: i32 = 0i32;
    let mut j1: i32 = 0i32;
    let mut k1: i32 = 0i32;
    let mut l: i32 = 0i32;
    let mut m: i32 = 0i32;
    let mut i: i32 = 0i32;
    let mut j: i32 = 0i32;
    let mut k: i32 = 0i32;
    let mut nh: i32 = 0i32;
    let mut xr: f32 = 0f32;
    let mut xi: f32 = 0f32;
    let mut yr: f32 = 0f32;
    let mut yi: f32 = 0f32;
    m = 4i32;
    {
        l = (n >> 2i32);
        while (l > 8i32) {
            m = (m << 1i32);
            l = (l >> 2i32);
        }
    }
    nh = (n >> 1i32);
    if (l == 8i32) {
        j0 = 0i32;
        {
            k0 = 0i32;
            while (k0 < m) {
                k = k0;
                {
                    j = j0;
                    while (j < (j0 + k0)) {
                        xr = a[(j) as usize];
                        xi = (-a[((j + 1i32)) as usize]);
                        yr = a[(k) as usize];
                        yi = (-a[((k + 1i32)) as usize]);
                        a[(j) as usize] = yr;
                        a[((j + 1i32)) as usize] = yi;
                        a[(k) as usize] = xr;
                        a[((k + 1i32)) as usize] = xi;
                        j1 = (j + m);
                        k1 = (k + (2i32 * m));
                        xr = a[(j1) as usize];
                        xi = (-a[((j1 + 1i32)) as usize]);
                        yr = a[(k1) as usize];
                        yi = (-a[((k1 + 1i32)) as usize]);
                        a[(j1) as usize] = yr;
                        a[((j1 + 1i32)) as usize] = yi;
                        a[(k1) as usize] = xr;
                        a[((k1 + 1i32)) as usize] = xi;
                        j1 = (j1 + m);
                        k1 = (k1 - m);
                        xr = a[(j1) as usize];
                        xi = (-a[((j1 + 1i32)) as usize]);
                        yr = a[(k1) as usize];
                        yi = (-a[((k1 + 1i32)) as usize]);
                        a[(j1) as usize] = yr;
                        a[((j1 + 1i32)) as usize] = yi;
                        a[(k1) as usize] = xr;
                        a[((k1 + 1i32)) as usize] = xi;
                        j1 = (j1 + m);
                        k1 = (k1 + (2i32 * m));
                        xr = a[(j1) as usize];
                        xi = (-a[((j1 + 1i32)) as usize]);
                        yr = a[(k1) as usize];
                        yi = (-a[((k1 + 1i32)) as usize]);
                        a[(j1) as usize] = yr;
                        a[((j1 + 1i32)) as usize] = yi;
                        a[(k1) as usize] = xr;
                        a[((k1 + 1i32)) as usize] = xi;
                        j1 = (j1 + nh);
                        k1 = (k1 + 2i32);
                        xr = a[(j1) as usize];
                        xi = (-a[((j1 + 1i32)) as usize]);
                        yr = a[(k1) as usize];
                        yi = (-a[((k1 + 1i32)) as usize]);
                        a[(j1) as usize] = yr;
                        a[((j1 + 1i32)) as usize] = yi;
                        a[(k1) as usize] = xr;
                        a[((k1 + 1i32)) as usize] = xi;
                        j1 = (j1 - m);
                        k1 = (k1 - (2i32 * m));
                        xr = a[(j1) as usize];
                        xi = (-a[((j1 + 1i32)) as usize]);
                        yr = a[(k1) as usize];
                        yi = (-a[((k1 + 1i32)) as usize]);
                        a[(j1) as usize] = yr;
                        a[((j1 + 1i32)) as usize] = yi;
                        a[(k1) as usize] = xr;
                        a[((k1 + 1i32)) as usize] = xi;
                        j1 = (j1 - m);
                        k1 = (k1 + m);
                        xr = a[(j1) as usize];
                        xi = (-a[((j1 + 1i32)) as usize]);
                        yr = a[(k1) as usize];
                        yi = (-a[((k1 + 1i32)) as usize]);
                        a[(j1) as usize] = yr;
                        a[((j1 + 1i32)) as usize] = yi;
                        a[(k1) as usize] = xr;
                        a[((k1 + 1i32)) as usize] = xi;
                        j1 = (j1 - m);
                        k1 = (k1 - (2i32 * m));
                        xr = a[(j1) as usize];
                        xi = (-a[((j1 + 1i32)) as usize]);
                        yr = a[(k1) as usize];
                        yi = (-a[((k1 + 1i32)) as usize]);
                        a[(j1) as usize] = yr;
                        a[((j1 + 1i32)) as usize] = yi;
                        a[(k1) as usize] = xr;
                        a[((k1 + 1i32)) as usize] = xi;
                        j1 = (j1 + 2i32);
                        k1 = (k1 + nh);
                        xr = a[(j1) as usize];
                        xi = (-a[((j1 + 1i32)) as usize]);
                        yr = a[(k1) as usize];
                        yi = (-a[((k1 + 1i32)) as usize]);
                        a[(j1) as usize] = yr;
                        a[((j1 + 1i32)) as usize] = yi;
                        a[(k1) as usize] = xr;
                        a[((k1 + 1i32)) as usize] = xi;
                        j1 = (j1 + m);
                        k1 = (k1 + (2i32 * m));
                        xr = a[(j1) as usize];
                        xi = (-a[((j1 + 1i32)) as usize]);
                        yr = a[(k1) as usize];
                        yi = (-a[((k1 + 1i32)) as usize]);
                        a[(j1) as usize] = yr;
                        a[((j1 + 1i32)) as usize] = yi;
                        a[(k1) as usize] = xr;
                        a[((k1 + 1i32)) as usize] = xi;
                        j1 = (j1 + m);
                        k1 = (k1 - m);
                        xr = a[(j1) as usize];
                        xi = (-a[((j1 + 1i32)) as usize]);
                        yr = a[(k1) as usize];
                        yi = (-a[((k1 + 1i32)) as usize]);
                        a[(j1) as usize] = yr;
                        a[((j1 + 1i32)) as usize] = yi;
                        a[(k1) as usize] = xr;
                        a[((k1 + 1i32)) as usize] = xi;
                        j1 = (j1 + m);
                        k1 = (k1 + (2i32 * m));
                        xr = a[(j1) as usize];
                        xi = (-a[((j1 + 1i32)) as usize]);
                        yr = a[(k1) as usize];
                        yi = (-a[((k1 + 1i32)) as usize]);
                        a[(j1) as usize] = yr;
                        a[((j1 + 1i32)) as usize] = yi;
                        a[(k1) as usize] = xr;
                        a[((k1 + 1i32)) as usize] = xi;
                        j1 = (j1 - nh);
                        k1 = (k1 - 2i32);
                        xr = a[(j1) as usize];
                        xi = (-a[((j1 + 1i32)) as usize]);
                        yr = a[(k1) as usize];
                        yi = (-a[((k1 + 1i32)) as usize]);
                        a[(j1) as usize] = yr;
                        a[((j1 + 1i32)) as usize] = yi;
                        a[(k1) as usize] = xr;
                        a[((k1 + 1i32)) as usize] = xi;
                        j1 = (j1 - m);
                        k1 = (k1 - (2i32 * m));
                        xr = a[(j1) as usize];
                        xi = (-a[((j1 + 1i32)) as usize]);
                        yr = a[(k1) as usize];
                        yi = (-a[((k1 + 1i32)) as usize]);
                        a[(j1) as usize] = yr;
                        a[((j1 + 1i32)) as usize] = yi;
                        a[(k1) as usize] = xr;
                        a[((k1 + 1i32)) as usize] = xi;
                        j1 = (j1 - m);
                        k1 = (k1 + m);
                        xr = a[(j1) as usize];
                        xi = (-a[((j1 + 1i32)) as usize]);
                        yr = a[(k1) as usize];
                        yi = (-a[((k1 + 1i32)) as usize]);
                        a[(j1) as usize] = yr;
                        a[((j1 + 1i32)) as usize] = yi;
                        a[(k1) as usize] = xr;
                        a[((k1 + 1i32)) as usize] = xi;
                        j1 = (j1 - m);
                        k1 = (k1 - (2i32 * m));
                        xr = a[(j1) as usize];
                        xi = (-a[((j1 + 1i32)) as usize]);
                        yr = a[(k1) as usize];
                        yi = (-a[((k1 + 1i32)) as usize]);
                        a[(j1) as usize] = yr;
                        a[((j1 + 1i32)) as usize] = yi;
                        a[(k1) as usize] = xr;
                        a[((k1 + 1i32)) as usize] = xi;
                        {
                            i = (nh >> 1i32);
                            while (i > { k = (k ^ i); k }) {
                                i = (i >> 1i32);
                            }
                        }
                        j = (j + 4i32);
                    }
                }
                k1 = (j0 + k0);
                j1 = (k1 + 2i32);
                k1 = (k1 + nh);
                a[((j1 - 1i32)) as usize] = (-a[((j1 - 1i32)) as usize]);
                xr = a[(j1) as usize];
                xi = (-a[((j1 + 1i32)) as usize]);
                yr = a[(k1) as usize];
                yi = (-a[((k1 + 1i32)) as usize]);
                a[(j1) as usize] = yr;
                a[((j1 + 1i32)) as usize] = yi;
                a[(k1) as usize] = xr;
                a[((k1 + 1i32)) as usize] = xi;
                a[((k1 + 3i32)) as usize] = (-a[((k1 + 3i32)) as usize]);
                j1 = (j1 + m);
                k1 = (k1 + (2i32 * m));
                xr = a[(j1) as usize];
                xi = (-a[((j1 + 1i32)) as usize]);
                yr = a[(k1) as usize];
                yi = (-a[((k1 + 1i32)) as usize]);
                a[(j1) as usize] = yr;
                a[((j1 + 1i32)) as usize] = yi;
                a[(k1) as usize] = xr;
                a[((k1 + 1i32)) as usize] = xi;
                j1 = (j1 + m);
                k1 = (k1 - m);
                xr = a[(j1) as usize];
                xi = (-a[((j1 + 1i32)) as usize]);
                yr = a[(k1) as usize];
                yi = (-a[((k1 + 1i32)) as usize]);
                a[(j1) as usize] = yr;
                a[((j1 + 1i32)) as usize] = yi;
                a[(k1) as usize] = xr;
                a[((k1 + 1i32)) as usize] = xi;
                j1 = (j1 - 2i32);
                k1 = (k1 - nh);
                xr = a[(j1) as usize];
                xi = (-a[((j1 + 1i32)) as usize]);
                yr = a[(k1) as usize];
                yi = (-a[((k1 + 1i32)) as usize]);
                a[(j1) as usize] = yr;
                a[((j1 + 1i32)) as usize] = yi;
                a[(k1) as usize] = xr;
                a[((k1 + 1i32)) as usize] = xi;
                j1 = (j1 + (nh + 2i32));
                k1 = (k1 + (nh + 2i32));
                xr = a[(j1) as usize];
                xi = (-a[((j1 + 1i32)) as usize]);
                yr = a[(k1) as usize];
                yi = (-a[((k1 + 1i32)) as usize]);
                a[(j1) as usize] = yr;
                a[((j1 + 1i32)) as usize] = yi;
                a[(k1) as usize] = xr;
                a[((k1 + 1i32)) as usize] = xi;
                j1 = (j1 - (nh - m));
                k1 = (k1 + ((2i32 * m) - 2i32));
                a[((j1 - 1i32)) as usize] = (-a[((j1 - 1i32)) as usize]);
                xr = a[(j1) as usize];
                xi = (-a[((j1 + 1i32)) as usize]);
                yr = a[(k1) as usize];
                yi = (-a[((k1 + 1i32)) as usize]);
                a[(j1) as usize] = yr;
                a[((j1 + 1i32)) as usize] = yi;
                a[(k1) as usize] = xr;
                a[((k1 + 1i32)) as usize] = xi;
                a[((k1 + 3i32)) as usize] = (-a[((k1 + 3i32)) as usize]);
                {
                    i = (nh >> 1i32);
                    while (i > { j0 = (j0 ^ i); j0 }) {
                        i = (i >> 1i32);
                    }
                }
                k0 = (k0 + 4i32);
            }
        }
    } else {
        j0 = 0i32;
        {
            k0 = 0i32;
            while (k0 < m) {
                k = k0;
                {
                    j = j0;
                    while (j < (j0 + k0)) {
                        xr = a[(j) as usize];
                        xi = (-a[((j + 1i32)) as usize]);
                        yr = a[(k) as usize];
                        yi = (-a[((k + 1i32)) as usize]);
                        a[(j) as usize] = yr;
                        a[((j + 1i32)) as usize] = yi;
                        a[(k) as usize] = xr;
                        a[((k + 1i32)) as usize] = xi;
                        j1 = (j + m);
                        k1 = (k + m);
                        xr = a[(j1) as usize];
                        xi = (-a[((j1 + 1i32)) as usize]);
                        yr = a[(k1) as usize];
                        yi = (-a[((k1 + 1i32)) as usize]);
                        a[(j1) as usize] = yr;
                        a[((j1 + 1i32)) as usize] = yi;
                        a[(k1) as usize] = xr;
                        a[((k1 + 1i32)) as usize] = xi;
                        j1 = (j1 + nh);
                        k1 = (k1 + 2i32);
                        xr = a[(j1) as usize];
                        xi = (-a[((j1 + 1i32)) as usize]);
                        yr = a[(k1) as usize];
                        yi = (-a[((k1 + 1i32)) as usize]);
                        a[(j1) as usize] = yr;
                        a[((j1 + 1i32)) as usize] = yi;
                        a[(k1) as usize] = xr;
                        a[((k1 + 1i32)) as usize] = xi;
                        j1 = (j1 - m);
                        k1 = (k1 - m);
                        xr = a[(j1) as usize];
                        xi = (-a[((j1 + 1i32)) as usize]);
                        yr = a[(k1) as usize];
                        yi = (-a[((k1 + 1i32)) as usize]);
                        a[(j1) as usize] = yr;
                        a[((j1 + 1i32)) as usize] = yi;
                        a[(k1) as usize] = xr;
                        a[((k1 + 1i32)) as usize] = xi;
                        j1 = (j1 + 2i32);
                        k1 = (k1 + nh);
                        xr = a[(j1) as usize];
                        xi = (-a[((j1 + 1i32)) as usize]);
                        yr = a[(k1) as usize];
                        yi = (-a[((k1 + 1i32)) as usize]);
                        a[(j1) as usize] = yr;
                        a[((j1 + 1i32)) as usize] = yi;
                        a[(k1) as usize] = xr;
                        a[((k1 + 1i32)) as usize] = xi;
                        j1 = (j1 + m);
                        k1 = (k1 + m);
                        xr = a[(j1) as usize];
                        xi = (-a[((j1 + 1i32)) as usize]);
                        yr = a[(k1) as usize];
                        yi = (-a[((k1 + 1i32)) as usize]);
                        a[(j1) as usize] = yr;
                        a[((j1 + 1i32)) as usize] = yi;
                        a[(k1) as usize] = xr;
                        a[((k1 + 1i32)) as usize] = xi;
                        j1 = (j1 - nh);
                        k1 = (k1 - 2i32);
                        xr = a[(j1) as usize];
                        xi = (-a[((j1 + 1i32)) as usize]);
                        yr = a[(k1) as usize];
                        yi = (-a[((k1 + 1i32)) as usize]);
                        a[(j1) as usize] = yr;
                        a[((j1 + 1i32)) as usize] = yi;
                        a[(k1) as usize] = xr;
                        a[((k1 + 1i32)) as usize] = xi;
                        j1 = (j1 - m);
                        k1 = (k1 - m);
                        xr = a[(j1) as usize];
                        xi = (-a[((j1 + 1i32)) as usize]);
                        yr = a[(k1) as usize];
                        yi = (-a[((k1 + 1i32)) as usize]);
                        a[(j1) as usize] = yr;
                        a[((j1 + 1i32)) as usize] = yi;
                        a[(k1) as usize] = xr;
                        a[((k1 + 1i32)) as usize] = xi;
                        {
                            i = (nh >> 1i32);
                            while (i > { k = (k ^ i); k }) {
                                i = (i >> 1i32);
                            }
                        }
                        j = (j + 4i32);
                    }
                }
                k1 = (j0 + k0);
                j1 = (k1 + 2i32);
                k1 = (k1 + nh);
                a[((j1 - 1i32)) as usize] = (-a[((j1 - 1i32)) as usize]);
                xr = a[(j1) as usize];
                xi = (-a[((j1 + 1i32)) as usize]);
                yr = a[(k1) as usize];
                yi = (-a[((k1 + 1i32)) as usize]);
                a[(j1) as usize] = yr;
                a[((j1 + 1i32)) as usize] = yi;
                a[(k1) as usize] = xr;
                a[((k1 + 1i32)) as usize] = xi;
                a[((k1 + 3i32)) as usize] = (-a[((k1 + 3i32)) as usize]);
                j1 = (j1 + m);
                k1 = (k1 + m);
                a[((j1 - 1i32)) as usize] = (-a[((j1 - 1i32)) as usize]);
                xr = a[(j1) as usize];
                xi = (-a[((j1 + 1i32)) as usize]);
                yr = a[(k1) as usize];
                yi = (-a[((k1 + 1i32)) as usize]);
                a[(j1) as usize] = yr;
                a[((j1 + 1i32)) as usize] = yi;
                a[(k1) as usize] = xr;
                a[((k1 + 1i32)) as usize] = xi;
                a[((k1 + 3i32)) as usize] = (-a[((k1 + 3i32)) as usize]);
                {
                    i = (nh >> 1i32);
                    while (i > { j0 = (j0 ^ i); j0 }) {
                        i = (i >> 1i32);
                    }
                }
                k0 = (k0 + 4i32);
            }
        }
    }
}

fn bitrv216neg(a: &mut [f32]) {
    let mut x1r: f32 = 0f32;
    let mut x1i: f32 = 0f32;
    let mut x2r: f32 = 0f32;
    let mut x2i: f32 = 0f32;
    let mut x3r: f32 = 0f32;
    let mut x3i: f32 = 0f32;
    let mut x4r: f32 = 0f32;
    let mut x4i: f32 = 0f32;
    let mut x5r: f32 = 0f32;
    let mut x5i: f32 = 0f32;
    let mut x6r: f32 = 0f32;
    let mut x6i: f32 = 0f32;
    let mut x7r: f32 = 0f32;
    let mut x7i: f32 = 0f32;
    let mut x8r: f32 = 0f32;
    let mut x8i: f32 = 0f32;
    let mut x9r: f32 = 0f32;
    let mut x9i: f32 = 0f32;
    let mut x10r: f32 = 0f32;
    let mut x10i: f32 = 0f32;
    let mut x11r: f32 = 0f32;
    let mut x11i: f32 = 0f32;
    let mut x12r: f32 = 0f32;
    let mut x12i: f32 = 0f32;
    let mut x13r: f32 = 0f32;
    let mut x13i: f32 = 0f32;
    let mut x14r: f32 = 0f32;
    let mut x14i: f32 = 0f32;
    let mut x15r: f32 = 0f32;
    let mut x15i: f32 = 0f32;
    x1r = a[(2i32) as usize];
    x1i = a[(3i32) as usize];
    x2r = a[(4i32) as usize];
    x2i = a[(5i32) as usize];
    x3r = a[(6i32) as usize];
    x3i = a[(7i32) as usize];
    x4r = a[(8i32) as usize];
    x4i = a[(9i32) as usize];
    x5r = a[(10i32) as usize];
    x5i = a[(11i32) as usize];
    x6r = a[(12i32) as usize];
    x6i = a[(13i32) as usize];
    x7r = a[(14i32) as usize];
    x7i = a[(15i32) as usize];
    x8r = a[(16i32) as usize];
    x8i = a[(17i32) as usize];
    x9r = a[(18i32) as usize];
    x9i = a[(19i32) as usize];
    x10r = a[(20i32) as usize];
    x10i = a[(21i32) as usize];
    x11r = a[(22i32) as usize];
    x11i = a[(23i32) as usize];
    x12r = a[(24i32) as usize];
    x12i = a[(25i32) as usize];
    x13r = a[(26i32) as usize];
    x13i = a[(27i32) as usize];
    x14r = a[(28i32) as usize];
    x14i = a[(29i32) as usize];
    x15r = a[(30i32) as usize];
    x15i = a[(31i32) as usize];
    a[(2i32) as usize] = x15r;
    a[(3i32) as usize] = x15i;
    a[(4i32) as usize] = x7r;
    a[(5i32) as usize] = x7i;
    a[(6i32) as usize] = x11r;
    a[(7i32) as usize] = x11i;
    a[(8i32) as usize] = x3r;
    a[(9i32) as usize] = x3i;
    a[(10i32) as usize] = x13r;
    a[(11i32) as usize] = x13i;
    a[(12i32) as usize] = x5r;
    a[(13i32) as usize] = x5i;
    a[(14i32) as usize] = x9r;
    a[(15i32) as usize] = x9i;
    a[(16i32) as usize] = x1r;
    a[(17i32) as usize] = x1i;
    a[(18i32) as usize] = x14r;
    a[(19i32) as usize] = x14i;
    a[(20i32) as usize] = x6r;
    a[(21i32) as usize] = x6i;
    a[(22i32) as usize] = x10r;
    a[(23i32) as usize] = x10i;
    a[(24i32) as usize] = x2r;
    a[(25i32) as usize] = x2i;
    a[(26i32) as usize] = x12r;
    a[(27i32) as usize] = x12i;
    a[(28i32) as usize] = x4r;
    a[(29i32) as usize] = x4i;
    a[(30i32) as usize] = x8r;
    a[(31i32) as usize] = x8i;
}

fn bitrv208neg(a: &mut [f32]) {
    let mut x1r: f32 = 0f32;
    let mut x1i: f32 = 0f32;
    let mut x2r: f32 = 0f32;
    let mut x2i: f32 = 0f32;
    let mut x3r: f32 = 0f32;
    let mut x3i: f32 = 0f32;
    let mut x4r: f32 = 0f32;
    let mut x4i: f32 = 0f32;
    let mut x5r: f32 = 0f32;
    let mut x5i: f32 = 0f32;
    let mut x6r: f32 = 0f32;
    let mut x6i: f32 = 0f32;
    let mut x7r: f32 = 0f32;
    let mut x7i: f32 = 0f32;
    x1r = a[(2i32) as usize];
    x1i = a[(3i32) as usize];
    x2r = a[(4i32) as usize];
    x2i = a[(5i32) as usize];
    x3r = a[(6i32) as usize];
    x3i = a[(7i32) as usize];
    x4r = a[(8i32) as usize];
    x4i = a[(9i32) as usize];
    x5r = a[(10i32) as usize];
    x5i = a[(11i32) as usize];
    x6r = a[(12i32) as usize];
    x6i = a[(13i32) as usize];
    x7r = a[(14i32) as usize];
    x7i = a[(15i32) as usize];
    a[(2i32) as usize] = x7r;
    a[(3i32) as usize] = x7i;
    a[(4i32) as usize] = x3r;
    a[(5i32) as usize] = x3i;
    a[(6i32) as usize] = x5r;
    a[(7i32) as usize] = x5i;
    a[(8i32) as usize] = x1r;
    a[(9i32) as usize] = x1i;
    a[(10i32) as usize] = x6r;
    a[(11i32) as usize] = x6i;
    a[(12i32) as usize] = x2r;
    a[(13i32) as usize] = x2i;
    a[(14i32) as usize] = x4r;
    a[(15i32) as usize] = x4i;
}

fn cftb1st<M: TwiddleMath>(n: i32, a: &mut [f32], math: &mut M) -> Result<(), M::Error> {
    let mut i: i32 = 0i32;
    let mut i0: i32 = 0i32;
    let mut j: i32 = 0i32;
    let mut j0: i32 = 0i32;
    let mut j1: i32 = 0i32;
    let mut j2: i32 = 0i32;
    let mut j3: i32 = 0i32;
    let mut m: i32 = 0i32;
    let mut mh: i32 = 0i32;
    let mut ew: f32 = 0f32;
    let mut w1r: f32 = 0f32;
    let mut w1i: f32 = 0f32;
    let mut wk1r: f32 = 0f32;
    let mut wk1i: f32 = 0f32;
    let mut wk3r: f32 = 0f32;
    let mut wk3i: f32 = 0f32;
    let mut wd1r: f32 = 0f32;
    let mut wd1i: f32 = 0f32;
    let mut wd3r: f32 = 0f32;
    let mut wd3i: f32 = 0f32;
    let mut ss1: f32 = 0f32;
    let mut ss3: f32 = 0f32;
    let mut x0r: f32 = 0f32;
    let mut x0i: f32 = 0f32;
    let mut x1r: f32 = 0f32;
    let mut x1i: f32 = 0f32;
    let mut x2r: f32 = 0f32;
    let mut x2i: f32 = 0f32;
    let mut x3r: f32 = 0f32;
    let mut x3i: f32 = 0f32;
    mh = (n >> 3i32);
    m = (2i32 * mh);
    j1 = m;
    j2 = (j1 + m);
    j3 = (j2 + m);
    x0r = (a[(0i32) as usize] + a[(j2) as usize]);
    x0i = ((-a[(1i32) as usize]) - a[((j2 + 1i32)) as usize]);
    x1r = (a[(0i32) as usize] - a[(j2) as usize]);
    x1i = ((-a[(1i32) as usize]) + a[((j2 + 1i32)) as usize]);
    x2r = (a[(j1) as usize] + a[(j3) as usize]);
    x2i = (a[((j1 + 1i32)) as usize] + a[((j3 + 1i32)) as usize]);
    x3r = (a[(j1) as usize] - a[(j3) as usize]);
    x3i = (a[((j1 + 1i32)) as usize] - a[((j3 + 1i32)) as usize]);
    a[(0i32) as usize] = (x0r + x2r);

    let first_imag_sum = a[((j2 + 1i32)) as usize] + a[(1i32) as usize];
    a[(1i32) as usize] = -(x2i + first_imag_sum);
    a[(j1) as usize] = (x0r - x2r);
    a[((j1 + 1i32)) as usize] = x2i - first_imag_sum;
    a[(j2) as usize] = (x1r + x3i);
    a[((j2 + 1i32)) as usize] = (x1i + x3r);
    a[(j3) as usize] = (x1r - x3i);
    a[((j3 + 1i32)) as usize] = (x1i - x3r);
    wd1r = (1i32 as f32);
    wd1i = (0i32 as f32);
    wd3r = (1i32 as f32);
    wd3i = (0i32 as f32);
    ew = ((1.570796326794896619231321691639751442098584699687f64 / (m as f64)) as f32);
    w1r = twiddle(math, TwiddleSite::CftB1st, false, TwiddleOperation::Cosine57ddb0, 2_f32 * ew, TwiddleScale::One)?;
    w1i = twiddle(math, TwiddleSite::CftB1st, false, TwiddleOperation::Sine586960, 2_f32 * ew, TwiddleScale::One)?;
    wk1r = w1r;
    wk1i = w1i;
    ss1 = ((2i32 as f32) * w1i);
    wk3i = (((2i32 as f32) * ss1) * wk1r);
    wk3r = (wk1r - (wk3i * wk1i));
    wk3i = (wk1i - (wk3i * wk1r));
    ss3 = ((2i32 as f32) * wk3i);
    i = 0i32;
    {
        loop {
            i0 = (i + (4i32 * 32i32));
            if (i0 > (mh - 4i32)) {
                i0 = (mh - 4i32);
            }
            {
                j = (i + 2i32);
                while (j < i0) {
                    wd1r = (wd1r - (ss1 * wk1i));
                    wd1i = (wd1i + (ss1 * wk1r));
                    wd3r = (wd3r - (ss3 * wk3i));
                    wd3i = (wd3i + (ss3 * wk3r));
                    j1 = (j + m);
                    j2 = (j1 + m);
                    j3 = (j2 + m);
                    x0r = (a[(j) as usize] + a[(j2) as usize]);
                    x0i = ((-a[((j + 1i32)) as usize]) - a[((j2 + 1i32)) as usize]);
                    x1r = (a[(j) as usize] - a[(j2) as usize]);
                    x1i = ((-a[((j + 1i32)) as usize]) + a[((j2 + 1i32)) as usize]);
                    x2r = (a[(j1) as usize] + a[(j3) as usize]);
                    x2i = (a[((j1 + 1i32)) as usize] + a[((j3 + 1i32)) as usize]);
                    x3r = (a[(j1) as usize] - a[(j3) as usize]);
                    x3i = (a[((j1 + 1i32)) as usize] - a[((j3 + 1i32)) as usize]);
                    a[(j) as usize] = (x0r + x2r);
                    a[((j + 1i32)) as usize] = (x0i - x2i);
                    a[(j1) as usize] = (x0r - x2r);
                    a[((j1 + 1i32)) as usize] = (x0i + x2i);
                    x0r = (x1r + x3i);
                    x0i = (x1i + x3r);
                    a[(j2) as usize] = ((wk1r * x0r) - (wk1i * x0i));
                    a[((j2 + 1i32)) as usize] = ((wk1r * x0i) + (wk1i * x0r));
                    x0r = (x1r - x3i);
                    x0i = (x1i - x3r);
                    a[(j3) as usize] = ((wk3r * x0r) + (wk3i * x0i));
                    a[((j3 + 1i32)) as usize] = ((wk3r * x0i) - (wk3i * x0r));
                    x0r = (a[((j + 2i32)) as usize] + a[((j2 + 2i32)) as usize]);
                    x0i = ((-a[((j + 3i32)) as usize]) - a[((j2 + 3i32)) as usize]);
                    x1r = (a[((j + 2i32)) as usize] - a[((j2 + 2i32)) as usize]);
                    x1i = ((-a[((j + 3i32)) as usize]) + a[((j2 + 3i32)) as usize]);
                    x2r = (a[((j1 + 2i32)) as usize] + a[((j3 + 2i32)) as usize]);
                    x2i = (a[((j1 + 3i32)) as usize] + a[((j3 + 3i32)) as usize]);
                    x3r = (a[((j1 + 2i32)) as usize] - a[((j3 + 2i32)) as usize]);
                    x3i = (a[((j1 + 3i32)) as usize] - a[((j3 + 3i32)) as usize]);
                    a[((j + 2i32)) as usize] = (x0r + x2r);
                    a[((j + 3i32)) as usize] = (x0i - x2i);
                    a[((j1 + 2i32)) as usize] = (x0r - x2r);
                    a[((j1 + 3i32)) as usize] = (x0i + x2i);
                    x0r = (x1r + x3i);
                    x0i = (x1i + x3r);
                    a[((j2 + 2i32)) as usize] = ((wd1r * x0r) - (wd1i * x0i));
                    a[((j2 + 3i32)) as usize] = ((wd1r * x0i) + (wd1i * x0r));
                    x0r = (x1r - x3i);
                    x0i = (x1i - x3r);
                    a[((j3 + 2i32)) as usize] = ((wd3r * x0r) + (wd3i * x0i));
                    a[((j3 + 3i32)) as usize] = ((wd3r * x0i) - (wd3i * x0r));
                    j0 = (m - j);
                    j1 = (j0 + m);
                    j2 = (j1 + m);
                    j3 = (j2 + m);
                    x0r = (a[(j0) as usize] + a[(j2) as usize]);
                    x0i = ((-a[((j0 + 1i32)) as usize]) - a[((j2 + 1i32)) as usize]);
                    x1r = (a[(j0) as usize] - a[(j2) as usize]);
                    x1i = ((-a[((j0 + 1i32)) as usize]) + a[((j2 + 1i32)) as usize]);
                    x2r = (a[(j1) as usize] + a[(j3) as usize]);
                    x2i = (a[((j1 + 1i32)) as usize] + a[((j3 + 1i32)) as usize]);
                    x3r = (a[(j1) as usize] - a[(j3) as usize]);
                    x3i = (a[((j1 + 1i32)) as usize] - a[((j3 + 1i32)) as usize]);
                    a[(j0) as usize] = (x0r + x2r);
                    a[((j0 + 1i32)) as usize] = (x0i - x2i);
                    a[(j1) as usize] = (x0r - x2r);
                    a[((j1 + 1i32)) as usize] = (x0i + x2i);
                    x0r = (x1r + x3i);
                    x0i = (x1i + x3r);
                    a[(j2) as usize] = ((wk1i * x0r) - (wk1r * x0i));
                    a[((j2 + 1i32)) as usize] = ((wk1i * x0i) + (wk1r * x0r));
                    x0r = (x1r - x3i);
                    x0i = (x1i - x3r);
                    a[(j3) as usize] = ((wk3i * x0r) + (wk3r * x0i));
                    a[((j3 + 1i32)) as usize] = ((wk3i * x0i) - (wk3r * x0r));
                    x0r = (a[((j0 - 2i32)) as usize] + a[((j2 - 2i32)) as usize]);
                    x0i = ((-a[((j0 - 1i32)) as usize]) - a[((j2 - 1i32)) as usize]);
                    x1r = (a[((j0 - 2i32)) as usize] - a[((j2 - 2i32)) as usize]);
                    x1i = ((-a[((j0 - 1i32)) as usize]) + a[((j2 - 1i32)) as usize]);
                    x2r = (a[((j1 - 2i32)) as usize] + a[((j3 - 2i32)) as usize]);
                    x2i = (a[((j1 - 1i32)) as usize] + a[((j3 - 1i32)) as usize]);
                    x3r = (a[((j1 - 2i32)) as usize] - a[((j3 - 2i32)) as usize]);
                    x3i = (a[((j1 - 1i32)) as usize] - a[((j3 - 1i32)) as usize]);
                    a[((j0 - 2i32)) as usize] = (x0r + x2r);
                    a[((j0 - 1i32)) as usize] = (x0i - x2i);
                    a[((j1 - 2i32)) as usize] = (x0r - x2r);
                    a[((j1 - 1i32)) as usize] = (x0i + x2i);
                    x0r = (x1r + x3i);
                    x0i = (x1i + x3r);
                    a[((j2 - 2i32)) as usize] = ((wd1i * x0r) - (wd1r * x0i));
                    a[((j2 - 1i32)) as usize] = ((wd1i * x0i) + (wd1r * x0r));
                    x0r = (x1r - x3i);
                    x0i = (x1i - x3r);
                    a[((j3 - 2i32)) as usize] = ((wd3i * x0r) + (wd3r * x0i));
                    a[((j3 - 1i32)) as usize] = ((wd3i * x0i) - (wd3r * x0r));
                    wk1r = (wk1r - (ss1 * wd1i));
                    wk1i = (wk1i + (ss1 * wd1r));
                    wk3r = (wk3r - (ss3 * wd3i));
                    wk3i = (wk3i + (ss3 * wd3r));
                    j = (j + 4i32);
                }
            }
            if (i0 == (mh - 4i32)) {
                break;
            }
            wd1r = twiddle(math, TwiddleSite::CftB1st, true, TwiddleOperation::Cosine57ddb0, ew * (i0 as f32), TwiddleScale::One)?;
            wd1i = twiddle(math, TwiddleSite::CftB1st, true, TwiddleOperation::Sine586960, ew * (i0 as f32), TwiddleScale::One)?;
            wd3i = (((4i32 as f32) * wd1i) * wd1r);
            wd3r = (wd1r - (wd3i * wd1i));
            wd3i = (wd1i - (wd3i * wd1r));
            wk1r = ((w1r * wd1r) - (w1i * wd1i));
            wk1i = ((w1r * wd1i) + (w1i * wd1r));
            wk3i = (((4i32 as f32) * wk1i) * wk1r);
            wk3r = (wk1r - (wk3i * wk1i));
            wk3i = (wk1i - (wk3i * wk1r));
            i = i0;
        }
    }
    wd1r = (0.707106781186547524400844362104849039284835937688f64 as f32);
    j0 = mh;
    j1 = (j0 + m);
    j2 = (j1 + m);
    j3 = (j2 + m);
    x0r = (a[((j0 - 2i32)) as usize] + a[((j2 - 2i32)) as usize]);
    x0i = ((-a[((j0 - 1i32)) as usize]) - a[((j2 - 1i32)) as usize]);
    x1r = (a[((j0 - 2i32)) as usize] - a[((j2 - 2i32)) as usize]);
    x1i = ((-a[((j0 - 1i32)) as usize]) + a[((j2 - 1i32)) as usize]);
    x2r = (a[((j1 - 2i32)) as usize] + a[((j3 - 2i32)) as usize]);
    x2i = (a[((j1 - 1i32)) as usize] + a[((j3 - 1i32)) as usize]);
    x3r = (a[((j1 - 2i32)) as usize] - a[((j3 - 2i32)) as usize]);
    x3i = (a[((j1 - 1i32)) as usize] - a[((j3 - 1i32)) as usize]);
    a[((j0 - 2i32)) as usize] = (x0r + x2r);
    a[((j0 - 1i32)) as usize] = (x0i - x2i);
    a[((j1 - 2i32)) as usize] = (x0r - x2r);
    a[((j1 - 1i32)) as usize] = (x0i + x2i);
    x0r = (x1r + x3i);
    x0i = (x1i + x3r);
    a[((j2 - 2i32)) as usize] = ((wk1r * x0r) - (wk1i * x0i));
    a[((j2 - 1i32)) as usize] = ((wk1r * x0i) + (wk1i * x0r));
    x0r = (x1r - x3i);
    x0i = (x1i - x3r);
    a[((j3 - 2i32)) as usize] = ((wk3r * x0r) + (wk3i * x0i));
    a[((j3 - 1i32)) as usize] = ((wk3r * x0i) - (wk3i * x0r));
    x0r = (a[(j0) as usize] + a[(j2) as usize]);
    x0i = ((-a[((j0 + 1i32)) as usize]) - a[((j2 + 1i32)) as usize]);
    x1r = (a[(j0) as usize] - a[(j2) as usize]);
    x1i = ((-a[((j0 + 1i32)) as usize]) + a[((j2 + 1i32)) as usize]);
    x2r = (a[(j1) as usize] + a[(j3) as usize]);
    x2i = (a[((j1 + 1i32)) as usize] + a[((j3 + 1i32)) as usize]);
    x3r = (a[(j1) as usize] - a[(j3) as usize]);
    x3i = (a[((j1 + 1i32)) as usize] - a[((j3 + 1i32)) as usize]);
    a[(j0) as usize] = (x0r + x2r);
    a[((j0 + 1i32)) as usize] = (x0i - x2i);
    a[(j1) as usize] = (x0r - x2r);
    a[((j1 + 1i32)) as usize] = (x0i + x2i);
    x0r = (x1r + x3i);
    x0i = (x1i + x3r);
    a[(j2) as usize] = (wd1r * (x0r - x0i));
    a[((j2 + 1i32)) as usize] = (wd1r * (x0i + x0r));
    x0r = (x1r - x3i);
    x0i = (x1i - x3r);
    a[(j3) as usize] = ((-wd1r) * (x0r + x0i));
    a[((j3 + 1i32)) as usize] = ((-wd1r) * (x0i - x0r));
    x0r = (a[((j0 + 2i32)) as usize] + a[((j2 + 2i32)) as usize]);
    x0i = ((-a[((j0 + 3i32)) as usize]) - a[((j2 + 3i32)) as usize]);
    x1r = (a[((j0 + 2i32)) as usize] - a[((j2 + 2i32)) as usize]);
    x1i = ((-a[((j0 + 3i32)) as usize]) + a[((j2 + 3i32)) as usize]);
    x2r = (a[((j1 + 2i32)) as usize] + a[((j3 + 2i32)) as usize]);
    x2i = (a[((j1 + 3i32)) as usize] + a[((j3 + 3i32)) as usize]);
    x3r = (a[((j1 + 2i32)) as usize] - a[((j3 + 2i32)) as usize]);
    x3i = (a[((j1 + 3i32)) as usize] - a[((j3 + 3i32)) as usize]);
    a[((j0 + 2i32)) as usize] = (x0r + x2r);
    a[((j0 + 3i32)) as usize] = (x0i - x2i);
    a[((j1 + 2i32)) as usize] = (x0r - x2r);
    a[((j1 + 3i32)) as usize] = (x0i + x2i);
    x0r = (x1r + x3i);
    x0i = (x1i + x3r);
    a[((j2 + 2i32)) as usize] = ((wk1i * x0r) - (wk1r * x0i));
    a[((j2 + 3i32)) as usize] = ((wk1i * x0i) + (wk1r * x0r));
    x0r = (x1r - x3i);
    x0i = (x1i - x3r);
    a[((j3 + 2i32)) as usize] = ((wk3i * x0r) + (wk3r * x0i));
    a[((j3 + 3i32)) as usize] = ((wk3i * x0i) - (wk3r * x0r));
    Ok(())
}

fn cftb040(a: &mut [f32]) {
    let mut x0r: f32 = 0f32;
    let mut x0i: f32 = 0f32;
    let mut x1r: f32 = 0f32;
    let mut x1i: f32 = 0f32;
    let mut x2r: f32 = 0f32;
    let mut x2i: f32 = 0f32;
    let mut x3r: f32 = 0f32;
    let mut x3i: f32 = 0f32;
    x0r = (a[(0i32) as usize] + a[(4i32) as usize]);
    x0i = (a[(1i32) as usize] + a[(5i32) as usize]);
    x1r = (a[(0i32) as usize] - a[(4i32) as usize]);
    x1i = (a[(1i32) as usize] - a[(5i32) as usize]);
    x2r = (a[(2i32) as usize] + a[(6i32) as usize]);
    x2i = (a[(3i32) as usize] + a[(7i32) as usize]);
    x3r = (a[(2i32) as usize] - a[(6i32) as usize]);
    x3i = (a[(3i32) as usize] - a[(7i32) as usize]);
    a[(0i32) as usize] = (x0r + x2r);
    a[(1i32) as usize] = (x0i + x2i);
    a[(2i32) as usize] = (x1r + x3i);
    a[(3i32) as usize] = (x1i - x3r);
    a[(4i32) as usize] = (x0r - x2r);
    a[(5i32) as usize] = (x0i - x2i);
    a[(6i32) as usize] = (x1r - x3i);
    a[(7i32) as usize] = (x1i + x3r);
}

fn rftbsub<M: TwiddleMath>(n: i32, a: &mut [f32], math: &mut M) -> Result<(), M::Error> {
    let mut i: i32 = 0i32;
    let mut i0: i32 = 0i32;
    let mut j: i32 = 0i32;
    let mut k: i32 = 0i32;
    let mut ec: f32 = 0f32;
    let mut w1r: f32 = 0f32;
    let mut w1i: f32 = 0f32;
    let mut wkr: f32 = 0f32;
    let mut wki: f32 = 0f32;
    let mut wdr: f32 = 0f32;
    let mut wdi: f32 = 0f32;
    let mut ss: f32 = 0f32;
    let mut xr: f32 = 0f32;
    let mut xi: f32 = 0f32;
    let mut yr: f32 = 0f32;
    let mut yi: f32 = 0f32;
    ec = ((((2i32 as f64) * 1.570796326794896619231321691639751442098584699687f64) / (n as f64)) as f32);
    wkr = (0i32 as f32);
    wki = (0i32 as f32);
    wdi = twiddle(math, TwiddleSite::RftBsub, false, TwiddleOperation::Cosine57ddb0, ec, TwiddleScale::One)?;
    wdr = twiddle(math, TwiddleSite::RftBsub, false, TwiddleOperation::Sine586960, ec, TwiddleScale::One)?;
    wdi = (wdi * wdr);
    wdr = (wdr * wdr);
    w1r = ((1i32 as f32) - ((2i32 as f32) * wdr));
    w1i = ((2i32 as f32) * wdi);
    ss = ((2i32 as f32) * w1i);
    i = (n >> 1i32);
    {
        loop {
            i0 = (i - (4i32 * 64i32));
            if (i0 < 4i32) {
                i0 = 4i32;
            }
            {
                j = (i - 4i32);
                while (j >= i0) {
                    k = (n - j);
                    xr = (a[((j + 2i32)) as usize] - a[((k - 2i32)) as usize]);
                    xi = (a[((j + 3i32)) as usize] + a[((k - 1i32)) as usize]);
                    yr = ((wdr * xr) + (wdi * xi));
                    yi = ((wdr * xi) - (wdi * xr));
                    a[((j + 2i32)) as usize] = (a[((j + 2i32)) as usize] - yr);
                    a[((j + 3i32)) as usize] = (a[((j + 3i32)) as usize] - yi);
                    a[((k - 2i32)) as usize] = (a[((k - 2i32)) as usize] + yr);
                    a[((k - 1i32)) as usize] = (a[((k - 1i32)) as usize] - yi);
                    wkr = (wkr + (ss * wdi));
                    wki = (((wki as f64) + ((ss as f64) * (0.5f64 - (wdr as f64)))) as f32);
                    xr = (a[(j) as usize] - a[(k) as usize]);
                    xi = (a[((j + 1i32)) as usize] + a[((k + 1i32)) as usize]);
                    yr = ((wkr * xr) + (wki * xi));
                    yi = ((wkr * xi) - (wki * xr));
                    a[(j) as usize] = (a[(j) as usize] - yr);
                    a[((j + 1i32)) as usize] = (a[((j + 1i32)) as usize] - yi);
                    a[(k) as usize] = (a[(k) as usize] + yr);
                    a[((k + 1i32)) as usize] = (a[((k + 1i32)) as usize] - yi);
                    wdr = (wdr + (ss * wki));
                    wdi = (((wdi as f64) + ((ss as f64) * (0.5f64 - (wkr as f64)))) as f32);
                    j = (j - 4i32);
                }
            }
            if (i0 == 4i32) {
                break;
            }
            wkr = twiddle(math, TwiddleSite::RftBsub, true, TwiddleOperation::Sine586960, ec * (i0 as f32), TwiddleScale::Half)?;
            wki = twiddle(math, TwiddleSite::RftBsub, true, TwiddleOperation::Cosine57ddb0, ec * (i0 as f32), TwiddleScale::Half)?;

            let positive = w1i * wki;
            let shifted = positive + 0.5_f32;
            let negative = w1r * wkr;
            wdr = shifted - negative;
            wdi = ((wkr * w1i) + (wki * w1r));
            wkr = ((0.5f64 - (wkr as f64)) as f32);
            i = i0;
        }
    }
    xr = (a[(2i32) as usize] - a[((n - 2i32)) as usize]);
    xi = (a[(3i32) as usize] + a[((n - 1i32)) as usize]);
    yr = ((wdr * xr) + (wdi * xi));
    yi = ((wdr * xi) - (wdi * xr));
    a[(2i32) as usize] = (a[(2i32) as usize] - yr);
    a[(3i32) as usize] = (a[(3i32) as usize] - yi);
    a[((n - 2i32)) as usize] = (a[((n - 2i32)) as usize] + yr);
    a[((n - 1i32)) as usize] = (a[((n - 1i32)) as usize] - yi);
    Ok(())
}
