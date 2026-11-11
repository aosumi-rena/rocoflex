#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct SourceUniform32 { state:u32 }
impl SourceUniform32 {
    pub fn from_state_word(state:u32)->Self {Self{state}}

    pub fn from_paired_state_reset()->Self {Self{state:1}}
    pub fn state_word(&self)->u32 {self.state}

    pub fn next_state(&mut self)->u32 {
        let product=(self.state as u64)*0xbc8f;
        let quotient=(((product as u128)*0x1000000020001u128)>>64) as u64 >>15;
        let folded=((quotient as u32)<<31)|(quotient as u32);
        self.state=(product as u32).wrapping_add(folded);
        self.state
    }

    pub fn next_uniform_rne(&mut self)->f32 {
        let integer=self.next_state() as f32;
        let adjusted=integer+f32::from_bits(0xbf800000);
        adjusted*f32::from_bits(0x30000000)
    }
}

#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct GaussianPairCache {pub pending:bool,pub partner_bits:u32}
impl GaussianPairCache {

    pub fn fresh_for_paired_call()->Self {Self{pending:false,partner_bits:0}}
}

#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum PairedBatchContractError {AlreadyEnded,StatusMismatch,CountMismatch}
pub fn validate_paired_batch(ended:bool,vector_status:u8,record_status:u8,
    vectors:usize,records:usize)->Result<bool,PairedBatchContractError> {
    if ended {return Err(PairedBatchContractError::AlreadyEnded);}
    if vector_status!=record_status {return Err(PairedBatchContractError::StatusMismatch);}
    if vectors!=records {return Err(PairedBatchContractError::CountMismatch);}
    Ok(vector_status!=0)
}

#[derive(Debug,Clone,PartialEq,Eq)]
pub struct SignalEmissionPlan {
    pub first_half_after_records:Vec<usize>,
    pub final_second_half:bool,
    pub output_status:u8,
}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum SignalStateError {AlreadyEnded,RecordCountOutsideSigned32Domain}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct SourceSignalEmission { first:bool, ended:bool }
impl SourceSignalEmission {

    pub fn from_reset()->Self {Self{first:true,ended:false}}
    pub fn plan_batch(&mut self,records:usize,status:u8)->Result<SignalEmissionPlan,SignalStateError> {
        if self.ended {return Err(SignalStateError::AlreadyEnded);}
        if records>i32::MAX as usize {return Err(SignalStateError::RecordCountOutsideSigned32Domain);}
        self.ended=status!=0;
        let start=usize::from(self.first).min(records);
        let first_half_after_records=(start..records).collect();
        if records!=0 {self.first=false;}
        Ok(SignalEmissionPlan{first_half_after_records,final_second_half:self.ended,
            output_status:u8::from(self.ended)})
    }
}
