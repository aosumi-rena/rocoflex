use crate::backend_factory::SourceFloatFactory;

pub const PROJECTED_CONV_VTABLE_RVA:u32=0x5a4478;
pub const PROJECTED_CONV_FORWARD_RVA:u32=0x32ca0;

#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum ConvBindingError {
    WrongVtable,
    ConfigurationLength,
    ParserRejectsFifthWord,
    NonpositiveTapCount,
    TapCount,
    MatrixDimensions,
    CoefficientCount,
    SizeOverflow,
}

#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct ProjectedConvConfig {
    pub taps:i32,pub stride:i32,pub padding:i32,pub dilation:i32,pub fifth_word:i32,
}
impl ProjectedConvConfig {

    pub fn parse(bytes:&[u8])->Result<Self,ConvBindingError> {
        if bytes.len()!=20 {return Err(ConvBindingError::ConfigurationLength);}
        let word=|i:usize|i32::from_le_bytes(bytes[4*i..4*i+4].try_into().unwrap());
        let config=Self{taps:word(0),stride:word(1),padding:word(2),dilation:word(3),fifth_word:word(4)};
        if config.fifth_word!=1 {return Err(ConvBindingError::ParserRejectsFifthWord);}
        Ok(config)
    }
}

#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct SharedMatrixProfile(pub u64);

#[derive(Debug,Clone,Copy)]
pub struct MatrixBinding {
    pub output_width:usize,
    pub input_width:usize,
    pub compatible_profile:Option<SharedMatrixProfile>,
}

#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum ProjectedConvMode { GenericTaps0,PackedMatrix1 }

#[derive(Debug,Clone)]
pub struct ProjectedConvBinding {
    pub(crate) config:ProjectedConvConfig,
    pub(crate) input_width:usize,
    pub(crate) output_width:usize,
    pub(crate) mode:ProjectedConvMode,
}

impl ProjectedConvBinding {
    pub fn config(&self)->ProjectedConvConfig {self.config}
    pub fn input_width(&self)->usize {self.input_width}
    pub fn output_width(&self)->usize {self.output_width}
    pub fn mode(&self)->ProjectedConvMode {self.mode}

    pub fn bind(vtable_rva:u32,config:ProjectedConvConfig,matrices:&[MatrixBinding],
                factory:SourceFloatFactory,cloned:bool)->Result<Self,ConvBindingError> {
        if vtable_rva!=PROJECTED_CONV_VTABLE_RVA {return Err(ConvBindingError::WrongVtable);}
        if config.fifth_word!=1 {return Err(ConvBindingError::ParserRejectsFifthWord);}
        if config.taps<=0 {return Err(ConvBindingError::NonpositiveTapCount);}
        if matrices.len()!=config.taps as usize {return Err(ConvBindingError::TapCount);}
        let first=matrices[0];
        if matrices.iter().any(|m|m.input_width!=first.input_width||m.output_width!=first.output_width) {
            return Err(ConvBindingError::MatrixDimensions);
        }
        let valid_profile=first.compatible_profile.filter(|p|p.0!=0);

        let promote=!cloned&&factory.flags().bytes()[0]!=0
            &&valid_profile.is_some()
            &&matrices.iter().all(|m|m.compatible_profile==valid_profile);
        Ok(Self{config,input_width:first.input_width,output_width:first.output_width,
                mode:if promote {ProjectedConvMode::PackedMatrix1} else {ProjectedConvMode::GenericTaps0}})
    }

    pub fn pack_coefficients(&self,mut coefficient:impl FnMut(usize,usize,usize)->f32)
        ->Result<Vec<f32>,ConvBindingError> {
        let count=(self.config.taps as usize).checked_mul(self.input_width)
            .and_then(|n|n.checked_mul(self.output_width)).ok_or(ConvBindingError::SizeOverflow)?;
        let mut values=Vec::with_capacity(count);
        for tap in 0..self.config.taps as usize {
            for input in 0..self.input_width {
                for output in 0..self.output_width {values.push(coefficient(tap,output,input));}
            }
        }
        Ok(values)
    }
}
