include!("tables.rs");
#[derive(Debug,PartialEq,Eq)]
pub enum BlurError { EmptyExtent, InvalidLayout, SizeOverflow }
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum FilterAxis { Horizontal, Vertical }
#[derive(Clone,Copy,Debug)]
pub struct FilterProgress { pub axis:FilterAxis, pub next_line:usize, pub effective_radius:usize }

pub struct FilterPartial { pub horizontal:Vec<u8>, pub filtered:Vec<u8>, pub progress:FilterProgress }
pub enum FilterOutput<C> {
 Complete { horizontal:Vec<u8>, filtered:Vec<u8> },
 Superseded { reason:C, partial:FilterPartial },
}
pub fn effective_radius(input:i32)->usize { (input as u32).clamp(2,254) as usize }

pub fn filter_format2(data:&[u8],width:usize,height:usize,row_stride:usize,radius:i32)
 -> Result<(Vec<u8>,Vec<u8>),BlurError> {
 match filter_format2_scheduled(data,width,height,row_stride,radius,
  &mut || None::<std::convert::Infallible>)? {
  FilterOutput::Complete{horizontal,filtered}=>Ok((horizontal,filtered)),
  FilterOutput::Superseded{reason,..}=>match reason {},
 }
}

pub fn filter_format2_scheduled<C>(data:&[u8],width:usize,height:usize,row_stride:usize,radius:i32,
 checkpoint:&mut impl FnMut()->Option<C>)->Result<FilterOutput<C>,BlurError> {
 if width==0 || height==0 { return Err(BlurError::EmptyExtent) }
 let packed=width.checked_mul(4).ok_or(BlurError::SizeOverflow)?;
 let size=row_stride.checked_mul(height).ok_or(BlurError::SizeOverflow)?;
 if row_stride<packed || data.len()!=size { return Err(BlurError::InvalidLayout) }
 let r=effective_radius(radius);let mul=MULTIPLY[r];let shift=SHIFT[r];
 let mut mid=data.to_vec();let mut out=data.to_vec();
 for y in 0..height {
  for c in 0..4 { filter_line(data,&mut mid,y*row_stride+c,4,width,r,mul,shift); }
  if let Some(reason)=checkpoint() {
   return Ok(FilterOutput::Superseded{reason,partial:FilterPartial{horizontal:mid,filtered:out,
    progress:FilterProgress{axis:FilterAxis::Horizontal,next_line:y+1,effective_radius:r}}});
  }
 }
 for x in 0..width {
  for c in 0..4 { filter_line(&mid,&mut out,x*4+c,row_stride,height,r,mul,shift); }
  if let Some(reason)=checkpoint() {
   return Ok(FilterOutput::Superseded{reason,partial:FilterPartial{horizontal:mid,filtered:out,
    progress:FilterProgress{axis:FilterAxis::Vertical,next_line:x+1,effective_radius:r}}});
  }
 }
 Ok(FilterOutput::Complete{horizontal:mid,filtered:out})
}

fn filter_line(source:&[u8],destination:&mut[u8],start:usize,stride:usize,
 length:usize,r:usize,mul:u32,shift:u32) {
 let last=length-1;
 let sample=|position:usize| u32::from(source[start+position.min(last)*stride]);

 let mut sum=0u32;
 for k in 0..2*r+1 {
  let at=k.saturating_sub(r).min(last);
  let weight=r+1-k.abs_diff(r);
  sum+=sample(at)*weight as u32;
 }

 let mut lower=(r+1) as u32*sample(0);
 let mut upper=0u32;
 for position in 1..=r+1 { upper+=sample(position); }
 for x in 0..length {
  destination[start+x*stride]=(sum.wrapping_mul(mul)>>shift) as u8;
  if x==last { break; }
  let next=sample(x+1);

  sum=sum-lower+upper;
  lower=lower-sample(x.saturating_sub(r))+next;
  upper=upper-next+sample(x.saturating_add(r+2));
 }
}
