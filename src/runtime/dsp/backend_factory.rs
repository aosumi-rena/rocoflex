use crate::activation::{ArithmeticMode,SourceGateKernels};

pub const FLOAT_FACTORY_VTABLE_RVA:u32=0x5a_aae8;
pub const FLOAT_VECTOR_VTABLE_RVA:u32=0x5a_b3f8;
pub const FLOAT_VECTOR_FACTORY_RVA:u32=0x2_3a40;

#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum FactoryBindingError {
    WrongVtable {expected:u32,actual:u32},
    NonBooleanFlag {offset:usize,value:u8},
}

#[derive(Debug,Clone,Copy)]
pub struct SourceFeatureWords { pub at_78:u64,pub at_80:u64 }

#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct BackendFlags16([u8;16]);

impl BackendFlags16 {

    pub fn from_feature_words(feature_words:Option<SourceFeatureWords>)->Self {
        let mut raw=[0u8;16];
        raw[8..12].copy_from_slice(&4u32.to_le_bytes());
        raw[12..16].copy_from_slice(&1u32.to_le_bytes());
        if let Some(words)=feature_words {
            raw[0]=((words.at_78>>46)&1) as u8;
            raw[2]=((words.at_78>>11)&1) as u8;
            raw[3]=((words.at_78>>12)&1) as u8;
            raw[4]=((words.at_78>>47)&1) as u8;
            raw[5]=((words.at_80>>33)&1) as u8;
            raw[1]=((words.at_80>>36)&1) as u8;
            raw[6]=raw[4]&raw[0];
        }
        Self(raw)
    }

    pub fn from_bytes(raw:[u8;16])->Result<Self,FactoryBindingError> {
        for (offset,&value) in raw[..8].iter().enumerate() {
            if value>1 {return Err(FactoryBindingError::NonBooleanFlag{offset,value});}
        }
        Ok(Self(raw))
    }

    pub fn normalized_for_factory(mut self)->Self {
        if self.0[2]==0 {
            self.0[0]=0;
            self.0[3]=0;
            self.0[4]=0;
            self.0[6]=0;
        } else if self.0[4]==0 {self.0[6]=0;}
        self
    }

    pub fn bytes(self)->[u8;16] {self.0}
    pub fn avx_vector_path(self)->bool {self.0[2]!=0}
    pub fn fused_multiply_add(self)->bool {self.0[4]!=0}

    pub fn matrix_specialization_flag6(self)->bool {self.0[6]!=0}
}

#[derive(Debug,Clone,Copy)]
pub struct SourceFloatFactory { flags:BackendFlags16 }

impl SourceFloatFactory {

    pub fn bind(vtable_rva:u32,flags:BackendFlags16)->Result<Self,FactoryBindingError> {
        if vtable_rva!=FLOAT_FACTORY_VTABLE_RVA {
            return Err(FactoryBindingError::WrongVtable{
                expected:FLOAT_FACTORY_VTABLE_RVA,actual:vtable_rva});
        }
        Ok(Self{flags:flags.normalized_for_factory()})
    }

    pub fn flags(self)->BackendFlags16 {self.flags}
    pub fn vector_vtable_rva(self)->u32 {FLOAT_VECTOR_VTABLE_RVA}

    pub fn gate_kernels(self)->SourceGateKernels {
        SourceGateKernels{arithmetic:ArithmeticMode{
            fused_multiply_add:self.flags.fused_multiply_add()}}
    }
}
