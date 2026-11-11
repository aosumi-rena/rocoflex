#[derive(Debug)]pub enum Error{NonFinite,RayDomain,Size}
#[derive(Default)]pub struct Output{pub ray:[u32;4],pub records:Vec<[u32;4]>,pub math:Vec<[u32;4]>,pub wraps:Vec<[u32;11]>,pub growth:Vec<[u32;4]>,pub capacity:usize}
#[link(name="ucrt")]unsafe extern "C"{fn atan2(y:f64,x:f64)->f64;}
fn angle(out:&mut Output,a:f32,b:f32)->Result<f32,Error>{let v=unsafe{atan2(a as f64,b as f64)}as f32;out.math.push([0,a.to_bits(),b.to_bits(),v.to_bits()]);if v.is_finite(){Ok(v)}else{Err(Error::NonFinite)}}
fn distance(out:&mut Output,a:f32,b:f32)->Result<f32,Error>{let a64=a as f64;let b64=b as f64;let v=((b64*b64)+(a64*a64)).sqrt()as f32;out.math.push([1,a.to_bits(),b.to_bits(),v.to_bits()]);if v.is_finite(){Ok(v)}else{Err(Error::NonFinite)}}
fn append(out:&mut Output,r:[f32;4]){if out.records.len()==out.capacity{let old=out.capacity;out.capacity=(old+old/2).max(out.records.len()+1);out.growth.push([old as u32,out.capacity as u32,out.records.len()as u32,out.records.len()as u32]);}out.records.push(r.map(f32::to_bits))}
fn wrap_vertical(origin:[f32;2],far:[f32;2],a:[f32;2],b:[f32;2])->Result<([f32;2],bool),Error>{
 if far==a{return Ok((far,true))}
 let dx=far[0]-origin[0];let dy=far[1]-origin[1];let ex=b[0]-a[0];let ey=b[1]-a[1];let den=(dx*ey)-(dy*ex);
 if dx!=0.0||!dy.is_finite()||dy<=0.0||!den.is_finite(){return Err(Error::RayDomain)}

 if den.abs()<=f32::MIN_POSITIVE{return Ok(([(far[0]+a[0])*0.5,(far[1]+a[1])*0.5],false))}
 let oyay=origin[1]-a[1];let oxax=origin[0]-a[0];let t=((ex*oyay)-(ey*oxax))/den;let point=[dx*t+origin[0],dy*t+origin[1]];
 let hit=if t<0.0||1.0<t{false}else{let neg=f32::from_bits(oxax.to_bits()^0x80000000);let u=(oyay*dx+dy*neg)/den;0.0<=u&&u<=1.0};
 if point.iter().any(|v|!v.is_finite()){return Err(Error::NonFinite)}Ok((point,hit))
}
pub fn produce_checked(origin:[f32;2],segments:&[[f32;4]],capacity:usize)->Result<Output,Error>{
 if capacity>i32::MAX as usize||segments.len()>i32::MAX as usize/2{return Err(Error::Size)}
 if origin.iter().chain(segments.iter().flatten()).any(|v|!v.is_finite()){return Err(Error::NonFinite)}
 let far=[origin[0]+0.0,origin[1]+100000.0];let mut out=Output{ray:[origin[0].to_bits(),origin[1].to_bits(),far[0].to_bits(),far[1].to_bits()],capacity,..Output::default()};
 for s in segments{let a=[s[0],s[1]];let b=[s[2],s[3]];let aa=angle(&mut out,a[0]-origin[0],origin[1]-a[1])?;let ab=angle(&mut out,b[0]-origin[0],origin[1]-b[1])?;let da=distance(&mut out,origin[0]-a[0],origin[1]-a[1])?;let db=distance(&mut out,origin[0]-b[0],origin[1]-b[1])?;
  let(start,end,ds,de)=if aa>ab{(ab,aa,db,da)}else{(aa,ab,da,db)};
  if end-start>f32::from_bits(0x40490fdb){let(p,hit)=wrap_vertical(origin,far,a,b)?;out.wraps.push([origin[0].to_bits(),origin[1].to_bits(),far[0].to_bits(),far[1].to_bits(),a[0].to_bits(),a[1].to_bits(),b[0].to_bits(),b[1].to_bits(),p[0].to_bits(),p[1].to_bits(),hit as u32]);let d=distance(&mut out,origin[0]-p[0],origin[1]-p[1])?;append(&mut out,[-f32::from_bits(0x40490fdb),start,d,ds]);append(&mut out,[end,f32::from_bits(0x40490fdb),de,d]);}
  else{append(&mut out,[start,end,ds,de]);}
 }Ok(out)
}
