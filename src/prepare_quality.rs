use serde_json::Value;
use crate::{Result,pitch_selected_head::PitchHeadSelection};

pub(crate) const MALFORMED:&str="Quality preferences are malformed. Use a boolean realtime preference and a text quality label.";

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub(crate) enum SynthesisFamily {Primary,Alternate}

#[repr(u32)]
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub(crate) enum PrepareProfile { Highest=0, Higher=1, Lower=2, Lowest=3 }
impl Default for PrepareProfile {fn default()->Self {Self::Highest}}
impl TryFrom<u32> for PrepareProfile {
    type Error=String;
    fn try_from(value:u32)->Result<Self> {match value {
        0=>Ok(Self::Highest),1=>Ok(Self::Higher),2=>Ok(Self::Lower),
        3=>Ok(Self::Lowest),_=>Err(MALFORMED.into()),
    }}
}
impl PrepareProfile {
    pub(crate) fn pitch_head(self)->PitchHeadSelection {match self {
        Self::Highest=>PitchHeadSelection::Head2,Self::Higher|Self::Lower|Self::Lowest=>PitchHeadSelection::Head0,
    }}
    pub(crate) fn decoder_child(self)->usize {match self {Self::Lower|Self::Lowest=>4,_=>5}}
    pub(crate) fn synthesis_family(self)->SynthesisFamily {
        match self {Self::Lowest=>SynthesisFamily::Alternate,_=>SynthesisFamily::Primary}
    }
}

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub(crate) enum QualityLabel {Highest,Higher,Lower,Lowest}
impl QualityLabel {
    pub(crate) fn index(self)->usize {match self {Self::Highest=>0,Self::Higher=>1,Self::Lower=>2,Self::Lowest=>3}}
}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub(crate) struct Preferences {
    pub realtime:bool,pub label:QualityLabel,pub defaulted_label:bool,
}
impl Default for Preferences {fn default()->Self {Self {realtime:true,label:QualityLabel::Highest,defaulted_label:false}}}
impl Preferences {

    pub(crate) fn from_document(document:&Value)->Result<Self> {
        if !document.is_object() {return Err(MALFORMED.into());}
        let voice=document.get("voice").unwrap_or(document);
        let realtime=match document.get("useRealtimeLatency").or_else(||voice.get("useRealtimeLatency")) {
            None=>true,Some(value)=>value.as_bool().ok_or(MALFORMED)?,
        };
        let label=document.get("realtimeLatencyOption").or_else(||voice.get("quality"));
        let (label,defaulted_label)=match label {
            None=>(QualityLabel::Highest,false),
            Some(value)=>match value.as_str().ok_or(MALFORMED)? {
                "Highest Quality"|"highest"=>(QualityLabel::Highest,false),
                "Higher Quality"|"higher"=>(QualityLabel::Higher,false),
                "Lower Latency"|"lower"=>(QualityLabel::Lower,false),
                "Lowest Latency"|"lowest"=>(QualityLabel::Lowest,false),
                _=>(QualityLabel::Highest,true),
            },
        };
        Ok(Self {realtime,label,defaulted_label})
    }
    pub(crate) fn profile(self)->Result<PrepareProfile> {
        if !self.realtime {return Ok(PrepareProfile::Highest);}
        PrepareProfile::try_from(self.label.index() as u32)
    }
    pub(crate) fn choice_code(self)->u32 {
        if !self.realtime {0} else {self.label.index() as u32}
    }
    pub(crate) fn indication(self)->Option<&'static str> {
        if self.defaulted_label {Some("Unrecognized quality label: using Highest Quality.")}
        else if !self.realtime {Some("Realtime preference is off: using Highest Quality.")}
        else {None}
    }
}

#[repr(C)]
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct RocoQualityChoice {pub profile:u32,pub defaulted_label:u32}
impl RocoQualityChoice {fn invalid()->Self {Self {profile:4,defaulted_label:0}}}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn roco_quality_resolve(document:*const u8,length:usize,out:*mut RocoQualityChoice)->i32 {
    if out.is_null() {return -1;}
    unsafe {*out=RocoQualityChoice::invalid();}
    if length>16*1024*1024 || (length!=0 && document.is_null()) {return -1;}
    let result=std::panic::catch_unwind(||->Result<RocoQualityChoice> {
        let preferences=if length==0 {Preferences::default()} else {
            let raw=unsafe {std::slice::from_raw_parts(document,length)};
            let value:Value=serde_json::from_slice(raw).map_err(|_|MALFORMED.to_owned())?;
            Preferences::from_document(&value)?
        };
        Ok(RocoQualityChoice {profile:preferences.choice_code(),defaulted_label:u32::from(preferences.defaulted_label)})
    });
    match result {Ok(Ok(choice))=>{unsafe {*out=choice;}0},Ok(Err(_))=>-1,Err(_)=>-3}
}

