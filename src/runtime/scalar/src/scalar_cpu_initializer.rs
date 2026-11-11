const REQUIRED_CPUID1_ECX: u32 = 0x1800_1000;
const REQUIRED_XCR0_LOW: u32 = 6;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceScalarCpuRequest {
    Cpuid { eax: u32, ecx: u32 },
    Xgetbv { ecx: u32 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SuppliedCpuidRecord {
    pub input_eax: u32,
    pub input_ecx: u32,
    pub eax: u32,
    pub ebx: u32,
    pub ecx: u32,
    pub edx: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SuppliedXgetbvRecord {
    pub input_ecx: u32,
    pub eax: u32,
    pub edx: u32,
}

impl SuppliedXgetbvRecord {
    pub fn joined_value(self) -> u64 { (u64::from(self.edx) << 32) | u64::from(self.eax) }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SuppliedScalarCpuRecord {
    Cpuid(SuppliedCpuidRecord),
    Xgetbv(SuppliedXgetbvRecord),
}

impl SuppliedScalarCpuRecord {
    pub fn request(self) -> SourceScalarCpuRequest {
        match self {
            Self::Cpuid(record) => SourceScalarCpuRequest::Cpuid { eax: record.input_eax, ecx: record.input_ecx },
            Self::Xgetbv(record) => SourceScalarCpuRequest::Xgetbv { ecx: record.input_ecx },
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceScalarInitializerWord {
    Feature861cb4,
    Selected861cb8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceScalarInitializerWrite {
    pub word: SourceScalarInitializerWord,
    pub value: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceScalarCpuInitialization {
    pub word_861cb4: u32,
    pub word_861cb8: u32,
    pub return_eax: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceScalarCpuProgress {
    Request(SourceScalarCpuRequest),
    Complete(SourceScalarCpuInitialization),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceScalarCpuInitializerError {
    RecordMismatch { expected: SourceScalarCpuRequest, supplied: SuppliedScalarCpuRecord },
    AlreadyComplete { completion: SourceScalarCpuInitialization, supplied: SuppliedScalarCpuRecord },
}

impl SourceScalarCpuInitializerError {
    pub fn reason(self) -> &'static str {
        match self {
            Self::RecordMismatch { .. } => "supplied CPU instruction/input does not match pending597370 request",
            Self::AlreadyComplete { .. } => "597370 source initializer already completed; record not consumed",
        }
    }
}

pub struct SourceScalarCpuInitializer {
    progress: SourceScalarCpuProgress,
    writes: [Option<SourceScalarInitializerWrite>; 3],
    cpuid: Option<SuppliedCpuidRecord>,
    xgetbv: Option<SuppliedXgetbvRecord>,
}

impl SourceScalarCpuInitializer {
    pub fn begin_serial() -> Self {
        Self {
            progress: SourceScalarCpuProgress::Request(SourceScalarCpuRequest::Cpuid { eax: 1, ecx: 0 }),
            writes: [Some(SourceScalarInitializerWrite { word: SourceScalarInitializerWord::Feature861cb4, value: 0 }), None, None],
            cpuid: None,
            xgetbv: None,
        }
    }
    pub fn progress(&self) -> SourceScalarCpuProgress { self.progress }
    pub fn request(&self) -> Option<SourceScalarCpuRequest> {
        match self.progress { SourceScalarCpuProgress::Request(request) => Some(request), _ => None }
    }
    pub fn completion(&self) -> Option<SourceScalarCpuInitialization> {
        match self.progress { SourceScalarCpuProgress::Complete(output) => Some(output), _ => None }
    }
    pub fn logical_writes(&self) -> [Option<SourceScalarInitializerWrite>; 3] { self.writes }
    pub fn supplied_cpuid(&self) -> Option<SuppliedCpuidRecord> { self.cpuid }
    pub fn supplied_xgetbv(&self) -> Option<SuppliedXgetbvRecord> { self.xgetbv }

    pub fn supply(&mut self, record: SuppliedScalarCpuRecord)
        -> Result<SourceScalarCpuProgress, SourceScalarCpuInitializerError>
    {
        let expected = match self.progress {
            SourceScalarCpuProgress::Request(request) => request,
            SourceScalarCpuProgress::Complete(completion) =>
                return Err(SourceScalarCpuInitializerError::AlreadyComplete { completion, supplied: record }),
        };
        if record.request() != expected {
            return Err(SourceScalarCpuInitializerError::RecordMismatch { expected, supplied: record });
        }
        match record {
            SuppliedScalarCpuRecord::Cpuid(record) => {
                self.cpuid = Some(record);
                if record.ecx & REQUIRED_CPUID1_ECX == REQUIRED_CPUID1_ECX {
                    self.progress = SourceScalarCpuProgress::Request(SourceScalarCpuRequest::Xgetbv { ecx: 0 });
                } else {

                    self.writes[1] = Some(SourceScalarInitializerWrite { word: SourceScalarInitializerWord::Selected861cb8, value: 0 });
                    self.progress = SourceScalarCpuProgress::Complete(SourceScalarCpuInitialization {
                        word_861cb4: 0, word_861cb8: 0, return_eax: 0,
                    });
                }
            }
            SuppliedScalarCpuRecord::Xgetbv(record) => {
                self.xgetbv = Some(record);

                let value = u32::from(record.eax & REQUIRED_XCR0_LOW == REQUIRED_XCR0_LOW);
                self.writes[1] = Some(SourceScalarInitializerWrite { word: SourceScalarInitializerWord::Feature861cb4, value });
                self.writes[2] = Some(SourceScalarInitializerWrite { word: SourceScalarInitializerWord::Selected861cb8, value });
                self.progress = SourceScalarCpuProgress::Complete(SourceScalarCpuInitialization {
                    word_861cb4: value, word_861cb8: value, return_eax: 0,
                });
            }
        }
        Ok(self.progress)
    }
    pub fn supply_cpuid(&mut self, record: SuppliedCpuidRecord)
        -> Result<SourceScalarCpuProgress, SourceScalarCpuInitializerError>
    { self.supply(SuppliedScalarCpuRecord::Cpuid(record)) }
    pub fn supply_xgetbv(&mut self, record: SuppliedXgetbvRecord)
        -> Result<SourceScalarCpuProgress, SourceScalarCpuInitializerError>
    { self.supply(SuppliedScalarCpuRecord::Xgetbv(record)) }
}
