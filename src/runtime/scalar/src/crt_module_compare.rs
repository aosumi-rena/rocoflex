use super::crt_module_load::{SourceCrtModuleLoadNativeCall, SourceCrtModuleLoadNativeKind};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCrtModuleComparePhase { LoopLeft, LoopRight, FinalLeft, FinalRight }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtModuleCompareRead {
    pub phase: SourceCrtModuleComparePhase,
    pub actual_address: u64,
    pub source_instruction_rva: u32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtModuleCompareReadReceipt {
    pub read: SourceCrtModuleCompareRead,
    pub actual_word: u16,

    pub actual_next_rip: u64,
    pub actual_rax: u64,
    pub actual_rcx: u64,
    pub actual_rdx: u64,
    pub actual_r8: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtModuleCompareLive {
    pub rax: u64, pub rcx: u64, pub rdx: u64, pub r8: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtModuleCompareCompletion {
    call: SourceCrtModuleLoadNativeCall,
    result_eax: u32,
    source_live_at_ret: SourceCrtModuleCompareLive,
    reads: [Option<SourceCrtModuleCompareReadReceipt>; 14],
    read_count: usize,
}
impl SourceCrtModuleCompareCompletion {
    pub fn call(&self) -> SourceCrtModuleLoadNativeCall { self.call }
    pub fn result_eax(&self) -> u32 { self.result_eax }
    pub fn source_live_at_ret(&self) -> SourceCrtModuleCompareLive { self.source_live_at_ret }
    pub fn reads(&self) -> &[Option<SourceCrtModuleCompareReadReceipt>; 14] { &self.reads }
    pub fn read_count(&self) -> usize { self.read_count }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCrtModuleCompareProgress {
    Read(SourceCrtModuleCompareRead),
    Complete(SourceCrtModuleCompareCompletion),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCrtModuleCompareError {
    BoundSevenWordCallRequired { actual_module_base: u64, call: SourceCrtModuleLoadNativeCall },
    ReceiptRequired { pending: SourceCrtModuleCompareProgress, actual: SourceCrtModuleCompareReadReceipt },
    ActualLiveContinuationRequired { actual: SourceCrtModuleCompareReadReceipt,
        expected_next_rip: u64, expected_live: SourceCrtModuleCompareLive },
    ReadStorageContinuationRequired { actual: SourceCrtModuleCompareReadReceipt },
}
impl SourceCrtModuleCompareError {
    pub fn reason(self) -> &'static str {
        match self {
            Self::BoundSevenWordCallRequired { .. } => "593e68 module suffix requires the exact bound59be1c seven-word call",
            Self::ReceiptRequired { .. } => "593e68 actual fresh word receipt differs from pending source read",
            Self::ActualLiveContinuationRequired { .. } => "593e68 actual read prefix changed source live control registers/resume point",
            Self::ReadStorageContinuationRequired { .. } => "593e68 bounded seven-word read-prefix storage requires continuation",
        }
    }
}
#[must_use = "retain actual memory/frame/fault/full-state owner and partial fresh-read cursor"]
pub struct SourceCrtModuleCompareServiceFailure<E> { pub cause: E, pub pending: SourceCrtModuleCompareRead }
pub struct SourceCrtModuleCompareRejectedFailure<E> { pub cause: E, pub completed: SourceCrtModuleCompareCompletion }

pub struct SourceCrtModuleCompare593e68 {
    actual_module_base: u64,
    call: SourceCrtModuleLoadNativeCall,
    left: u64,
    right: u64,
    remaining: u64,
    loop_left_word: Option<u16>,
    final_left_word: Option<u16>,
    reads: [Option<SourceCrtModuleCompareReadReceipt>; 14],
    read_count: usize,
    progress: SourceCrtModuleCompareProgress,
    failed_service: bool,
}
impl SourceCrtModuleCompare593e68 {
    pub fn begin_bound_module_call(actual_module_base: u64, call: SourceCrtModuleLoadNativeCall)
        -> Result<Self, SourceCrtModuleCompareError> {
        let invalid = SourceCrtModuleCompareError::BoundSevenWordCallRequired { actual_module_base, call };
        if call.kind != SourceCrtModuleLoadNativeKind::Compare593e68 || call.overrides.r8 != Some(7)
            || call.overrides.rdx != Some(actual_module_base.wrapping_add(0x77e3a0))
            || call.required_actual_resume_rip != actual_module_base.wrapping_add(0x59be21) { return Err(invalid); }
        let Some(left) = call.overrides.rcx else { return Err(invalid); };
        let Some(right) = call.overrides.rdx else { return Err(invalid); };
        if call.required_actual_saved.rbx != 0 || call.required_actual_saved.r15 != left
            || call.required_actual_saved.r14 != u64::MAX { return Err(invalid); }

        let remaining = 7u64.wrapping_sub(1);
        let read = SourceCrtModuleCompareRead { phase: SourceCrtModuleComparePhase::LoopLeft,
            actual_address: left, source_instruction_rva: 0x593e70 };
        Ok(Self { actual_module_base, call, left, right, remaining, loop_left_word: None, final_left_word: None,
            reads: [None; 14], read_count: 0, progress: SourceCrtModuleCompareProgress::Read(read), failed_service: false })
    }
    pub fn call(&self) -> SourceCrtModuleLoadNativeCall { self.call }
    pub fn progress(&self) -> SourceCrtModuleCompareProgress { self.progress }
    pub fn reads(&self) -> &[Option<SourceCrtModuleCompareReadReceipt>; 14] { &self.reads }
    pub fn read_count(&self) -> usize { self.read_count }
    pub fn request(&self) -> Option<SourceCrtModuleCompareRead> {
        if self.failed_service { None } else { self.awaiting_actual_completion() }
    }
    pub fn awaiting_actual_completion(&self) -> Option<SourceCrtModuleCompareRead> {
        match self.progress { SourceCrtModuleCompareProgress::Read(read) => Some(read), _ => None }
    }
    pub fn accept_actual_read(&mut self, actual: SourceCrtModuleCompareReadReceipt)
        -> Result<SourceCrtModuleCompareProgress, SourceCrtModuleCompareError> {
        self.failed_service = true;
        let SourceCrtModuleCompareProgress::Read(expected) = self.progress else {
            return Err(SourceCrtModuleCompareError::ReceiptRequired { pending: self.progress, actual });
        };
        if actual.read != expected {
            return Err(SourceCrtModuleCompareError::ReceiptRequired { pending: self.progress, actual });
        }
        let (rax, rcx, next_rva) = match expected.phase {
            SourceCrtModuleComparePhase::LoopLeft => (u64::from(actual.actual_word), self.left, 0x593e73),
            SourceCrtModuleComparePhase::LoopRight => {
                let Some(left) = self.loop_left_word else {
                    return Err(SourceCrtModuleCompareError::ReceiptRequired { pending: self.progress, actual });
                };
                (u64::from(left), self.left, 0x593e7b)
            }
            SourceCrtModuleComparePhase::FinalLeft => (u64::from(actual.actual_word), self.left, 0x593e8e),
            SourceCrtModuleComparePhase::FinalRight => {
                let Some(left) = self.final_left_word else {
                    return Err(SourceCrtModuleCompareError::ReceiptRequired { pending: self.progress, actual });
                };
                (u64::from(left), u64::from(actual.actual_word), 0x593e91)
            }
        };
        let expected_live = SourceCrtModuleCompareLive { rax, rcx, rdx: self.right, r8: self.remaining };
        let expected_next_rip = self.actual_module_base.wrapping_add(next_rva);
        if actual.actual_next_rip != expected_next_rip || actual.actual_rax != rax || actual.actual_rcx != rcx
            || actual.actual_rdx != self.right || actual.actual_r8 != self.remaining {
            return Err(SourceCrtModuleCompareError::ActualLiveContinuationRequired { actual, expected_next_rip, expected_live });
        }
        let Some(slot) = self.reads.get_mut(self.read_count) else {
            return Err(SourceCrtModuleCompareError::ReadStorageContinuationRequired { actual });
        };
        *slot = Some(actual); self.read_count += 1;
        match expected.phase {
            SourceCrtModuleComparePhase::LoopLeft => {
                self.loop_left_word = Some(actual.actual_word);
                if actual.actual_word == 0 { self.next(SourceCrtModuleComparePhase::FinalLeft); }
                else { self.next(SourceCrtModuleComparePhase::LoopRight); }
            }
            SourceCrtModuleComparePhase::LoopRight => {
                if self.loop_left_word != Some(actual.actual_word) { self.next(SourceCrtModuleComparePhase::FinalLeft); }
                else {
                    self.left = self.left.wrapping_add(2); self.right = self.right.wrapping_add(2);
                    self.remaining = self.remaining.wrapping_sub(1);
                    if self.remaining != 0 { self.next(SourceCrtModuleComparePhase::LoopLeft); }
                    else { self.next(SourceCrtModuleComparePhase::FinalLeft); }
                }
            }
            SourceCrtModuleComparePhase::FinalLeft => {
                self.final_left_word = Some(actual.actual_word);
                self.next(SourceCrtModuleComparePhase::FinalRight);
            }
            SourceCrtModuleComparePhase::FinalRight => {

                let result_eax = (rax as u32).wrapping_sub(u32::from(actual.actual_word));
                self.progress = SourceCrtModuleCompareProgress::Complete(SourceCrtModuleCompareCompletion {
                    call: self.call, result_eax,
                    source_live_at_ret: SourceCrtModuleCompareLive {
                        rax: u64::from(result_eax), rcx, rdx: self.right, r8: self.remaining },
                    reads: self.reads, read_count: self.read_count,
                });
            }
        }
        self.failed_service = false;
        Ok(self.progress)
    }
    pub fn retain_service_failure<E>(&mut self, cause: E)
        -> Result<SourceCrtModuleCompareServiceFailure<E>, SourceCrtModuleCompareRejectedFailure<E>> {
        match self.progress {
            SourceCrtModuleCompareProgress::Read(pending) => {
                self.failed_service = true; Ok(SourceCrtModuleCompareServiceFailure { cause, pending })
            }
            SourceCrtModuleCompareProgress::Complete(completed) => Err(SourceCrtModuleCompareRejectedFailure { cause, completed }),
        }
    }
    fn next(&mut self, phase: SourceCrtModuleComparePhase) {
        let (actual_address, source_instruction_rva) = match phase {
            SourceCrtModuleComparePhase::LoopLeft => (self.left, 0x593e70),
            SourceCrtModuleComparePhase::LoopRight => (self.right, 0x593e78),
            SourceCrtModuleComparePhase::FinalLeft => (self.left, 0x593e8b),
            SourceCrtModuleComparePhase::FinalRight => (self.right, 0x593e8e),
        };
        self.progress = SourceCrtModuleCompareProgress::Read(SourceCrtModuleCompareRead { phase, actual_address, source_instruction_rva });
    }
}
