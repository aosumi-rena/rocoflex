#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum SummaryError { Empty, MissingSourceBlockDepth, InvalidSourceBlockDepth }

pub fn summary(points:&[[f32;2]],source_block_depth:Option<usize>)
    ->Result<([f32;2],[f32;4]),SummaryError> {
    let n=points.len();if n==0{return Err(SummaryError::Empty);}
    let kc=if n<48 {n} else {source_block_depth.ok_or(SummaryError::MissingSourceBlockDepth)?};
    if kc==0 || kc>n {return Err(SummaryError::InvalidSourceBlockDepth);}
    let mut mean=[0.0;2];
    for d in 0..2 {
        let col:Vec<_>=points.iter().map(|p|p[d]).collect();
        mean[d]=super::normalization::aligned_sum(&col,d*n)/n as f32;
    }
    let mut covariance=[0.0;4];
    for c in 0..2 {for r in 0..2 {
        let mut accumulated=0.0_f32;
        if n+4<=19 {

            let products:Vec<_>=points.iter().map(|p|(p[r]-mean[r])*(p[c]-mean[c])).collect();
            accumulated=super::normalization::aligned_sum(&products,0);
        } else {

            for block in points.chunks(kc) {
                let mut partial=0.0_f32;
                for p in block {
                    partial=(p[r]-mean[r])*(p[c]-mean[c])+partial;
                }
                accumulated=partial*1.0+accumulated;
            }
        }
        covariance[c*2+r]=accumulated/n as f32;
    }}
    Ok((mean,covariance))
}

