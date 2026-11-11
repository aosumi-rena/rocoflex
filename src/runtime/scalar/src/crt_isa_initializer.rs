use super::scalar_cpu_initializer::{SourceScalarCpuRequest, SuppliedScalarCpuRecord,
    SuppliedCpuidRecord, SuppliedXgetbvRecord};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCrtIsaRequest {
    Instruction(SourceScalarCpuRequest),

    Flags860288,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCrtIsaRecord {
    Instruction(SuppliedScalarCpuRecord),
    Flags860288 { value: u32 },
}
impl SourceCrtIsaRecord {
    fn request(self) -> SourceCrtIsaRequest {
        match self {
            Self::Instruction(record) => SourceCrtIsaRequest::Instruction(record.request()),
            Self::Flags860288 { .. } => SourceCrtIsaRequest::Flags860288,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCrtIsaWrite {

    OrQword82b7d0 { mask: u64 },
    Flags860288 { value: u32 },
    Available82b7c8 { value: u32 },
    Enabled82b7cc { value: u32 },

    OrEnabled82b7cc { old_value: u32, mask: u32, value: u32 },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtIsaCompletion {
    flags_860288: u32,
    available_82b7c8: u32,
    enabled_82b7cc: u32,
    cpuid0: SuppliedCpuidRecord,
    cpuid1: SuppliedCpuidRecord,
    cpuid7: Option<SuppliedCpuidRecord>,
    xgetbv: Option<SuppliedXgetbvRecord>,
}
impl SourceCrtIsaCompletion {
    pub fn flags_860288(&self) -> u32 { self.flags_860288 }
    pub fn available_82b7c8(&self) -> u32 { self.available_82b7c8 }
    pub fn enabled_82b7cc(&self) -> u32 { self.enabled_82b7cc }
    pub fn supplied_cpuid0(&self) -> SuppliedCpuidRecord { self.cpuid0 }
    pub fn supplied_cpuid1(&self) -> SuppliedCpuidRecord { self.cpuid1 }
    pub fn supplied_cpuid7(&self) -> Option<SuppliedCpuidRecord> { self.cpuid7 }
    pub fn supplied_xgetbv(&self) -> Option<SuppliedXgetbvRecord> { self.xgetbv }
    pub fn return_eax(&self) -> u32 { 0 }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCrtIsaProgress {
    Request(SourceCrtIsaRequest),
    Complete(SourceCrtIsaCompletion),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCrtIsaError {
    RecordBindingRequired { pending: SourceCrtIsaProgress, supplied: SourceCrtIsaRecord },
}
impl SourceCrtIsaError {
    pub fn reason(self) -> &'static str {
        "561dd4 supplied instruction/flags record does not match pending source read"
    }
}

pub struct SourceCrtGeneralIsa561dd4 {
    progress: SourceCrtIsaProgress,
    cpuid0: Option<SuppliedCpuidRecord>,
    cpuid1: Option<SuppliedCpuidRecord>,
    cpuid7: Option<SuppliedCpuidRecord>,
    xgetbv: Option<SuppliedXgetbvRecord>,
    original_flags: Option<u32>,
    flags: u32,
    available: u32,
    enabled: u32,
    writes: [Option<SourceCrtIsaWrite>; 13],
    write_count: usize,
}
impl SourceCrtGeneralIsa561dd4 {
    pub fn begin_serial() -> Self {
        Self {
            progress: SourceCrtIsaProgress::Request(SourceCrtIsaRequest::Instruction(
                SourceScalarCpuRequest::Cpuid { eax: 0, ecx: 0 })),
            cpuid0: None, cpuid1: None, cpuid7: None, xgetbv: None, original_flags: None,
            flags: 0, available: 0, enabled: 0, writes: [None; 13], write_count: 0,
        }
    }
    pub fn progress(&self) -> SourceCrtIsaProgress { self.progress }
    pub fn request(&self) -> Option<SourceCrtIsaRequest> {
        match self.progress { SourceCrtIsaProgress::Request(request) => Some(request), _ => None }
    }
    pub fn logical_writes(&self) -> &[Option<SourceCrtIsaWrite>; 13] { &self.writes }
    pub fn supplied_flags_860288(&self) -> Option<u32> { self.original_flags }
    pub fn supplied_cpuid0(&self) -> Option<SuppliedCpuidRecord> { self.cpuid0 }
    pub fn supplied_cpuid1(&self) -> Option<SuppliedCpuidRecord> { self.cpuid1 }
    pub fn supplied_cpuid7(&self) -> Option<SuppliedCpuidRecord> { self.cpuid7 }
    pub fn supplied_xgetbv(&self) -> Option<SuppliedXgetbvRecord> { self.xgetbv }
    pub fn supply(&mut self, supplied: SourceCrtIsaRecord) -> Result<SourceCrtIsaProgress, SourceCrtIsaError> {
        if self.request() != Some(supplied.request()) {
            return Err(SourceCrtIsaError::RecordBindingRequired { pending: self.progress, supplied });
        }
        match supplied {
            SourceCrtIsaRecord::Instruction(SuppliedScalarCpuRecord::Cpuid(record)) => match record.input_eax {
                0 => {
                    self.cpuid0 = Some(record);
                    self.instruction(SourceScalarCpuRequest::Cpuid { eax: 1, ecx: 0 });
                }
                1 => {
                    self.cpuid1 = Some(record);

                    if let Some(zero) = self.cpuid0 {
                        if source_genuine_intel(zero) {
                            self.push(SourceCrtIsaWrite::OrQword82b7d0 { mask: u64::MAX });
                        }
                    }
                    self.progress = SourceCrtIsaProgress::Request(SourceCrtIsaRequest::Flags860288);
                }
                7 => {
                    self.cpuid7 = Some(record);
                    if record.ebx & (1 << 9) != 0 {
                        self.flags |= 2;
                        self.push(SourceCrtIsaWrite::Flags860288 { value: self.flags });
                    }
                    self.after_leaf7();
                }

                _ => return Err(SourceCrtIsaError::RecordBindingRequired { pending: self.progress, supplied }),
            },
            SourceCrtIsaRecord::Flags860288 { value } => {
                self.original_flags = Some(value);
                self.flags = value;
                if let (Some(zero), Some(one)) = (self.cpuid0, self.cpuid1) {
                    if source_genuine_intel(zero) && source_intel_quirk(one.eax) {
                        self.flags |= 1;
                        self.push(SourceCrtIsaWrite::Flags860288 { value: self.flags });
                    }

                    if (zero.eax as i32) >= 7 {
                        self.instruction(SourceScalarCpuRequest::Cpuid { eax: 7, ecx: 0 });
                    } else { self.after_leaf7(); }
                }
            }
            SourceCrtIsaRecord::Instruction(SuppliedScalarCpuRecord::Xgetbv(record)) => {
                self.xgetbv = Some(record);
                if record.eax & 6 == 6 {

                    let mut enabled = self.enabled | 8;
                    self.set_available(3);
                    self.set_enabled(enabled);

                    let leaf7_ebx = match self.cpuid7 { Some(seven) => seven.ebx, None => 0 };
                    if leaf7_ebx & 0x20 != 0 {
                        enabled |= 0x20;
                        self.set_available(5);
                        self.set_enabled(enabled);
                        if leaf7_ebx & 0xd003_0000 == 0xd003_0000 && record.eax & 0xe0 == 0xe0 {
                            let old = self.enabled;
                            self.enabled |= 0x40;
                            self.push(SourceCrtIsaWrite::OrEnabled82b7cc { old_value: old, mask: 0x40, value: self.enabled });
                            self.set_available(6);
                        }
                    }
                }
                self.complete();
            }
        }
        Ok(self.progress)
    }
    fn instruction(&mut self, request: SourceScalarCpuRequest) {
        self.progress = SourceCrtIsaProgress::Request(SourceCrtIsaRequest::Instruction(request));
    }
    fn after_leaf7(&mut self) {
        self.set_available(1);
        self.set_enabled(2);
        if let Some(one) = self.cpuid1 {
            if one.ecx & (1 << 20) != 0 {
                self.set_available(2);
                self.set_enabled(6);
                if one.ecx & 0x1800_0000 == 0x1800_0000 {
                    self.instruction(SourceScalarCpuRequest::Xgetbv { ecx: 0 });
                    return;
                }
            }
        }
        self.complete();
    }
    fn push(&mut self, write: SourceCrtIsaWrite) {

        self.writes[self.write_count] = Some(write);
        self.write_count += 1;
    }
    fn set_available(&mut self, value: u32) {
        self.available = value;
        self.push(SourceCrtIsaWrite::Available82b7c8 { value });
    }
    fn set_enabled(&mut self, value: u32) {
        self.enabled = value;
        self.push(SourceCrtIsaWrite::Enabled82b7cc { value });
    }
    fn complete(&mut self) {
        if let (Some(zero), Some(one)) = (self.cpuid0, self.cpuid1) {
            self.progress = SourceCrtIsaProgress::Complete(SourceCrtIsaCompletion {
                flags_860288: self.flags, available_82b7c8: self.available, enabled_82b7cc: self.enabled,
                cpuid0: zero, cpuid1: one, cpuid7: self.cpuid7, xgetbv: self.xgetbv,
            });
        }
    }
}

fn source_genuine_intel(zero: SuppliedCpuidRecord) -> bool {
    zero.ebx == 0x756e_6547 && zero.edx == 0x4965_6e69 && zero.ecx == 0x6c65_746e
}
fn source_intel_quirk(cpuid1_eax: u32) -> bool {
    let key = cpuid1_eax & 0x0fff_3ff0;
    if key == 0x106c0 || key == 0x20660 || key == 0x20670 { return true; }
    let index = key.wrapping_add(0xfffc_f9b0);
    index <= 0x20 && (0x1_0001_0001u64 >> index) & 1 != 0
}
