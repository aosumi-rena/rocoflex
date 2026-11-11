#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeKind { Point, Trajectory, Secondary }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ControlSlot { First, Second }

pub trait SourceReleaseServices<C, B: Copy> {
    type Error;

    fn prepare_shared_tail(&mut self, source: &mut C, kind: NodeKind) -> Result<(), Self::Error>;

    fn current_control(&mut self, source: &C, slot: ControlSlot) -> Result<Option<B>, Self::Error>;

    fn decrement_strong(&mut self, source: &mut C, control: B) -> Result<bool, Self::Error>;

    fn destroy_payload(&mut self, source: &mut C, control: B) -> Result<(), Self::Error>;

    fn decrement_weak(&mut self, source: &mut C, control: B) -> Result<bool, Self::Error>;

    fn destroy_control_storage(&mut self, source: &mut C, control: B) -> Result<(), Self::Error>;
    fn deallocate_source_storage(&mut self, source: &mut C) -> Result<(), Self::Error>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase { Prelude, FirstRead, Strong, Payload, Weak, ControlStorage, SecondRead, SourceStorage, Returned }
pub enum Issue<E> { AlreadyHalted, AlreadyReturned, RequiredService(E) }
pub struct Stopped<C, B, S, E> { pub pending: SourceRelease<C, B, S>, pub issue: Issue<E> }

#[must_use = "Keep partially released source/control/services; never recreate old ownership after Err."]
pub struct SourceRelease<C, B, S> {
    source: C, services: S, kind: NodeKind, delete_flags: u32,
    phase: Phase, halted: bool, slot: Option<ControlSlot>, control: Option<B>,
}
pub struct OwnedParts<C, B, S> {
    pub source: C, pub services: S, pub kind: NodeKind, pub delete_flags: u32,
    pub phase: Phase, pub halted: bool, pub slot: Option<ControlSlot>, pub control: Option<B>,
}
impl<C, B: Copy, S: SourceReleaseServices<C, B>> SourceRelease<C, B, S> {

    pub fn new(source: C, kind: NodeKind, delete_flags: u32, services: S) -> Self {
        Self { source, services, kind, delete_flags, phase: Phase::Prelude,
            halted: false, slot: None, control: None }
    }
    pub fn source(&self) -> &C { &self.source }
    pub fn phase(&self) -> Phase { self.phase }
    fn finish_control(&mut self) {
        self.control = None;
        self.phase = match self.slot.take().unwrap() {
            ControlSlot::First => Phase::SecondRead, ControlSlot::Second => Phase::SourceStorage,
        };
    }

    pub fn advance(mut self) -> Result<Self, Stopped<C, B, S, S::Error>> {
        if self.halted { return Err(Stopped { pending: self, issue: Issue::AlreadyHalted }); }
        let result = match self.phase {
            Phase::Prelude => match self.services.prepare_shared_tail(&mut self.source, self.kind) {
                Ok(()) => { self.phase = Phase::FirstRead; Ok(()) }
                Err(error) => Err(Issue::RequiredService(error)),
            },
            Phase::FirstRead | Phase::SecondRead => {
                let slot = if self.phase == Phase::FirstRead { ControlSlot::First } else { ControlSlot::Second };
                self.slot = Some(slot);
                match self.services.current_control(&self.source, slot) {
                    Ok(control) => {
                        self.control = control;
                        if control.is_some() { self.phase = Phase::Strong; } else { self.finish_control(); }
                        Ok(())
                    }
                    Err(error) => Err(Issue::RequiredService(error)),
                }
            }
            Phase::Strong => match self.services.decrement_strong(&mut self.source, self.control.unwrap()) {
                Ok(true) => { self.phase = Phase::Payload; Ok(()) }
                Ok(false) => { self.finish_control(); Ok(()) }
                Err(error) => Err(Issue::RequiredService(error)),
            },
            Phase::Payload => match self.services.destroy_payload(&mut self.source, self.control.unwrap()) {
                Ok(()) => { self.phase = Phase::Weak; Ok(()) }
                Err(error) => Err(Issue::RequiredService(error)),
            },
            Phase::Weak => match self.services.decrement_weak(&mut self.source, self.control.unwrap()) {
                Ok(true) => { self.phase = Phase::ControlStorage; Ok(()) }
                Ok(false) => { self.finish_control(); Ok(()) }
                Err(error) => Err(Issue::RequiredService(error)),
            },
            Phase::ControlStorage => match self.services.destroy_control_storage(&mut self.source, self.control.unwrap()) {
                Ok(()) => { self.finish_control(); Ok(()) }
                Err(error) => Err(Issue::RequiredService(error)),
            },
            Phase::SourceStorage => {
                if self.delete_flags == 0 { self.phase = Phase::Returned; Ok(()) }
                else { match self.services.deallocate_source_storage(&mut self.source) {
                    Ok(()) => { self.phase = Phase::Returned; Ok(()) }
                    Err(error) => Err(Issue::RequiredService(error)),
                } }
            }
            Phase::Returned => Err(Issue::AlreadyReturned),
        };
        match result {
            Ok(()) => Ok(self),
            Err(issue) => { self.halted = true; Err(Stopped { pending: self, issue }) }
        }
    }

    pub fn into_returned(self) -> Result<(C, S), Self> {
        if !self.halted && self.phase == Phase::Returned { Ok((self.source, self.services)) } else { Err(self) }
    }
    pub fn into_owned_parts(self) -> OwnedParts<C, B, S> {
        OwnedParts { source: self.source, services: self.services, kind: self.kind,
            delete_flags: self.delete_flags, phase: self.phase, halted: self.halted,
            slot: self.slot, control: self.control }
    }
}

pub trait TrajectoryChildServices<C, N> {
    type Error;

    fn current_count(&mut self, source: &C) -> Result<i32, Self::Error>;

    fn detach_shift_decrement(&mut self, source: &mut C, index: u32, tail_count: u32)
        -> Result<Option<N>, (Option<N>, Self::Error)>;
    fn delete_child(&mut self, source: &mut C, child: N)
        -> Result<(), (Option<N>, Self::Error)>;

    fn zero_count_free_current_array(&mut self, source: &mut C) -> Result<(), Self::Error>;
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChildPhase { CaptureCount, CurrentIndex, Detach, Release, ZeroAndFree, Returned }
pub enum ChildIssue<E> {
    AlreadyHalted, AlreadyReturned, NegativeCount(i32),
    CapturedIndexInvalid { index: u32, count: i32 }, RequiredService(E),
}
pub struct ChildStopped<C, N, S, E> {
    pub pending: TrajectoryChildDrain<C, N, S>, pub issue: ChildIssue<E>,
}
#[must_use = "Keep source/services/detached child; captured traversal must not restart after refusal."]
pub struct TrajectoryChildDrain<C, N, S> {
    source: C, services: S, phase: ChildPhase, halted: bool,
    initial_count: Option<u32>, remaining_indices: u32, tail_count: Option<u32>, child: Option<N>,
}
pub struct ChildOwnedParts<C, N, S> {
    pub source: C, pub services: S, pub phase: ChildPhase, pub halted: bool,
    pub initial_count: Option<u32>, pub remaining_indices: u32,
    pub tail_count: Option<u32>, pub child: Option<N>,
}
impl<C, N, S: TrajectoryChildServices<C, N>> TrajectoryChildDrain<C, N, S> {
    pub fn new(source: C, services: S) -> Self {
        Self { source, services, phase: ChildPhase::CaptureCount, halted: false,
            initial_count: None, remaining_indices: 0, tail_count: None, child: None }
    }
    pub fn source(&self) -> &C { &self.source }
    pub fn phase(&self) -> ChildPhase { self.phase }

    pub fn advance(mut self) -> Result<Self, ChildStopped<C, N, S, S::Error>> {
        if self.halted { return Err(ChildStopped { pending: self, issue: ChildIssue::AlreadyHalted }); }
        let result = match self.phase {
            ChildPhase::CaptureCount => match self.services.current_count(&self.source) {
                Ok(count) if count < 0 => Err(ChildIssue::NegativeCount(count)),
                Ok(count) => {
                    self.initial_count = Some(count as u32); self.remaining_indices = count as u32;
                    self.phase = if count == 0 { ChildPhase::ZeroAndFree } else { ChildPhase::CurrentIndex }; Ok(())
                }
                Err(error) => Err(ChildIssue::RequiredService(error)),
            },
            ChildPhase::CurrentIndex => match self.services.current_count(&self.source) {
                Ok(count) if count < 0 => Err(ChildIssue::NegativeCount(count)),
                Ok(count) => {
                    let index = self.remaining_indices - 1;
                    if count as u32 <= index { Err(ChildIssue::CapturedIndexInvalid { index, count }) }
                    else {
                        self.tail_count = Some(count as u32 - index - 1);
                        self.phase = ChildPhase::Detach; Ok(())
                    }
                }
                Err(error) => Err(ChildIssue::RequiredService(error)),
            },
            ChildPhase::Detach => match self.services.detach_shift_decrement(&mut self.source,
                self.remaining_indices - 1, self.tail_count.unwrap()) {
                Ok(child) => { self.child = child; self.phase = ChildPhase::Release; Ok(()) }
                Err((child, error)) => { self.child = child; Err(ChildIssue::RequiredService(error)) }
            },
            ChildPhase::Release => {
                let result = match self.child.take() {
                    None => Ok(()),
                    Some(child) => match self.services.delete_child(&mut self.source, child) {
                        Ok(()) => Ok(()),
                        Err((remaining, error)) => { self.child = remaining; Err(ChildIssue::RequiredService(error)) }
                    }
                };
                result.map(|()| {
                    self.remaining_indices -= 1; self.tail_count = None;
                    self.phase = if self.remaining_indices == 0 { ChildPhase::ZeroAndFree } else { ChildPhase::CurrentIndex };
                })
            }
            ChildPhase::ZeroAndFree => match self.services.zero_count_free_current_array(&mut self.source) {
                Ok(()) => { self.phase = ChildPhase::Returned; Ok(()) }
                Err(error) => Err(ChildIssue::RequiredService(error)),
            },
            ChildPhase::Returned => Err(ChildIssue::AlreadyReturned),
        };
        match result {
            Ok(()) => Ok(self),
            Err(issue) => { self.halted = true; Err(ChildStopped { pending: self, issue }) }
        }
    }
    pub fn into_returned(self) -> Result<(C, S), Self> {
        if !self.halted && self.phase == ChildPhase::Returned { Ok((self.source, self.services)) } else { Err(self) }
    }
    pub fn into_owned_parts(self) -> ChildOwnedParts<C, N, S> {
        ChildOwnedParts { source: self.source, services: self.services, phase: self.phase, halted: self.halted,
            initial_count: self.initial_count, remaining_indices: self.remaining_indices,
            tail_count: self.tail_count, child: self.child }
    }
}

pub struct NameOwner<B> { pub control: B, pub static_sentinel: bool }

pub trait NameReleaseServices<C, B: Copy> {
    type Error;

    fn current_name_owner(&mut self, source: &C) -> Result<NameOwner<B>, Self::Error>;

    fn fetch_decrement_name(&mut self, source: &mut C, owner: B) -> Result<u32, Self::Error>;
    fn free_name_owner(&mut self, source: &mut C, owner: B) -> Result<(), Self::Error>;
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NamePhase { CurrentOwner, Atomic, Free, Returned }
pub enum NameIssue<E> { AlreadyHalted, AlreadyReturned, RequiredService(E) }
pub struct NameStopped<C, B, S, E> { pub pending: NameRelease<C, B, S>, pub issue: NameIssue<E> }
#[must_use = "Keep the advanced source/name owner/services after partial release failure."]
pub struct NameRelease<C, B, S> {
    source: C, services: S, phase: NamePhase, halted: bool,
    owner: Option<B>, old_word: Option<u32>,
}
pub struct NameOwnedParts<C, B, S> {
    pub source: C, pub services: S, pub phase: NamePhase, pub halted: bool,
    pub owner: Option<B>, pub old_word: Option<u32>,
}
impl<C, B: Copy, S: NameReleaseServices<C, B>> NameRelease<C, B, S> {
    pub fn new(source: C, services: S) -> Self {
        Self { source, services, phase: NamePhase::CurrentOwner, halted: false, owner: None, old_word: None }
    }
    pub fn source(&self) -> &C { &self.source }
    pub fn phase(&self) -> NamePhase { self.phase }
    pub fn advance(mut self) -> Result<Self, NameStopped<C, B, S, S::Error>> {
        if self.halted { return Err(NameStopped { pending: self, issue: NameIssue::AlreadyHalted }); }
        let result = match self.phase {
            NamePhase::CurrentOwner => match self.services.current_name_owner(&self.source) {
                Ok(owner) => {
                    self.owner = Some(owner.control);
                    self.phase = if owner.static_sentinel { NamePhase::Returned } else { NamePhase::Atomic }; Ok(())
                }
                Err(error) => Err(NameIssue::RequiredService(error)),
            },
            NamePhase::Atomic => match self.services.fetch_decrement_name(&mut self.source, self.owner.unwrap()) {
                Ok(old) => { self.old_word = Some(old); self.phase = if old == 0 { NamePhase::Free } else { NamePhase::Returned }; Ok(()) }
                Err(error) => Err(NameIssue::RequiredService(error)),
            },
            NamePhase::Free => match self.services.free_name_owner(&mut self.source, self.owner.unwrap()) {
                Ok(()) => { self.phase = NamePhase::Returned; Ok(()) }
                Err(error) => Err(NameIssue::RequiredService(error)),
            },
            NamePhase::Returned => Err(NameIssue::AlreadyReturned),
        };
        match result {
            Ok(()) => Ok(self),
            Err(issue) => { self.halted = true; Err(NameStopped { pending: self, issue }) }
        }
    }
    pub fn into_returned(self) -> Result<(C, S), Self> {
        if !self.halted && self.phase == NamePhase::Returned { Ok((self.source, self.services)) } else { Err(self) }
    }
    pub fn into_owned_parts(self) -> NameOwnedParts<C, B, S> {
        NameOwnedParts { source: self.source, services: self.services, phase: self.phase, halted: self.halted,
            owner: self.owner, old_word: self.old_word }
    }
}
