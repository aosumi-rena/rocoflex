use crate::root_binding::VocoderConfigV2;

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum SourceTruncationUnit{Integer,Half}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct SourceTruncation578dd8{
    pub bits:u32,

    pub classification:i16,

    pub stored:bool,
}

pub fn truncate_578dd8_bits(bits:u32,unit:SourceTruncationUnit)->SourceTruncation578dd8{
    let exponent=(bits>>23)&255;let magnitude=bits&0x7fff_ffff;
    if exponent==255{return SourceTruncation578dd8{bits,classification:if bits&0x007f_ffff==0{1}else{2},stored:false};}
    if magnitude==0{return SourceTruncation578dd8{bits,classification:0,stored:false};}
    let offset=match unit{SourceTruncationUnit::Integer=>0,SourceTruncationUnit::Half=>1};
    let shift=150-exponent as i32-offset;
    if shift<=0{return SourceTruncation578dd8{bits,classification:0,stored:false};}
    if shift>=24{return SourceTruncation578dd8{bits:bits&0x8000_0000,classification:-1,stored:true};}
    let mask=(1u32<<shift)-1;
    SourceTruncation578dd8{bits:bits&!mask,classification:if bits&mask==0{0}else{-1},stored:true}
}

pub fn round_586310_bits(input_bits:u32)->u32{
    let half=truncate_578dd8_bits(input_bits,SourceTruncationUnit::Half);
    if half.classification==1||half.classification==2{return half.bits;}
    let integer=truncate_578dd8_bits(half.bits,SourceTruncationUnit::Integer);
    if integer.classification==0{return integer.bits;}
    let sign=integer.bits&0x8000_0000;
    if integer.bits&0x7fff_ffff==0{return sign|0x3f80_0000;}
    let biased=(integer.bits>>23)&255;let exponent=biased-127;
    let increment=1u32<<(23-exponent);
    let sum=(integer.bits&0x007f_ffff|0x0080_0000)+increment;
    if sum&0x0100_0000!=0{sign|((biased+1)<<23)|((sum>>1)&0x007f_ffff)}
    else{sign|(biased<<23)|(sum&0x007f_ffff)}
}

pub trait SourceRootRateInstructions{
    type Error;
    fn cvtsi2ss_signed32(&mut self,input:i32)->Result<u32,Self::Error>;
    fn mulss(&mut self,left:u32,right:u32)->Result<u32,Self::Error>;
}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct SourceSignalRootRate1136{
    sample_rate92:i32,divisor96_bits:u32,converted_rate_bits:u32,product_bits:u32,rounded_bits:u32,hop:i32,
}
pub enum SignalRootRateError<E>{Convert(E),Multiply{reason:E,converted_rate_bits:u32},
    UnsupportedCvtt{converted_rate_bits:u32,product_bits:u32,rounded_bits:u32},
    ProducerWidth{converted_rate_bits:u32,product_bits:u32,rounded_bits:u32,hop:i32}}
impl SourceSignalRootRate1136{

    pub fn from_bound_config<S:SourceRootRateInstructions>(config:&VocoderConfigV2,instructions:&mut S)
        ->Result<Self,SignalRootRateError<S::Error>>{
        let converted_rate_bits=instructions.cvtsi2ss_signed32(config.sample_rate).map_err(SignalRootRateError::Convert)?;
        let product_bits=instructions.mulss(converted_rate_bits,config.hop_seconds_bits)
            .map_err(|reason|SignalRootRateError::Multiply{reason,converted_rate_bits})?;
        let rounded_bits=round_586310_bits(product_bits);
        let biased=(rounded_bits>>23)&255;
        let negative=rounded_bits>>31!=0;
        let hop=if biased<127{0}else if biased==158&&negative&&rounded_bits&0x007f_ffff==0{i32::MIN}
        else if biased>=158{
            return Err(SignalRootRateError::UnsupportedCvtt{converted_rate_bits,product_bits,rounded_bits});
        }else{
            let exponent=biased-127;let significand=rounded_bits&0x007f_ffff|0x0080_0000;
            let magnitude=if exponent>=23{significand<<(exponent-23)}else{significand>>(23-exponent)};
            if negative{-(magnitude as i32)}else{magnitude as i32}
        };
        if hop<=0||hop.checked_mul(2).is_none(){return Err(SignalRootRateError::ProducerWidth{converted_rate_bits,product_bits,rounded_bits,hop});}
        Ok(Self{sample_rate92:config.sample_rate,divisor96_bits:config.hop_seconds_bits,converted_rate_bits,product_bits,rounded_bits,hop})
    }
    pub fn hop1136(self)->i32{self.hop}
    pub fn divisor96_bits(self)->u32{self.divisor96_bits}
    pub fn prefix_words(self)->[u32;3]{[self.converted_rate_bits,self.product_bits,self.rounded_bits]}
    pub fn sample_rate92(self)->i32{self.sample_rate92}
}
