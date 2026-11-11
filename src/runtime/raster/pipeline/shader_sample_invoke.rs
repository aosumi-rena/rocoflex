use super::{Entry,Progress,Services,Target};
#[must_use]
pub struct Completed<I,R,O> {pub progress:Progress<I>,pub normal:super::super::NormalReturn<I,R,O>}
#[must_use = "keep SAME caller Frame and ALL exact stage failures/resources"]
pub enum Failure<I,R,O,E> {
    Entry {expected:Target<I>,observed:Option<Entry<I>>,error:Option<E>,mismatched:bool},
    Prefix(super::Failure<I,E>),
    Remainder(super::remainder::Failure<I,R,O,E>),
}
impl<I,R,O,E> Failure<I,R,O,E> {
    pub fn progress(&self)->Option<&Progress<I>> {match self {
        Self::Entry {..}=>None,Self::Prefix(f)=>Some(&f.progress),Self::Remainder(f)=>Some(&f.progress),
    }}
}
pub fn source<S:Services>(host:&mut S,frame:&mut S::Frame,target:Target<S::Identity>)
    ->Result<Completed<S::Identity,S::SampleReturnPacket,S::SampleOutputPacket>,
        Failure<S::Identity,S::SampleReturnPacket,S::SampleOutputPacket,S::Error>>
{
    let entry=host.enter_sample_27fe50(frame,&target).map_err(|error|Failure::Entry {
        expected:target,observed:None,error:Some(error),mismatched:false})?;
    if entry.target!=target {return Err(Failure::Entry {expected:target,observed:Some(entry),error:None,mismatched:true});}
    let required=super::prefix(host,frame,entry).map_err(Failure::Prefix)?;
    match super::remainder::continue_after_prefix(host,frame,required).map_err(Failure::Remainder)? {
        super::remainder::Outcome::Returned {progress,normal}=>Ok(Completed {progress,normal}),
    }
}
