pub const LTVFS1_TAG:u64=0x7567fac43d3254fd;
pub const LTVFS1_OUTER_CLASS:u32=0x5b0f58;
pub const LTVFS1_INNER_CLASS:u32=0x5b1258;
pub const LTVFS1_DIRECT_FORWARD:u32=0x66330;
pub const CONDITION_TAG:u64=0x2c02d2a65d15aa52;
pub const CONDITION_CLASS:u32=0x5adbf8;
pub const CONDITION_DIRECT_FORWARD:u32=0x4e5f0;

#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum OperatorBindingError {WrongTag,WrongClass,ConfigurationRejected,GenericForwardSlotIsNull}

#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct Ltvfs1Config13 {
    pub raw_word0:i32,
    pub flags4:u8,
    pub validation_word5:i32,
    pub validation_word9:i32,
}
impl Ltvfs1Config13 {

    pub fn parse(bytes:&[u8])->Result<Option<Self>,OperatorBindingError> {
        if bytes.len()!=13 {return Ok(None);}
        let word=|at|i32::from_le_bytes(bytes[at..at+4].try_into().unwrap());
        let result=Self{raw_word0:word(0),flags4:bytes[4],validation_word5:word(5),validation_word9:word(9)};
        if result.flags4&1!=0||result.validation_word5<=0||result.validation_word9<=0 {
            return Err(OperatorBindingError::ConfigurationRejected);
        }
        Ok(Some(result))
    }
}

#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct SourceLtvfs1Binding {config:Ltvfs1Config13}
impl SourceLtvfs1Binding {
    pub fn bind(decoded_tag:u64,outer_class:u32,inner_class:u32,config:Ltvfs1Config13)
      ->Result<Self,OperatorBindingError> {
        if decoded_tag!=LTVFS1_TAG {return Err(OperatorBindingError::WrongTag);}
        if outer_class!=LTVFS1_OUTER_CLASS||inner_class!=LTVFS1_INNER_CLASS {return Err(OperatorBindingError::WrongClass);}
        if config.flags4&1!=0||config.validation_word5<=0||config.validation_word9<=0 {return Err(OperatorBindingError::ConfigurationRejected);}
        Ok(Self{config})
    }
    pub fn config(self)->Ltvfs1Config13 {self.config}
    pub fn generic_forward32(self)->Result<(),OperatorBindingError> {Err(OperatorBindingError::GenericForwardSlotIsNull)}
    pub fn direct_forward_rva(self)->u32 {LTVFS1_DIRECT_FORWARD}
}

#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct ConditionModuleConfig {
    pub first:u16,pub second:u16,pub mode:u32,pub output_width:i32,
}
impl ConditionModuleConfig {

    pub fn parse(bytes:&[u8])->Option<Self> {
        if bytes.len()!=4&&bytes.len()!=12 {return None;}
        let first=u16::from_le_bytes(bytes[0..2].try_into().unwrap());
        let second=u16::from_le_bytes(bytes[2..4].try_into().unwrap());
        let (mode,output_width)=if bytes.len()==12 {
            (u32::from_le_bytes(bytes[4..8].try_into().unwrap()),i32::from_le_bytes(bytes[8..12].try_into().unwrap()))
        } else {(0,72)};
        if first==0||second==0||mode>1||output_width<=0 {return None;}
        Some(Self{first,second,mode,output_width})
    }
    pub fn bind_direct(self,decoded_tag:u64,class:u32)->Result<u32,OperatorBindingError> {
        if decoded_tag!=CONDITION_TAG {return Err(OperatorBindingError::WrongTag);}
        if class!=CONDITION_CLASS {return Err(OperatorBindingError::WrongClass);}
        if self.first==0||self.second==0||self.mode>1||self.output_width<=0 {return Err(OperatorBindingError::ConfigurationRejected);}
        Ok(CONDITION_DIRECT_FORWARD)
    }
}

#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum FilterCallError {AlreadyEnded,StatusMismatch,CountMismatch,FrameChanged,OutsidePositiveFrameDomain}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct FilterCallPrefix {
    pub initialize:bool,
    pub frame:i32,

    pub window_product_word:u32,
}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct SourceFilterCallState {frame:Option<i32>,ended_status:u8}
impl SourceFilterCallState {

    pub fn from_explicit_initialized_frame(frame:i32,ended_status:u8)->Result<Self,FilterCallError>{
        if frame<=0{return Err(FilterCallError::OutsidePositiveFrameDomain);}
        Ok(Self{frame:Some(frame),ended_status})
    }

    pub fn from_reset()->Self {Self{frame:None,ended_status:0}}
    pub fn ended_status(self)->u8 {self.ended_status}
    pub fn observe_positive_frame(&mut self,binding:SourceLtvfs1Binding,frame:i32,
      pointer_status:u8,handle_status:u8,pointer_rows:usize,pooled_rows:usize)
      ->Result<FilterCallPrefix,FilterCallError> {

        if frame<=0 {return Err(FilterCallError::OutsidePositiveFrameDomain);}
        if pointer_status!=handle_status {return Err(FilterCallError::StatusMismatch);}
        if self.ended_status!=0 {return Err(FilterCallError::AlreadyEnded);}
        if pointer_rows!=pooled_rows {return Err(FilterCallError::CountMismatch);}
        self.ended_status=handle_status;
        let initialize=self.frame.is_none();
        if let Some(previous)=self.frame {
            if previous!=frame {return Err(FilterCallError::FrameChanged);}
        } else {self.frame=Some(frame);}
        Ok(FilterCallPrefix{initialize,frame,window_product_word:(binding.config.raw_word0 as u32).wrapping_mul(frame as u32)})
    }
}
