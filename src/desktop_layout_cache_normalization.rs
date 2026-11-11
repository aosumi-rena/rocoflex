#[derive(Clone,Debug)]
pub struct Normalization {pub offset:[f32;4],pub scale:[f32;4]}
pub fn aligned_sum(values: &[f32], word_offset: usize) -> f32 {
    let prefix=((4-word_offset%4)%4).min(values.len());
    if values.len()-prefix < 4 {
        return values.iter().copied().reduce(|a,b|a+b).unwrap_or(0.0);
    }
    let packet_end=prefix+(values.len()-prefix)/4*4;
    let mut a=[0.0;4];a.copy_from_slice(&values[prefix..prefix+4]);
    let mut i=prefix+4;
    if packet_end-prefix>=8 {
        let mut b=[0.0;4];b.copy_from_slice(&values[i..i+4]);i+=4;
        while i+8<=packet_end {
            for k in 0..4 {a[k]+=values[i+k];b[k]+=values[i+4+k];}
            i+=8;
        }
        for k in 0..4 {a[k]=b[k]+a[k];}
    }
    if i<packet_end {for k in 0..4 {a[k]+=values[i+k];}}
    let mut sum=(a[1]+a[3])+(a[0]+a[2]);
    for &v in &values[..prefix] {sum+=v;}
    for &v in &values[packet_end..] {sum+=v;}
    sum
}

pub fn normalization(points:&[[f32;4]])->Normalization {
    let n=points.len(); assert!(n>1,"Normalization requires at least two rows");
    let mut offset=[0.0;4];let mut centered=vec![0.0;n*4];
    for d in 0..4 {
        let column:Vec<_>=points.iter().map(|p|p[d]).collect();
        offset[d]=-aligned_sum(&column,d*n)/n as f32;
        for i in 0..n {centered[d*n+i]=offset[d]+points[i][d];}
    }
    let squares:Vec<_>=centered.iter().map(|v|v*v).collect();
    let scale=1.0/(aligned_sum(&squares,0)/n as f32/4.0+f32::from_bits(0x3a83126f)).sqrt();
    for v in &mut centered {*v*=scale;}
    Normalization {offset,scale:[scale;4]}
}
