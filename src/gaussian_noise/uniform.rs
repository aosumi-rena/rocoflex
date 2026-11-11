
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
