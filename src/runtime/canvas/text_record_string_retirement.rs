#[path = "text_record_string_retirement_borrowed.rs"]
pub mod borrowed;

pub trait SavedTextServices {
    type Error;

    fn lock_xadd_minus_one(&mut self, header: u64, held: &mut Option<u32>) -> Result<(), Self::Error>;

    fn free_saved_header(&mut self, header: u64) -> Result<(), Self::Error>;
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SavedTextPhase { Sentinel, Atomic, Branch, Free, Done }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SavedTextResult { StaticSentinel, ReferenceDecremented, FreeReturned }
#[derive(Debug)]
pub enum SavedTextFault<E> { Service(E), MissingPrior, MissingResult }
#[derive(Debug)]
pub struct SavedTextFailure<E> {
    pub fault: SavedTextFault<E>, pub phase: SavedTextPhase,
    pub saved_text: u64, pub header: u64, pub prior: Option<u32>,
    pub result: Option<SavedTextResult>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SavedTextRetired {
    pub saved_text: u64, pub header: u64, pub prior: Option<u32>, pub result: SavedTextResult,
}
fn saved_header(saved_text: u64) -> u64 { saved_text.wrapping_sub(0x10) }

pub struct SavedTextCursor { saved_text: u64, source_image_base: u64, header: u64,
    prior: Option<u32>, result: Option<SavedTextResult>, phase: SavedTextPhase, stopped: bool }
#[derive(Debug)]
pub enum SavedTextCursorFault<E> { AlreadyStopped, AlreadyComplete, Graph(SavedTextFailure<E>) }
impl SavedTextCursor {
    pub fn from_saved_word(saved_text: u64, actual_source_image_base: u64) -> Self {
        Self { saved_text, source_image_base: actual_source_image_base, header: saved_header(saved_text),
            prior: None, result: None, phase: SavedTextPhase::Sentinel, stopped: false }
    }
    pub fn phase(&self) -> SavedTextPhase { self.phase }
    pub fn saved_text(&self) -> u64 { self.saved_text }
    pub fn header(&self) -> u64 { self.header }
    pub fn prior(&self) -> Option<u32> { self.prior }
    pub fn stopped(&self) -> bool { self.stopped }
    pub fn retired(&self) -> Option<SavedTextRetired> {
        if self.stopped || self.phase != SavedTextPhase::Done { return None; }
        self.result.map(|result| SavedTextRetired { saved_text: self.saved_text,
            header: self.header, prior: self.prior, result })
    }
    pub fn advance_once<S: SavedTextServices + ?Sized>(&mut self, services: &mut S)
        -> Result<(), SavedTextCursorFault<S::Error>> {
        if self.stopped { return Err(SavedTextCursorFault::AlreadyStopped); }
        if self.phase == SavedTextPhase::Done { return Err(SavedTextCursorFault::AlreadyComplete); }
        self.advance_core(services).map_err(SavedTextCursorFault::Graph)
    }
    fn advance_core<S: SavedTextServices + ?Sized>(&mut self, services: &mut S)
        -> Result<(), SavedTextFailure<S::Error>> {
        let attempted = self.phase;
        if let Err(fault) = step_saved_text(services, self.header, self.source_image_base,
            &mut self.prior, &mut self.result, &mut self.phase) {
            self.stopped = true;
            return Err(SavedTextFailure { fault, phase: attempted, saved_text: self.saved_text,
                header: self.header, prior: self.prior, result: self.result });
        }
        Ok(())
    }
}

fn step_saved_text<S: SavedTextServices + ?Sized>(services: &mut S,
    header: u64, actual_source_image_base: u64, prior: &mut Option<u32>,
    result: &mut Option<SavedTextResult>, phase: &mut SavedTextPhase)
    -> Result<(), SavedTextFault<S::Error>>
{
    match *phase {
        SavedTextPhase::Sentinel => {
            if header == actual_source_image_base.wrapping_add(0x839cc8) {
                *result = Some(SavedTextResult::StaticSentinel); *phase = SavedTextPhase::Done;
            } else { *phase = SavedTextPhase::Atomic; }
        }
        SavedTextPhase::Atomic => {
            services.lock_xadd_minus_one(header, prior).map_err(SavedTextFault::Service)?;
            if prior.is_none() { return Err(SavedTextFault::MissingPrior); }
            *phase = SavedTextPhase::Branch;
        }
        SavedTextPhase::Branch => {
            if prior.ok_or(SavedTextFault::MissingPrior)? == 0 { *phase = SavedTextPhase::Free; }
            else { *result = Some(SavedTextResult::ReferenceDecremented); *phase = SavedTextPhase::Done; }
        }
        SavedTextPhase::Free => {
            services.free_saved_header(header).map_err(SavedTextFault::Service)?;
            *result = Some(SavedTextResult::FreeReturned); *phase = SavedTextPhase::Done;
        }
        SavedTextPhase::Done => {}
    }
    Ok(())
}

pub fn retire_saved_text_borrowed<S: SavedTextServices + ?Sized>(services: &mut S,
    saved_text: u64, actual_source_image_base: u64)
    -> Result<SavedTextRetired, SavedTextFailure<S::Error>>
{
    let mut cursor = SavedTextCursor::from_saved_word(saved_text, actual_source_image_base);
    while cursor.phase != SavedTextPhase::Done {
        cursor.advance_core(services)?;
    }
    match cursor.retired() {
        Some(retired) => Ok(retired),
        None => Err(SavedTextFailure { fault: SavedTextFault::MissingResult,
            phase: cursor.phase, saved_text, header: cursor.header, prior: cursor.prior, result: cursor.result }),
    }
}

pub trait Services {
    type Error;

    fn read_u64(&mut self, address: u64) -> Result<u64, Self::Error>;

    fn lock_xadd_minus_one(&mut self, header: u64, held: &mut Option<u32>) -> Result<(), Self::Error>;

    fn tail_free55da4c(&mut self, saved_header: u64) -> Result<(), Self::Error>;
}
struct TailServices<'a, S: Services> { services: &'a mut S }
impl<S: Services> SavedTextServices for TailServices<'_, S> {
    type Error = S::Error;
    fn lock_xadd_minus_one(&mut self, header: u64, held: &mut Option<u32>) -> Result<(), Self::Error> {
        Services::lock_xadd_minus_one(self.services, header, held)
    }
    fn free_saved_header(&mut self, header: u64) -> Result<(), Self::Error> {
        Services::tail_free55da4c(self.services, header)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase { Pointer, Sentinel, Atomic, Branch, TailFree, Done }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Retirement { StaticSentinel, ReferenceDecremented, FreeTailReturned }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Entry { Record1a7310, StaticCallback1cf040 }
#[derive(Debug)]
pub enum Fault<E> { Service(E), MissingState, AlreadyStopped }
pub struct Prepared<S: Services> {
    services: S, entry: Entry, slot: u64, source_image_base: u64, text: Option<u64>, header: Option<u64>,
    prior: Option<u32>, result: Option<Retirement>,
    phase: Phase, attempted: Option<Phase>, halted: bool,
}
pub struct Stopped<S: Services> { pub pending: Prepared<S>, pub fault: Fault<S::Error> }
pub enum Poll<S: Services> { Pending(Prepared<S>), Ready(Finished<S>), Failed(Stopped<S>) }

pub struct Finished<S: Services> {
    pub services: S, pub entry: Entry, pub saved_slot: u64, pub retired_text: u64,
    pub source_image_base: u64, pub saved_header: u64, pub prior: Option<u32>, pub result: Retirement,
}
impl<S: Services> Prepared<S> {

    pub fn at_1a7310(services: S, actual_rcx_slot: u64, actual_source_image_base: u64) -> Self {
        Self::from_slot(services, Entry::Record1a7310, actual_rcx_slot, actual_source_image_base)
    }

    pub fn at_1cf040(services: S, actual_source_image_base: u64) -> Self {
        Self::from_slot(services, Entry::StaticCallback1cf040,
            actual_source_image_base.wrapping_add(0x863618), actual_source_image_base)
    }
    fn from_slot(services: S, entry: Entry, slot: u64, source_image_base: u64) -> Self {
        Self { services, entry, slot, source_image_base, text: None, header: None,
            prior: None, result: None, phase: Phase::Pointer, attempted: None, halted: false }
    }
    pub fn entry(&self) -> Entry { self.entry }
    pub fn phase(&self) -> Phase { self.phase }
    pub fn attempted(&self) -> Option<Phase> { self.attempted }
    pub fn services(&self) -> &S { &self.services }
    pub fn run(self) -> Result<Finished<S>, Stopped<S>> {
        let mut pending = self;
        loop {
            match pending.poll(1) {
                Poll::Pending(next) => pending = next,
                Poll::Ready(done) => return Ok(done),
                Poll::Failed(stopped) => return Err(stopped),
            }
        }
    }

    pub fn poll(mut self, budget: usize) -> Poll<S> {
        if self.halted { return Poll::Failed(Stopped { pending: self, fault: Fault::AlreadyStopped }); }
        let mut spent = 0usize;
        while self.phase != Phase::Done && spent < budget {
            self.attempted = Some(self.phase);
            spent += 1;
            if let Err(fault) = self.step() {
                self.halted = true;
                return Poll::Failed(Stopped { pending: self, fault });
            }
            self.attempted = None;
        }
        if self.phase != Phase::Done { return Poll::Pending(self); }
        let (Some(text), Some(header), Some(result)) = (self.text, self.header, self.result) else {
            self.halted = true;
            return Poll::Failed(Stopped { pending: self, fault: Fault::MissingState });
        };
        Poll::Ready(Finished { services: self.services, entry: self.entry, saved_slot: self.slot,
            retired_text: text, source_image_base: self.source_image_base,
            saved_header: header, prior: self.prior, result })
    }
    fn step(&mut self) -> Result<(), Fault<S::Error>> {
        match self.phase {
            Phase::Pointer => {
                let text = self.services.read_u64(self.slot).map_err(Fault::Service)?;
                self.text = Some(text); self.header = Some(saved_header(text));
                self.phase = Phase::Sentinel;
            }
            Phase::Sentinel | Phase::Atomic | Phase::Branch | Phase::TailFree => {
                let mut phase = match self.phase {
                    Phase::Sentinel => SavedTextPhase::Sentinel,
                    Phase::Atomic => SavedTextPhase::Atomic,
                    Phase::Branch => SavedTextPhase::Branch,
                    Phase::TailFree => SavedTextPhase::Free,
                    _ => unreachable!(),
                };
                let mut result = self.result.map(|result| match result {
                    Retirement::StaticSentinel => SavedTextResult::StaticSentinel,
                    Retirement::ReferenceDecremented => SavedTextResult::ReferenceDecremented,
                    Retirement::FreeTailReturned => SavedTextResult::FreeReturned,
                });
                let returned = step_saved_text(&mut TailServices { services: &mut self.services },
                    self.header.ok_or(Fault::MissingState)?, self.source_image_base,
                    &mut self.prior, &mut result, &mut phase);
                self.phase = match phase {
                    SavedTextPhase::Sentinel => Phase::Sentinel,
                    SavedTextPhase::Atomic => Phase::Atomic,
                    SavedTextPhase::Branch => Phase::Branch,
                    SavedTextPhase::Free => Phase::TailFree,
                    SavedTextPhase::Done => Phase::Done,
                };
                self.result = result.map(|result| match result {
                    SavedTextResult::StaticSentinel => Retirement::StaticSentinel,
                    SavedTextResult::ReferenceDecremented => Retirement::ReferenceDecremented,
                    SavedTextResult::FreeReturned => Retirement::FreeTailReturned,
                });
                returned.map_err(|fault| match fault {
                    SavedTextFault::Service(error) => Fault::Service(error),
                    SavedTextFault::MissingPrior | SavedTextFault::MissingResult => Fault::MissingState,
                })?;
            }
            Phase::Done => {}
        }
        Ok(())
    }
}
