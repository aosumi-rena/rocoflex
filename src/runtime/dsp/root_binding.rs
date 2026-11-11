#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindingError {
    ConfigurationLength,
    NonpositiveCount,
    UnsupportedVocoderConfiguration,
    MissingChild,
    CepstralModeMismatch,
    GateChildCount,
    InvalidGateConfiguration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GateConfig {
    pub input_width: i32,
    pub half_width: i32,
    pub condition_width: i32,
    pub has_auxiliary: bool,
    pub warmup_rows: i32,
}

impl GateConfig {
    pub fn parse(bytes: &[u8]) -> Result<Self, BindingError> {
        if ![12, 16, 20].contains(&bytes.len()) {
            return Err(BindingError::ConfigurationLength);
        }
        let value = Self {
            input_width: word(bytes, 0) as i32,
            half_width: word(bytes, 4) as i32,
            condition_width: word(bytes, 8) as i32,
            has_auxiliary: bytes.len() >= 16 && word(bytes, 12) != 0,
            warmup_rows: if bytes.len() == 20 { word(bytes, 16) as i32 } else { 0 },
        };

        if bytes.len() == 20 &&
            (value.warmup_rows < 0 || value.input_width <= 0 || value.half_width <= 0) {
            return Err(BindingError::InvalidGateConfiguration);
        }
        Ok(value)
    }

    pub fn plan(&self, child_count: usize) -> Result<GatePlan, BindingError> {
        GatePlan::from_runtime_fields(self.half_width as u32, self.condition_width,
                                     self.has_auxiliary, child_count)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GatePlan {
    pub main_projection_child: usize,
    pub condition_projection_child: Option<usize>,
    pub auxiliary_child: Option<usize>,
    pub temporary_vector_width: u32,
}

impl GatePlan {
    pub fn from_runtime_fields(width_at_84: u32, condition_width_at_88: i32,
                               auxiliary_flag_at_96: bool, child_count: usize)
        -> Result<Self, BindingError>
    {
        let conditional = condition_width_at_88 > 0;
        let expected = 1 + usize::from(conditional) + usize::from(auxiliary_flag_at_96);
        if child_count != expected { return Err(BindingError::GateChildCount); }
        Ok(Self {
            main_projection_child: 0,
            condition_projection_child: if conditional { Some(1) } else { None },
            auxiliary_child: if auxiliary_flag_at_96 {
                Some(if conditional { 2 } else { 1 })
            } else { None },
            temporary_vector_width: width_at_84,
        })
    }
}

fn word(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RootConfig18 {
    pub count: i32,
    pub parameter_24_bits: u32,
    pub parameter_28_bits: u32,
    pub has_pre_module: bool,
    pub subgroup_flag: bool,
    pub parameter_32_bits: u32,
}

impl RootConfig18 {
    pub fn parse(bytes: &[u8]) -> Result<Self, BindingError> {
        if bytes.len() != 18 { return Err(BindingError::ConfigurationLength); }
        let count = word(bytes, 0) as i32;
        if count <= 0 { return Err(BindingError::NonpositiveCount); }
        Ok(Self {
            count,
            parameter_24_bits: word(bytes, 4),
            parameter_28_bits: word(bytes, 8),
            has_pre_module: bytes[12] != 0,
            subgroup_flag: bytes[13] != 0,
            parameter_32_bits: word(bytes, 14),
        })
    }

    pub const fn has_extra_filter(&self) -> bool { false }

    fn first_main_child(&self) -> usize { if self.has_pre_module { 2 } else { 1 } }

    pub fn cepstral_child_index(&self) -> usize { self.first_main_child() + 2 }

    pub fn bind_children(&self, count: usize, cepstral_mode: u32)
        -> Result<DescriptorPlan, BindingError>
    {

        if cepstral_mode != 2 { return Err(BindingError::CepstralModeMismatch); }
        let n = self.first_main_child();

        if count < n + 11 { return Err(BindingError::MissingChild); }
        let children = [
            Some(0),
            if self.has_pre_module { Some(1) } else { None },
            Some(n), Some(n + 1), Some(n + 4), Some(n + 3), Some(n + 2),
            Some(n + 5), Some(n + 6), None,
            Some(n + 7), Some(n + 8), Some(n + 9), Some(n + 10), Some(count - 1),
        ];
        Ok(DescriptorPlan {
            children,
            count: self.count,
            parameter_24_bits: self.parameter_24_bits,
            parameter_28_bits: self.parameter_28_bits,
            parameter_32_bits: self.parameter_32_bits,
            subgroup_flag: self.subgroup_flag,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DescriptorPlan {
    pub children: [Option<usize>; 15],
    pub count: i32,
    pub parameter_24_bits: u32,
    pub parameter_28_bits: u32,
    pub parameter_32_bits: u32,
    pub subgroup_flag: bool,
}

impl DescriptorPlan {

    pub fn direct_context_fields(&self) -> [(usize, Option<usize>); 8] {
        [(80, self.children[0]), (96, self.children[2]),
         (104, self.children[3]), (112, self.children[4]),
         (120, self.children[5]), (128, self.children[6]),
         (136, self.children[7]), (144, self.children[8])]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VocoderConfigV2 {
    pub field_80: u32,
    pub field_84: u32,
    pub sample_rate: i32,
    pub hop_seconds_bits: u32,
    pub field_100: i32,
    pub fields_104_to_116: [u32; 4],
    pub flags_120_to_123: [bool; 4],
}

impl VocoderConfigV2 {
    pub fn parse(bytes: &[u8]) -> Result<Self, BindingError> {
        if bytes.len() != 52 { return Err(BindingError::ConfigurationLength); }
        let field_100 = word(bytes, 16) as i32;
        if field_100 <= 3 { return Err(BindingError::UnsupportedVocoderConfiguration); }
        Ok(Self {
            field_80: word(bytes, 0), field_84: word(bytes, 4),
            sample_rate: word(bytes, 8) as i32,
            hop_seconds_bits: word(bytes, 12), field_100,
            fields_104_to_116: [word(bytes, 20), word(bytes, 24), word(bytes, 28), word(bytes, 32)],
            flags_120_to_123: [word(bytes, 36) != 0, word(bytes, 40) != 0,
                               word(bytes, 44) != 0, word(bytes, 48) != 0],
        })
    }
    pub const fn native_field_88(&self) -> u32 { 0 }
}
