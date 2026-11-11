use super::cache_blocking::{blocking,CacheBytes,Product,Blocking,Error};
pub use super::geometry::Statistics;
pub fn global_statistics(points:&[[f32;4]],cache:CacheBytes)->Result<(Statistics,Blocking),Error>{
    let n=points.len();if n<2{return Err(Error::InvalidDepth);}
    let plan=blocking(Product::Global4d,n,cache)?;
    if plan.nc!=4 || plan.mc==0 || plan.mc>4 {return Err(Error::SourceDivisionByZero);}
    let normalized=super::normalization::normalization(points);
    let mut stats=Statistics{offset:normalized.offset,scale:normalized.scale,covariance:[0.0;16]};
    for c in 0..4 {for r in 0..4 {
        let products:Vec<f32>=points.iter().map(|p|((stats.offset[r]+p[r])*stats.scale[r])*((stats.offset[c]+p[c])*stats.scale[c])).collect();
        let mut accumulated=0.0_f32;
        if n+8<=19 {
            accumulated=super::normalization::aligned_sum(&products,0);
        }else{
            let block_rows=(4-r/plan.mc*plan.mc).min(plan.mc);
            for chunk in products.chunks(plan.kc) {
                let mut partial;
                if block_rows==4 {

                    let end=chunk.len()/8*8;let(mut even,mut odd)=(0.0_f32,0.0_f32);
                    for i in (0..end).step_by(2){even=chunk[i]+even;odd=chunk[i+1]+odd;}
                    partial=even+odd;for &v in &chunk[end..]{partial+=v;}
                }else{

                    let end=chunk.len()/4*4;let mut a=[0.0_f32;4];
                    for i in (0..end).step_by(4){for lane in 0..4{a[lane]+=chunk[i+lane];}}
                    partial=(a[0]+a[1])+(a[2]+a[3]);
                    for &v in &chunk[end..]{partial+=v;}
                }
                accumulated=partial*1.0+accumulated;
            }
        }
        stats.covariance[c*4+r]=accumulated/n as f32;
    }}
    Ok((stats,plan))
}

pub fn parent_summary(points:&[[f32;2]],cache:CacheBytes)->Result<(([f32;2],[f32;4]),Blocking),Error>{
    let plan=blocking(Product::Parent2d,points.len(),cache)?;
    let result=super::parent_summary::summary(points,Some(plan.kc)).map_err(|_|Error::InvalidDepth)?;
    Ok((result,plan))
}
