use crate::source_scalar::{self as scalar, ExplicitExceptionState, PowerError,
    SourceCvttMxcsrCapabilities, SourceRawSqrtssOutcome, SourceRawSqrtssError,
    SourceRawCvtss2sdOutcome, SourceRawCvtss2sdError, SourceStrictUcomisdOutcome,
    SourceStrictUcomisdError, SourceRawUcomisdOutcome, SourceUcomisdFlags,
    SourceRawF64SelectionOutcome, SourceRawF64SelectionError};
use scalar::shared_scalar_f32::{ScalarF32Architecture, ScalarF32InstructionSet,
    ScalarF32Operation, ScalarF32Outcome, ScalarF32Error, ScalarF32Mxcsr, SourceScalarF32};

#[derive(Clone, Copy, Debug)]
pub struct SourceGenerationDecimationConfiguration {

    pub best_seed_bits: u64,
    pub epsilon_bits: u32,
    pub merge_scale_bits: u32,
    pub architecture: ScalarF32Architecture,
    pub instructions: ScalarF32InstructionSet,
    pub capabilities: SourceCvttMxcsrCapabilities,
}

#[derive(Clone, Copy, Debug)]
pub enum SourceGenerationDecimationBindingCause {
    Architecture,
    Controls,
    Backend(ScalarF32Error),
}
pub struct SourceGenerationDecimationBindingFailure {
    configuration: SourceGenerationDecimationConfiguration,
    entry: ExplicitExceptionState,
    cause: SourceGenerationDecimationBindingCause,
}
impl SourceGenerationDecimationBindingFailure {
    pub fn configuration(&self) -> SourceGenerationDecimationConfiguration { self.configuration }
    pub fn entry(&self) -> ExplicitExceptionState { self.entry }
    pub fn cause(&self) -> SourceGenerationDecimationBindingCause { self.cause }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceGenerationDecimationCandidateTag {
    pub round: usize,
    pub candidate: u32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceGenerationDecimationSpace { Rows4, Rows128 }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceGenerationDecimationSide { Left, Right }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceGenerationDecimationMergeTag {
    pub round: usize,
    pub selected: u32,
    pub space: SourceGenerationDecimationSpace,
    pub side: SourceGenerationDecimationSide,
    pub element: usize,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceGenerationDecimationCurrent {
    Candidate(SourceGenerationDecimationCandidateTag),
    Merge(SourceGenerationDecimationMergeTag),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceGenerationDecimationStage {
    Begin,
    Operation { slot: usize, pc: u32 },
    Complete,
}

#[derive(Clone, Copy, Debug)]
pub enum SourceGenerationDecimationOperands {
    Scalar(ScalarF32Operation),
    F32(u32),
    F64 { left: u64, right: u64 },
}
#[derive(Clone, Copy, Debug)]
pub enum SourceGenerationDecimationReport {
    Scalar(Result<ScalarF32Outcome, ScalarF32Error>),
    SquareRoot(Result<SourceRawSqrtssOutcome, SourceRawSqrtssError>),
    Widen(Result<SourceRawCvtss2sdOutcome, SourceRawCvtss2sdError>),
    Compare(Result<SourceStrictUcomisdOutcome, SourceStrictUcomisdError>),
    Minimum(Result<SourceRawF64SelectionOutcome, SourceRawF64SelectionError>),
}
#[derive(Clone, Copy, Debug)]
pub struct SourceGenerationDecimationAttempt {
    pub pc: u32,
    pub entry: ExplicitExceptionState,
    pub operands: SourceGenerationDecimationOperands,
    pub reported: SourceGenerationDecimationReport,

    pub raw_admission: Option<Result<ExplicitExceptionState, PowerError>>,
    pub known_advanced: ExplicitExceptionState,
}

#[derive(Clone, Copy, Debug)]
pub struct SourceGenerationDecimationScore {
    pub score_bits: u32,
    pub widened_score_bits: u64,
    pub new_best_bits: u64,
    pub index_should_replace: bool,
}
#[derive(Clone, Copy, Debug)]
pub struct SourceGenerationDecimationCandidate {
    pub tag: SourceGenerationDecimationCandidateTag,
    pub center: [u32; 4],
    pub left: [u32; 4],
    pub right: [u32; 4],
    pub prior_best_bits: u64,
    pub attempts: [Option<SourceGenerationDecimationAttempt>; 32],
    pub left_differences: [Option<u32>; 4],
    pub right_differences: [Option<u32>; 4],
    pub left_products: [Option<u32>; 4],
    pub right_products: [Option<u32>; 4],
    pub left_sum: Option<u32>,
    pub right_sum: Option<u32>,
    pub left_epsilon_sum: Option<u32>,
    pub right_epsilon_sum: Option<u32>,
    pub left_norm: Option<u32>,
    pub right_norm: Option<u32>,
    pub score_bits: Option<u32>,
    pub widened_score_bits: Option<u64>,

    pub pending_flags: Option<SourceUcomisdFlags>,
    pub completed: Option<SourceGenerationDecimationScore>,
}
#[derive(Clone, Copy, Debug)]
pub struct SourceGenerationDecimationMerge {
    pub tag: SourceGenerationDecimationMergeTag,
    pub center_bits: u32,
    pub neighbor_bits: u32,
    pub attempts: [Option<SourceGenerationDecimationAttempt>; 3],
    pub center_product: Option<u32>,
    pub neighbor_product: Option<u32>,
    pub completed_bits: Option<u32>,
}
pub struct SourceGenerationDecimationProgress {
    current: SourceGenerationDecimationCurrent,
    stage: SourceGenerationDecimationStage,
    candidate: Option<SourceGenerationDecimationCandidate>,
    merge: Option<SourceGenerationDecimationMerge>,
    candidate_reached: bool,
    merge_reached: bool,
}
impl SourceGenerationDecimationProgress {
    pub fn current(&self) -> SourceGenerationDecimationCurrent { self.current }
    pub fn stage(&self) -> SourceGenerationDecimationStage { self.stage }

    pub fn candidate(&self) -> Option<&SourceGenerationDecimationCandidate> { self.candidate.as_ref() }

    pub fn merge(&self) -> Option<&SourceGenerationDecimationMerge> { self.merge.as_ref() }
    pub fn candidate_reached(&self) -> bool { self.candidate_reached }
    pub fn merge_reached(&self) -> bool { self.merge_reached }
}

#[derive(Clone, Copy, Debug)]
pub enum SourceGenerationDecimationCause {
    ScalarInputStateRequired { mxcsr: u32 },
    Scalar(ScalarF32Error),
    ScalarTrapped,
    SquareRoot(SourceRawSqrtssError),
    SquareRootTrapped,
    Widen(SourceRawCvtss2sdError),
    WidenTrapped,
    Compare(SourceStrictUcomisdError),
    CompareTrapped,
    Minimum(SourceRawF64SelectionError),
    MinimumTrapped,
    RawStateAdmission(PowerError),
}
pub struct SourceGenerationDecimationFailure {
    current: SourceGenerationDecimationCurrent,
    slot: usize,
    pc: u32,
    cause: SourceGenerationDecimationCause,
}
impl SourceGenerationDecimationFailure {
    pub fn current(&self) -> SourceGenerationDecimationCurrent { self.current }
    pub fn slot(&self) -> usize { self.slot }
    pub fn pc(&self) -> u32 { self.pc }
    pub fn cause(&self) -> SourceGenerationDecimationCause { self.cause }
}

pub struct SourceGenerationDecimationStopped { _private: () }

pub struct SourceGenerationDecimationMath {
    configuration: SourceGenerationDecimationConfiguration,
    backend: SourceScalarF32,
    state: ExplicitExceptionState,
    progress: Option<SourceGenerationDecimationProgress>,
    failure: Option<SourceGenerationDecimationFailure>,
    active: bool,
}
impl SourceGenerationDecimationMath {
    pub fn from_explicit_parts(configuration: SourceGenerationDecimationConfiguration,
        entry: ExplicitExceptionState) -> Result<Self, SourceGenerationDecimationBindingFailure>
    {
        let cause = if configuration.architecture != ScalarF32Architecture::IntelSdm {
            Some(SourceGenerationDecimationBindingCause::Architecture)
        } else if entry.mxcsr() & 0xffff_ffc0 != 0x1f80 {
            Some(SourceGenerationDecimationBindingCause::Controls)
        } else { None };
        if let Some(cause) = cause {
            return Err(SourceGenerationDecimationBindingFailure { configuration, entry, cause });
        }
        let backend = match SourceScalarF32::from_explicit_architecture(
            configuration.architecture, configuration.instructions)
        {
            Ok(backend) => backend,
            Err(error) => return Err(SourceGenerationDecimationBindingFailure {
                configuration, entry, cause: SourceGenerationDecimationBindingCause::Backend(error) }),
        };
        Ok(Self { configuration, backend, state: entry, progress: None, failure: None, active: false })
    }
    pub fn configuration(&self) -> SourceGenerationDecimationConfiguration { self.configuration }

    pub fn exception_state(&self) -> ExplicitExceptionState { self.state }
    pub fn backend(&self) -> &SourceScalarF32 { &self.backend }
    pub fn progress(&self) -> Option<&SourceGenerationDecimationProgress> { self.progress.as_ref() }
    pub fn failure(&self) -> Option<&SourceGenerationDecimationFailure> { self.failure.as_ref() }
    pub fn is_active(&self) -> bool { self.active }

    fn marker() -> SourceGenerationDecimationStopped { SourceGenerationDecimationStopped { _private: () } }
    fn stop(&mut self, slot: usize, pc: u32, cause: SourceGenerationDecimationCause)
        -> SourceGenerationDecimationStopped
    {
        if self.failure.is_none() {
            self.failure = Some(SourceGenerationDecimationFailure {
                current: self.progress.as_ref().unwrap().current, slot, pc, cause });
        }
        self.active = false;
        Self::marker()
    }
    fn candidate_mut(&mut self) -> &mut SourceGenerationDecimationCandidate {
        self.progress.as_mut().unwrap().candidate.as_mut().unwrap()
    }
    fn merge_mut(&mut self) -> &mut SourceGenerationDecimationMerge {
        self.progress.as_mut().unwrap().merge.as_mut().unwrap()
    }
    fn step(&mut self, slot: usize, pc: u32) {
        self.progress.as_mut().unwrap().stage = SourceGenerationDecimationStage::Operation { slot, pc };
    }
    fn attempt_mut(&mut self, slot: usize) -> &mut SourceGenerationDecimationAttempt {
        match self.progress.as_ref().unwrap().current {
            SourceGenerationDecimationCurrent::Candidate(_) => self.candidate_mut().attempts[slot].as_mut().unwrap(),
            SourceGenerationDecimationCurrent::Merge(_) => self.merge_mut().attempts[slot].as_mut().unwrap(),
        }
    }
    fn save(&mut self, slot: usize, pc: u32, operands: SourceGenerationDecimationOperands,
        reported: SourceGenerationDecimationReport)
    {
        let attempt = SourceGenerationDecimationAttempt { pc, entry: self.state, operands, reported,
            raw_admission: None, known_advanced: self.state };
        match self.progress.as_ref().unwrap().current {
            SourceGenerationDecimationCurrent::Candidate(_) => self.candidate_mut().attempts[slot] = Some(attempt),
            SourceGenerationDecimationCurrent::Merge(_) => self.merge_mut().attempts[slot] = Some(attempt),
        }
    }
    fn adopt_raw(&mut self, word: u32, slot: usize, pc: u32) -> Result<(), SourceGenerationDecimationStopped> {
        let admission = ExplicitExceptionState::from_explicit_parts(word, self.state.errno());
        self.attempt_mut(slot).raw_admission = Some(admission);
        match admission {
            Ok(advanced) => {
                self.state = advanced;
                self.attempt_mut(slot).known_advanced = advanced;
                Ok(())
            }
            Err(reason) => Err(self.stop(slot, pc, SourceGenerationDecimationCause::RawStateAdmission(reason))),
        }
    }
    fn scalar(&mut self, slot: usize, pc: u32, operation: ScalarF32Operation)
        -> Result<u32, SourceGenerationDecimationStopped>
    {
        self.step(slot, pc);
        let before = match ScalarF32Mxcsr::from_explicit_word(self.state.mxcsr()) {
            Some(before) => before,
            None => return Err(self.stop(slot, pc, SourceGenerationDecimationCause::ScalarInputStateRequired {
                mxcsr: self.state.mxcsr() })),
        };
        let reported = self.backend.execute(operation, before);
        self.save(slot, pc, SourceGenerationDecimationOperands::Scalar(operation),
            SourceGenerationDecimationReport::Scalar(reported));
        let word = match reported {
            Ok(ScalarF32Outcome::Completed { state, .. })
            | Ok(ScalarF32Outcome::Trapped { state, .. }) => state.word(),

            Err(error) => return Err(self.stop(slot, pc, SourceGenerationDecimationCause::Scalar(error))),
        };
        self.adopt_raw(word, slot, pc)?;
        match reported {
            Ok(ScalarF32Outcome::Completed { bits, .. }) => Ok(bits),
            Ok(ScalarF32Outcome::Trapped { .. }) => Err(self.stop(slot, pc, SourceGenerationDecimationCause::ScalarTrapped)),
            Err(error) => Err(self.stop(slot, pc, SourceGenerationDecimationCause::Scalar(error))),
        }
    }
    fn square_root(&mut self, slot: usize, pc: u32, source: u32)
        -> Result<u32, SourceGenerationDecimationStopped>
    {
        self.step(slot, pc);
        let reported = scalar::sqrtss_with_raw_mxcsr(source, self.state.mxcsr(),
            self.configuration.capabilities, self.configuration.architecture);
        self.save(slot, pc, SourceGenerationDecimationOperands::F32(source),
            SourceGenerationDecimationReport::SquareRoot(reported));
        let word = match reported {
            Ok(SourceRawSqrtssOutcome::Completed { state, .. })
            | Ok(SourceRawSqrtssOutcome::Trapped { state, .. }) => state.word(),
            Err(error) => error.known_advanced_mxcsr(self.state.mxcsr()),
        };
        self.adopt_raw(word, slot, pc)?;
        match reported {
            Ok(SourceRawSqrtssOutcome::Completed { bits, .. }) => Ok(bits),
            Ok(SourceRawSqrtssOutcome::Trapped { .. }) => Err(self.stop(slot, pc, SourceGenerationDecimationCause::SquareRootTrapped)),
            Err(error) => Err(self.stop(slot, pc, SourceGenerationDecimationCause::SquareRoot(error))),
        }
    }
    fn widen(&mut self, slot: usize, pc: u32, source: u32)
        -> Result<u64, SourceGenerationDecimationStopped>
    {
        self.step(slot, pc);
        let reported = scalar::cvtss2sd_with_raw_mxcsr(source, self.state.mxcsr(),
            self.configuration.capabilities, self.configuration.architecture);
        self.save(slot, pc, SourceGenerationDecimationOperands::F32(source),
            SourceGenerationDecimationReport::Widen(reported));
        let word = match reported {
            Ok(SourceRawCvtss2sdOutcome::Completed { state, .. })
            | Ok(SourceRawCvtss2sdOutcome::Trapped { state, .. }) => state.word(),
            Err(error) => error.known_advanced_mxcsr(self.state.mxcsr()),
        };
        self.adopt_raw(word, slot, pc)?;
        match reported {
            Ok(SourceRawCvtss2sdOutcome::Completed { bits, .. }) => Ok(bits),
            Ok(SourceRawCvtss2sdOutcome::Trapped { .. }) => Err(self.stop(slot, pc, SourceGenerationDecimationCause::WidenTrapped)),
            Err(error) => Err(self.stop(slot, pc, SourceGenerationDecimationCause::Widen(error))),
        }
    }
    fn compare(&mut self, slot: usize, pc: u32, left: u64, right: u64)
        -> Result<SourceUcomisdFlags, SourceGenerationDecimationStopped>
    {
        self.step(slot, pc);
        let reported = scalar::ucomisd_f64_with_explicit_state(left, right, self.state, self.configuration.architecture);
        self.save(slot, pc, SourceGenerationDecimationOperands::F64 { left, right },
            SourceGenerationDecimationReport::Compare(reported));
        match reported {
            Ok(value) => {
                self.state = value.exception_state;
                self.attempt_mut(slot).known_advanced = self.state;
                match value.reported {
                    SourceRawUcomisdOutcome::Completed { flags, .. } => Ok(flags),
                    SourceRawUcomisdOutcome::Trapped { .. } => Err(self.stop(slot, pc, SourceGenerationDecimationCause::CompareTrapped)),
                }
            }
            Err(error) => {
                self.state = error.known_advanced_state(self.state);
                self.attempt_mut(slot).known_advanced = self.state;
                Err(self.stop(slot, pc, SourceGenerationDecimationCause::Compare(error)))
            }
        }
    }
    fn minimum(&mut self, slot: usize, pc: u32, left: u64, right: u64)
        -> Result<u64, SourceGenerationDecimationStopped>
    {
        self.step(slot, pc);
        let reported = scalar::minsd_with_raw_mxcsr(left, right, self.state.mxcsr(),
            self.configuration.capabilities, self.configuration.architecture);
        self.save(slot, pc, SourceGenerationDecimationOperands::F64 { left, right },
            SourceGenerationDecimationReport::Minimum(reported));
        let word = match reported {
            Ok(SourceRawF64SelectionOutcome::Completed { state, .. })
            | Ok(SourceRawF64SelectionOutcome::Trapped { state, .. }) => state.word(),
            Err(error) => error.known_advanced_mxcsr(self.state.mxcsr()),
        };
        self.adopt_raw(word, slot, pc)?;
        match reported {
            Ok(SourceRawF64SelectionOutcome::Completed { bits, .. }) => Ok(bits),
            Ok(SourceRawF64SelectionOutcome::Trapped { .. }) => Err(self.stop(slot, pc, SourceGenerationDecimationCause::MinimumTrapped)),
            Err(error) => Err(self.stop(slot, pc, SourceGenerationDecimationCause::Minimum(error))),
        }
    }

    pub(crate) fn score_candidate(&mut self, tag: SourceGenerationDecimationCandidateTag,
        center: [u32; 4], left: [u32; 4], right: [u32; 4], prior_best_bits: u64)
        -> Result<SourceGenerationDecimationScore, SourceGenerationDecimationStopped>
    {
        if self.failure.is_some() || self.active { return Err(Self::marker()); }
        let merge = self.progress.as_ref().and_then(|progress| progress.merge);
        self.progress = Some(SourceGenerationDecimationProgress {
            current: SourceGenerationDecimationCurrent::Candidate(tag), stage: SourceGenerationDecimationStage::Begin,
            candidate: Some(SourceGenerationDecimationCandidate { tag, center, left, right, prior_best_bits,
                attempts: [None; 32], left_differences: [None; 4], right_differences: [None; 4],
                left_products: [None; 4], right_products: [None; 4], left_sum: None, right_sum: None,
                left_epsilon_sum: None, right_epsilon_sum: None, left_norm: None, right_norm: None,
                score_bits: None, widened_score_bits: None, pending_flags: None, completed: None }),
            merge, candidate_reached: true, merge_reached: false });
        self.active = true;

        let dl0 = self.scalar(0, 0x2c9eb9, ScalarF32Operation::Subss { left: center[0], right: left[0] })?;
        self.candidate_mut().left_differences[0] = Some(dl0);
        let dr0 = self.scalar(1, 0x2c9ebe, ScalarF32Operation::Subss { left: center[0], right: right[0] })?;
        self.candidate_mut().right_differences[0] = Some(dr0);
        let pl0 = self.scalar(2, 0x2c9ec3, ScalarF32Operation::Mulss { left: dl0, right: dl0 })?;
        self.candidate_mut().left_products[0] = Some(pl0);

        let sl0 = self.scalar(3, 0x2c9ec7, ScalarF32Operation::Addss { left: pl0, right: 0x0000_0000 })?;
        self.candidate_mut().left_sum = Some(sl0);
        let pr0 = self.scalar(4, 0x2c9ecc, ScalarF32Operation::Mulss { left: dr0, right: dr0 })?;
        self.candidate_mut().right_products[0] = Some(pr0);
        let dl1 = self.scalar(5, 0x2c9ed3, ScalarF32Operation::Subss { left: center[1], right: left[1] })?;
        self.candidate_mut().left_differences[1] = Some(dl1);
        let dr1 = self.scalar(6, 0x2c9ed9, ScalarF32Operation::Subss { left: center[1], right: right[1] })?;
        self.candidate_mut().right_differences[1] = Some(dr1);
        let sr0 = self.scalar(7, 0x2c9edf, ScalarF32Operation::Addss { left: pr0, right: 0x0000_0000 })?;
        self.candidate_mut().right_sum = Some(sr0);
        let pl1 = self.scalar(8, 0x2c9ee4, ScalarF32Operation::Mulss { left: dl1, right: dl1 })?;
        self.candidate_mut().left_products[1] = Some(pl1);
        let pr1 = self.scalar(9, 0x2c9ee8, ScalarF32Operation::Mulss { left: dr1, right: dr1 })?;
        self.candidate_mut().right_products[1] = Some(pr1);
        let dl2 = self.scalar(10, 0x2c9ef6, ScalarF32Operation::Subss { left: center[2], right: left[2] })?;
        self.candidate_mut().left_differences[2] = Some(dl2);
        let sl01 = self.scalar(11, 0x2c9efc, ScalarF32Operation::Addss { left: pl1, right: sl0 })?;
        self.candidate_mut().left_sum = Some(sl01);
        let sr01 = self.scalar(12, 0x2c9f00, ScalarF32Operation::Addss { left: pr1, right: sr0 })?;
        self.candidate_mut().right_sum = Some(sr01);
        let dr2 = self.scalar(13, 0x2c9f04, ScalarF32Operation::Subss { left: center[2], right: right[2] })?;
        self.candidate_mut().right_differences[2] = Some(dr2);
        let pl2 = self.scalar(14, 0x2c9f0a, ScalarF32Operation::Mulss { left: dl2, right: dl2 })?;
        self.candidate_mut().left_products[2] = Some(pl2);
        let pr2 = self.scalar(15, 0x2c9f0f, ScalarF32Operation::Mulss { left: dr2, right: dr2 })?;
        self.candidate_mut().right_products[2] = Some(pr2);
        let sl012 = self.scalar(16, 0x2c9f13, ScalarF32Operation::Addss { left: pl2, right: sl01 })?;
        self.candidate_mut().left_sum = Some(sl012);
        let dl3 = self.scalar(17, 0x2c9f21, ScalarF32Operation::Subss { left: center[3], right: left[3] })?;
        self.candidate_mut().left_differences[3] = Some(dl3);
        let dr3 = self.scalar(18, 0x2c9f27, ScalarF32Operation::Subss { left: center[3], right: right[3] })?;
        self.candidate_mut().right_differences[3] = Some(dr3);
        let sr012 = self.scalar(19, 0x2c9f2d, ScalarF32Operation::Addss { left: pr2, right: sr01 })?;
        self.candidate_mut().right_sum = Some(sr012);
        let pl3 = self.scalar(20, 0x2c9f31, ScalarF32Operation::Mulss { left: dl3, right: dl3 })?;
        self.candidate_mut().left_products[3] = Some(pl3);
        let sl = self.scalar(21, 0x2c9f35, ScalarF32Operation::Addss { left: pl3, right: sl012 })?;
        self.candidate_mut().left_sum = Some(sl);
        let pr3 = self.scalar(22, 0x2c9f3a, ScalarF32Operation::Mulss { left: dr3, right: dr3 })?;
        self.candidate_mut().right_products[3] = Some(pr3);
        let sr = self.scalar(23, 0x2c9f3e, ScalarF32Operation::Addss { left: pr3, right: sr012 })?;
        self.candidate_mut().right_sum = Some(sr);
        let el = self.scalar(24, 0x2c9f42, ScalarF32Operation::Addss { left: sl, right: self.configuration.epsilon_bits })?;
        self.candidate_mut().left_epsilon_sum = Some(el);
        let nl = self.square_root(25, 0x2c9f4a, el)?;
        self.candidate_mut().left_norm = Some(nl);
        let er = self.scalar(26, 0x2c9f4e, ScalarF32Operation::Addss { left: sr, right: self.configuration.epsilon_bits })?;
        self.candidate_mut().right_epsilon_sum = Some(er);
        let nr = self.square_root(27, 0x2c9f53, er)?;
        self.candidate_mut().right_norm = Some(nr);
        let score = self.scalar(28, 0x2c9f57, ScalarF32Operation::Addss { left: nr, right: nl })?;
        self.candidate_mut().score_bits = Some(score);
        let widened = self.widen(29, 0x2c9f5e, score)?;
        self.candidate_mut().widened_score_bits = Some(widened);
        let flags = self.compare(30, 0x2c9f66, prior_best_bits, widened)?;
        self.candidate_mut().pending_flags = Some(flags);
        let new_best = self.minimum(31, 0x2c9f6a, widened, prior_best_bits)?;
        let completed = SourceGenerationDecimationScore { score_bits: score, widened_score_bits: widened,
            new_best_bits: new_best, index_should_replace: !flags.carry && !flags.zero };
        self.candidate_mut().completed = Some(completed);
        self.progress.as_mut().unwrap().stage = SourceGenerationDecimationStage::Complete;
        self.active = false;
        Ok(completed)
    }

    pub(crate) fn merge_neighbor(&mut self, tag: SourceGenerationDecimationMergeTag,
        center_bits: u32, neighbor_bits: u32) -> Result<u32, SourceGenerationDecimationStopped>
    {
        if self.failure.is_some() || self.active { return Err(Self::marker()); }
        let candidate = self.progress.as_ref().and_then(|progress| progress.candidate);
        self.progress = Some(SourceGenerationDecimationProgress {
            current: SourceGenerationDecimationCurrent::Merge(tag), stage: SourceGenerationDecimationStage::Begin,
            candidate, merge: Some(SourceGenerationDecimationMerge { tag, center_bits, neighbor_bits,
                attempts: [None; 3], center_product: None, neighbor_product: None, completed_bits: None }),
            candidate_reached: false, merge_reached: true });
        self.active = true;
        let pcs = match (tag.space, tag.side) {
            (SourceGenerationDecimationSpace::Rows4, SourceGenerationDecimationSide::Left) => [0x2c9fb9, 0x2c9fc4, 0x2c9fc9],
            (SourceGenerationDecimationSpace::Rows4, SourceGenerationDecimationSide::Right) => [0x2ca019, 0x2ca024, 0x2ca029],
            (SourceGenerationDecimationSpace::Rows128, SourceGenerationDecimationSide::Left) => [0x2ca069, 0x2ca074, 0x2ca079],
            (SourceGenerationDecimationSpace::Rows128, SourceGenerationDecimationSide::Right) => [0x2ca0b9, 0x2ca0c3, 0x2ca0c8],
        };
        let scale = self.configuration.merge_scale_bits;
        let center_product = self.scalar(0, pcs[0], ScalarF32Operation::Mulss { left: center_bits, right: scale })?;
        self.merge_mut().center_product = Some(center_product);
        let neighbor_product = self.scalar(1, pcs[1], ScalarF32Operation::Mulss { left: neighbor_bits, right: scale })?;
        self.merge_mut().neighbor_product = Some(neighbor_product);
        let bits = self.scalar(2, pcs[2], ScalarF32Operation::Addss { left: neighbor_product, right: center_product })?;
        self.merge_mut().completed_bits = Some(bits);
        self.progress.as_mut().unwrap().stage = SourceGenerationDecimationStage::Complete;
        self.active = false;
        Ok(bits)
    }
}
