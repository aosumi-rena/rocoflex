pub fn quadratic8_source(values:[f64;8])->Result<[f64;3],&'static str> {
    if values.iter().any(|x|!x.is_finite()) {return Err("finite source stencil inputs required");}

    let mut first=values[4]-values[2];
    let mut second=values[1]-values[5];
    second*=16.0;
    first*=61.0;
    first+=second;
    second=values[6]-values[0];
    second*=3.0;
    second+=first;
    let linear=second/76.0;
    let mut a=values[4]+values[2];
    let b=values[6]*10.0;
    a*=106.0;
    a+=b;
    let mut c=values[0]*6.0;
    c+=a;
    let mut d=values[7]*-3.0;
    d+=c;
    let mut e=values[5]+values[1];
    e*=-29.0;
    e+=d;
    let mut f=values[3]*-167.0;
    f+=e;
    let quadratic=f/76.0;
    if !linear.is_finite() || !quadratic.is_finite() {return Err("finite stencil output scope required");}
    Ok([values[3],linear,quadratic])
}

pub struct QuadraticCoefficients {
    pub phase_count:usize,
    pub taps:usize,
    pub row_stride:usize,
    pub coefficients:Vec<f64>,
}

impl QuadraticCoefficients {

    pub fn from_source_normalized_rows(samples:&[f64],phase_count:usize,taps:usize)->Result<Self,&'static str> {
        let needed=(phase_count.checked_add(8).ok_or("source sample-count overflow")?).checked_mul(taps).ok_or("source sample-span overflow")?;
        let row_stride=taps.checked_mul(3).ok_or("source row-stride overflow")?;
        let words=(phase_count.checked_add(1).ok_or("source coefficient-count overflow")?).checked_mul(row_stride).ok_or("source coefficient-span overflow")?;
        if phase_count==0 || taps==0 || taps%2!=0 || words>(1<<20) || samples.len()!=needed {
            return Err("bounded source8-point/even-tap table shape required");
        }
        let mut coefficients=vec![0.0;words];
        for phase in 0..=phase_count {
            for pair in 0..taps/2 {
                let tap=pair*2;
                let a=quadratic8_source(std::array::from_fn(|offset|samples[(phase+offset)*taps+tap]))?;
                let b=quadratic8_source(std::array::from_fn(|offset|samples[(phase+offset)*taps+tap+1]))?;
                let start=phase*row_stride+pair*6;

                coefficients[start..start+6].copy_from_slice(&[a[0],b[0],a[1],b[1],a[2],b[2]]);
            }
        }
        Ok(Self {phase_count,taps,row_stride,coefficients})
    }

    pub fn row(&self,phase:usize)->Result<&[f64],&'static str> {
        if phase>self.phase_count {return Err("source coefficient phase out of bounds");}
        Ok(&self.coefficients[phase*self.row_stride..(phase+1)*self.row_stride])
    }
}
