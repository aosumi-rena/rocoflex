use crate::Result;

#[derive(Clone,Copy,Default)]
struct Complex { re:f32, im:f32 }
impl Complex {
    fn mul(self,other:Self)->Self {
        let re0=self.re*other.re;
        let re1=self.im*other.im;
        let im0=self.re*other.im;
        let im1=self.im*other.re;
        Self {re:re0-re1,im:im0+im1}
    }
    fn div(self,other:Self)->Result<Self> {
        let value=if other.re.abs()>other.im.abs() {
            let ratio=other.im/other.re;
            let denominator=other.re+other.im*ratio;
            Self {re:(self.re+self.im*ratio)/denominator,
                  im:(self.im-self.re*ratio)/denominator}
        } else {
            let ratio=other.re/other.im;
            let denominator=other.im+other.re*ratio;
            Self {re:(self.im+self.re*ratio)/denominator,
                  im:(-self.re+self.im*ratio)/denominator}
        };
        if !value.re.is_finite() || !value.im.is_finite() {
            return Err("Singular source-generator complex response".into());
        }
        Ok(value)
    }
}
fn exponential(real:f32,imaginary:f32)->Complex {

    let scale=f64::from(real).exp();
    Complex {re:(scale*f64::from(imaginary).cos()) as f32,
             im:(scale*f64::from(imaginary).sin()) as f32}
}

pub(crate) fn polar_response(parameters:[f32;5],frequency:f32)->Result<(f32,f32)> {
    let t=0.005_f32*parameters[0];
    let p=0.005_f32*parameters[1];
    let ta=0.005_f32*parameters[2];
    let e=parameters[3];
    let a=parameters[4];
    let omega=f32::from_bits(0x4049_0fdb)/p;
    let angle=omega*t;
    let sine=angle.sin();
    let cosine=angle.cos();
    let exponential_scale=(a*t).exp();
    let normalization=-1.0_f32/(exponential_scale*sine);
    let w=(frequency.max(0.1)*f32::from_bits(0x40c9_0fdb))/0.5;
    let negative_w=-w;
    let aa=a*a;
    let ww=w*w;
    let oo=omega*omega;
    let real_denominator=(aa-ww)-oo;
    let imaginary_denominator=a*negative_w-w*a;
    let opening_factor=Complex {re:normalization,im:0.0}.div(
        Complex {re:real_denominator,im:imaginary_denominator})?;
    let opening_exponential=exponential(a*t,negative_w*t);
    let opening_base=Complex {re:a*sine-omega*cosine,im:negative_w*sine};
    let mut opening_numerator=opening_exponential.mul(opening_base);
    opening_numerator.re+=omega;
    let opening=opening_factor.mul(opening_numerator);

    let ta_e=ta*e;
    let weighted_w=ta_e*w;
    let cap=((1.0_f64-f64::from(ta_e))*f64::from(e)) as f32;
    let return_denominator=Complex {re:e*weighted_w,im:(-weighted_w)*w};
    let return_factor=exponential(0.0,negative_w*t).div(return_denominator)?;
    let return_numerator=if 1.0>cap {
        Complex {re:0.0,im:-weighted_w}
    } else {
        let tail=exponential(0.0,(1.0-t)*negative_w);
        Complex {re:cap-cap*tail.re,im:(-cap*tail.im)-weighted_w}
    };
    let returning=return_factor.mul(return_numerator);
    let real=opening.re+returning.re;
    let imaginary=opening.im+returning.im;
    let imaginary_squared=f64::from(imaginary)*f64::from(imaginary);
    let real_squared=f64::from(real)*f64::from(real);
    let magnitude=(imaginary_squared+real_squared).sqrt() as f32;
    let phase=imaginary.atan2(real);
    if !magnitude.is_finite() || !phase.is_finite() {
        return Err("Nonfinite source-generator polar response".into());
    }
    Ok((magnitude,phase))
}

pub(crate) struct ChirpTransform {
    count:usize,
    complex_count:usize,
    signal:Vec<f32>,
    kernel:Vec<f32>,
    chirp:Vec<Complex>,
}
impl ChirpTransform {
    pub(crate) fn new(count:usize)->Result<Self> {
        if count==0 || count>2048 {return Err("Unsupported source-generator transform extent".into());}
        let complex_count=count.next_power_of_two()*2;
        Ok(Self {count,complex_count,signal:vec![0.0;2*complex_count],
            kernel:vec![0.0;2*complex_count],chirp:vec![Complex::default();count]})
    }
    pub(crate) fn real_output(&mut self,real:&[f32],imaginary:&[f32],rate:f32,
        output:&mut [f32])->Result<()> {
        if real.len()!=self.count || imaginary.len()!=self.count || output.len()!=self.count
            || !rate.is_finite() || real.iter().chain(imaginary).any(|x|!x.is_finite()) {
            return Err("Invalid source-generator chirp input".into());
        }
        self.signal.fill(0.0);
        self.kernel.fill(0.0);
        let gamma=f64::from(rate)*-0.5;
        for j in 0..self.count {
            let jf=j as f64;
            let angle=((jf*jf)*gamma) as f32;
            let widened=f64::from(angle);
            let q=Complex {re:widened.cos() as f32,im:widened.sin() as f32};
            self.chirp[j]=q;
            let input=Complex {re:real[j],im:imaginary[j]}.mul(q);
            self.signal[2*j]=input.re;
            self.signal[2*j+1]=input.im;
            self.kernel[2*j]=q.re;
            self.kernel[2*j+1]=-q.im;
            if j!=0 {
                let mirrored=self.complex_count-j;
                self.kernel[2*mirrored]=q.re;
                self.kernel[2*mirrored+1]=-q.im;
            }
        }
        crate::fft::cdft_in_place(&mut self.signal,false)?;
        crate::fft::cdft_in_place(&mut self.kernel,false)?;
        for (a,b) in self.signal.chunks_exact_mut(2).zip(self.kernel.chunks_exact(2)) {
            let product=Complex {re:a[0],im:a[1]}.mul(Complex {re:b[0],im:b[1]});
            a[0]=product.re;a[1]=product.im;
        }
        crate::fft::cdft_in_place(&mut self.signal,true)?;
        let inverse=1.0/self.complex_count as f32;
        for (j,out) in output.iter_mut().enumerate() {
            let product=Complex {re:self.signal[2*j],im:self.signal[2*j+1]}.mul(self.chirp[j]);
            *out=product.re*inverse;
        }
        if output.iter().any(|value| !value.is_finite()) {
            return Err("Nonfinite source-generator chirp output".into());
        }
        Ok(())
    }
}
