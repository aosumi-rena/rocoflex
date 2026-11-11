#[derive(Debug,PartialEq)]pub enum Error{InvalidRectangle,NonFinite,InvalidZoom,TooManyRecords,Allocation}
#[derive(Clone,Copy)]pub struct Record{pub start:f32,pub end:f32,pub start_distance:f32,pub end_distance:f32}
#[derive(Debug,PartialEq)]pub struct Output{pub planar:Vec<f32>,pub trig:Vec<[u32;3]>,pub stores:Vec<[u32;2]>}
#[link(name="ucrt")]unsafe extern "C"{fn sinf(x:f32)->f32;fn cosf(x:f32)->f32;}
pub fn project_checked(records:&[Record],origin:[f32;2],rect:[i32;4],zoom:f32)->Result<Output,Error>{
 if rect[2]<=0||rect[3]<=0{return Err(Error::InvalidRectangle)}
 if !zoom.is_finite()||zoom<=0.0{return Err(Error::InvalidZoom)}
 if origin.iter().any(|v|!v.is_finite())||records.iter().any(|r|[r.start,r.end,r.start_distance,r.end_distance].iter().any(|v|!v.is_finite())){return Err(Error::NonFinite)}
 if records.len()>u32::MAX as usize/4{return Err(Error::TooManyRecords)}
 let mut output=Output{planar:Vec::new(),trig:Vec::new(),stores:Vec::new()};output.planar.try_reserve_exact(records.len()*4).map_err(|_|Error::Allocation)?;output.planar.resize(records.len()*4,0.);
 let half=f32::from_bits(0x3f000000);let shift=f32::from_bits(0xbfc90fdb);let m=(rect[2].min(rect[3])as f32)*half;let cx=(rect[2]as f32)*half+rect[0]as f32;let cy=(rect[3]as f32)*half+rect[1]as f32;
 for(i,r)in records.iter().enumerate(){
  let ea=r.end+shift;let ec=unsafe{sinf(ea)};output.trig.push([0,ea.to_bits(),ec.to_bits()]);
  let ea2=r.end+shift;let es=unsafe{cosf(ea2)};output.trig.push([1,ea2.to_bits(),es.to_bits()]);
  let sa=r.start+shift;let sc=unsafe{sinf(sa)};output.trig.push([0,sa.to_bits(),sc.to_bits()]);
  let sa2=r.start+shift;let ss=unsafe{cosf(sa2)};output.trig.push([1,sa2.to_bits(),ss.to_bits()]);
  let ey=ec*r.end_distance+origin[1];let ex=es*r.end_distance+origin[0];let sy=sc*r.start_distance+origin[1];let sx=ss*r.start_distance+origin[0];
  let px=((sx*m)*zoom+cx)+(-half);let py=(cy-(sy*m)*zoom)+(-half);let qx=((ex*m)*zoom+cx)+(-half);let qy=(cy-(ey*m)*zoom)+(-half);
  let n=2*records.len();for(index,value)in[(2*i,px),(n+2*i,py),(2*i+1,qx),(n+2*i+1,qy)]{if !value.is_finite(){return Err(Error::NonFinite)}output.planar[index]=value;output.stores.push([index as u32,value.to_bits()])}
 }Ok(output)
}
