#[path = "text_live_comparison.rs"]
pub mod live_comparison;

pub(super) fn split_comparison_value_prefix<S: live_comparison::Services>(
    required: live_comparison::ReturnRequired<S>,
) -> (S, live_comparison::Input<S::Owners>, live_comparison::Entry, live_comparison::Captures) {
    let p = required.into_parts();
    (p.services, p.input, p.entry, p.captures)
}
#[path = "text_fragment_count_borrowed.rs"]
pub mod fragment_count_borrowed;

pub trait Services {
    type Node;
    type Input;

    type Text: Clone + Eq;
    type Event;
    type Error;
    fn input_pointer(&mut self, input: &Self::Input) -> Result<Self::Text, Self::Error>;
    fn node_name_pointer(&mut self, node: &Self::Node) -> Result<Self::Text, Self::Error>;

    fn compare_text(&mut self, old: &Self::Text, incoming: &Self::Text) -> Result<i32, Self::Error>;

    fn is_empty_sentinel(&mut self, text: &Self::Text) -> Result<bool, Self::Error>;

    fn retain_reference(&mut self, text: &Self::Text) -> Result<(), Self::Error>;

    fn exchange_node_name(&mut self, node: &Self::Node, incoming: &Self::Text,
        old: &mut Option<Self::Text>) -> Result<(), Self::Error>;

    fn release_reference(&mut self, held: &mut Option<Self::Text>) -> Result<(), Self::Error>;

    fn prepare_name_event(&mut self, held: &mut Option<Self::Event>) -> Result<(), Self::Error>;

    fn dispatch_name_event(&mut self, node: &Self::Node,
        held: &mut Option<Self::Event>) -> Result<(), Self::Error>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase { Read, Compare, Retain, Exchange, ReleaseOld, AllocateEvent, Event, RereadInput, ReleaseInput, Done }
#[derive(Debug)]
pub enum Fault<E> { Service(E), MissingOwner, UnconsumedReference, UnconsumedEvent, AlreadyStopped }

pub struct Prepared<S: Services> {
    services: S,
    node: S::Node,
    input: S::Input,
    incoming: Option<S::Text>,
    compared_old: Option<S::Text>,
    old: Option<S::Text>,
    event: Option<S::Event>,
    changed: bool,
    phase: Phase,
    attempted: Option<Phase>,
    halted: bool,
}
pub struct Stopped<S: Services> { pub pending: Prepared<S>, pub fault: Fault<S::Error> }

pub struct RetiredInput<I> { input: I }
impl<I> RetiredInput<I> {

    pub fn captured_input(&self) -> &I { &self.input }
}
pub struct Finished<S: Services> {
    pub services: S, pub node: S::Node, pub retired_input: RetiredInput<S::Input>, pub changed: bool,
}
pub enum Poll<S: Services> {
    Pending(Prepared<S>), Ready(Finished<S>), Failed(Stopped<S>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side { Left, Right }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TextReadBoundary { pub side: Side, pub cursor: usize, pub needed_byte: usize }

pub fn compare_source_utf8(left: &[u8], right: &[u8]) -> Result<i32, TextReadBoundary> {
    let mut a = 0;
    let mut b = 0;
    loop {
        let lhs = decode_source_character(left, &mut a, Side::Left)?;
        let rhs = decode_source_character(right, &mut b, Side::Right)?;
        if lhs != rhs { return Ok(if lhs < rhs { -1 } else { 1 }); }
        if lhs == 0 { return Ok(0); }
    }
}

pub trait FoldServices {
    type Error;
    fn uppercase_58a19c(&mut self, actual_call_pc: u32, input_ecx: u32)
        -> Result<u16, Self::Error>;
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FoldPhase { DecodeLeft, DecodeRight, UpperRight, UpperLeft }
#[derive(Clone, Copy, Debug, Default)]
pub struct FoldPrefix {
    pub left_cursor: usize, pub right_cursor: usize,
    pub left: Option<u32>, pub right: Option<u32>,
    pub upper_right: Option<u16>, pub upper_left: Option<u16>,
}
fn folded_order(left: u16, right: u16) -> i32 {
    if left == right { 0 } else if left < right { -1 } else { 1 }
}
#[derive(Debug)]
pub enum FoldFault<E> { Read(TextReadBoundary), Required(E) }
#[derive(Debug)]
pub struct FoldFailure<E> {
    pub fault: FoldFault<E>, pub attempted: FoldPhase, pub prefix: FoldPrefix,
}

pub fn compare_source_utf8_folded<S: FoldServices>(services: &mut S,
    left: &[u8], right: &[u8]) -> Result<i32, FoldFailure<S::Error>>
{
    let mut prefix = FoldPrefix::default();
    loop {
        prefix.left = None; prefix.right = None;
        prefix.upper_right = None; prefix.upper_left = None;
        let lhs = decode_source_character(left, &mut prefix.left_cursor, Side::Left)
            .map_err(|error| FoldFailure { fault: FoldFault::Read(error),
                attempted: FoldPhase::DecodeLeft, prefix })?;
        prefix.left = Some(lhs);
        let rhs = decode_source_character(right, &mut prefix.right_cursor, Side::Right)
            .map_err(|error| FoldFailure { fault: FoldFault::Read(error),
                attempted: FoldPhase::DecodeRight, prefix })?;
        prefix.right = Some(rhs);
        let difference = if lhs == rhs { 0 } else {

            let upper_right = services.uppercase_58a19c(0x1fc14a, rhs)
                .map_err(|error| FoldFailure { fault: FoldFault::Required(error),
                    attempted: FoldPhase::UpperRight, prefix })?;
            prefix.upper_right = Some(upper_right);
            let upper_left = services.uppercase_58a19c(0x1fc154, lhs)
                .map_err(|error| FoldFailure { fault: FoldFault::Required(error),
                    attempted: FoldPhase::UpperLeft, prefix })?;
            prefix.upper_left = Some(upper_left);
            folded_order(upper_left, upper_right)
        };

        if difference != 0 { return Ok(difference); }
        if lhs == 0 { return Ok(0); }
    }
}

pub fn source_fragment_encoded_byte_count(bytes: &[u8]) -> Result<u64, TextReadBoundary> {
    let mut memory = SpanMemory { bytes, side: Side::Left, initial: 0 };
    source_fragment_encoded_byte_count_from_memory(&mut memory, 0usize).map_err(|failure| failure.error)
}

pub struct FragmentCounter<A: CharacterAddress> {
    cursor: A, count: u64, last_character: Option<u32>,
    decoding: bool, stopped: bool,
}
pub struct FragmentCounted<A: CharacterAddress> { cursor: A, count: u64 }
impl<A: CharacterAddress> FragmentCounted<A> {
    pub fn cursor(&self) -> A { self.cursor }
    pub fn count(&self) -> u64 { self.count }
}
pub struct FragmentFailure<A: CharacterAddress, E> {
    pub pending: FragmentCounter<A>, pub error: E,
}
pub enum FragmentPoll<A: CharacterAddress, E> {
    Pending(FragmentCounter<A>), Ready(FragmentCounted<A>),
    Failed(FragmentFailure<A, E>),

    Stopped(FragmentCounter<A>),
}
impl<A: CharacterAddress> FragmentCounter<A> {
    pub fn from_actual_fragment(initial: A) -> Self {
        Self { cursor: initial, count: 0, last_character: None, decoding: false, stopped: false }
    }
    pub fn cursor(&self) -> A { self.cursor }
    pub fn count(&self) -> u64 { self.count }
    pub fn last_character(&self) -> Option<u32> { self.last_character }
    pub fn attempted_decode(&self) -> bool { self.decoding }

    pub fn poll<M: CharacterMemory<Address = A> + ?Sized>(mut self, memory: &mut M,
        budget: usize) -> FragmentPoll<A, M::Error> {
        if self.stopped { return FragmentPoll::Stopped(self); }
        for _ in 0..budget {
            match self.step(memory) {
                Ok(true) => return FragmentPoll::Ready(FragmentCounted { cursor: self.cursor, count: self.count }),
                Ok(false) => {},
                Err(error) => return FragmentPoll::Failed(FragmentFailure { pending: self, error }),
            }
        }
        FragmentPoll::Pending(self)
    }
    fn step<M: CharacterMemory<Address = A> + ?Sized>(&mut self, memory: &mut M)
        -> Result<bool, M::Error> {
        self.decoding = true;
        let character = match decode_from_memory(memory, &mut self.cursor) {
            Ok(value) => value,
            Err(error) => { self.stopped = true; return Err(error); },
        };
        self.last_character = Some(character); self.decoding = false;
        if character == 0 { return Ok(true); }
        self.count = fragment_count_add_width(self.count, encoded_character_width(character));
        Ok(false)
    }
}

pub fn encoded_character_width(character: u32) -> u64 {
    if character < 0x80 { 1 } else if character < 0x800 { 2 }
        else if character < 0x10000 { 3 } else { 4 }
}
pub fn fragment_count_add_width(count: u64, width: u64) -> u64 { count.wrapping_add(width) }
pub fn character_low_six(word: u32) -> u32 { word & 0x3f }
pub fn character_shift_six(word: u32) -> u32 { word.wrapping_shl(6) }
pub fn character_or(left: u32, right: u32) -> u32 { left | right }
pub fn character_lead_payload(lead: u8, mask: u8) -> u32 { u32::from(lead & mask) }

pub fn character_continuation_class(word: u32) -> u32 { word & 0xc0 }
pub fn character_is_continuation(byte: u8) -> bool { character_continuation_class(u32::from(byte)) == 0x80 }

pub fn source_fragment_encoded_byte_count_from_memory<M: CharacterMemory + ?Sized>(
    memory: &mut M, initial: M::Address) -> Result<u64, FragmentFailure<M::Address, M::Error>> {
    let mut pending = FragmentCounter::from_actual_fragment(initial);
    loop {
        match pending.step(memory) {
            Ok(true) => return Ok(pending.count),
            Ok(false) => {},
            Err(error) => return Err(FragmentFailure { pending, error }),
        }
    }
}

pub trait CharacterAddress: Copy {
    fn plus(self, offset: u8) -> Self;
}
impl CharacterAddress for usize {
    fn plus(self, offset: u8) -> Self { self.wrapping_add(usize::from(offset)) }
}
impl CharacterAddress for u64 {
    fn plus(self, offset: u8) -> Self { self.wrapping_add(u64::from(offset)) }
}

pub trait CharacterMemory {
    type Address: CharacterAddress;
    type Error;
    fn read_byte(&mut self, address: Self::Address) -> Result<u8, Self::Error>;

    fn progress(&mut self, _progress: CharacterProgress<Self::Address>) {}
}
#[derive(Clone, Copy, Debug)]
pub struct CharacterProgress<A> {
    pub initial: A, pub next_cursor: A,
    pub lead: Option<u8>, pub value: Option<u32>, pub accepted_continuations: u8,
}

pub fn decode_from_memory<M: CharacterMemory + ?Sized>(memory: &mut M, cursor: &mut M::Address)
    -> Result<u32, M::Error>
{
    let initial = *cursor;
    let mut progress = CharacterProgress { initial, next_cursor: initial,
        lead: None, value: None, accepted_continuations: 0 };
    memory.progress(progress);
    let lead = memory.read_byte(initial)?;
    *cursor = initial.plus(1);
    progress.lead = Some(lead); progress.next_cursor = *cursor;
    if lead & 0x80 == 0 {
        progress.value = Some(u32::from(lead)); memory.progress(progress);
        return Ok(u32::from(lead));
    }
    if lead & 0x40 == 0 {
        progress.value = Some(character_low_six(u32::from(lead))); memory.progress(progress);
        return Ok(character_low_six(u32::from(lead)));
    }

    let (count, mask) = if lead & 0x20 == 0 { (2, 0x3f) }
        else if lead & 0x10 == 0 { (3, 0x1f) } else { (4, 0x0f) };
    let mut value = character_lead_payload(lead, mask);
    progress.value = Some(value); memory.progress(progress);
    for offset in 1..count {
        let next = memory.read_byte(initial.plus(offset))?;
        if !character_is_continuation(next) { return Ok(value); }
        value = character_or(character_shift_six(value), character_low_six(u32::from(next)));
        *cursor = (*cursor).plus(1);
        progress.value = Some(value); progress.next_cursor = *cursor;
        progress.accepted_continuations += 1; memory.progress(progress);
    }
    Ok(value)
}
struct SpanMemory<'a> { bytes: &'a [u8], side: Side, initial: usize }
impl CharacterMemory for SpanMemory<'_> {
    type Address = usize;
    type Error = TextReadBoundary;
    fn read_byte(&mut self, address: usize) -> Result<u8, Self::Error> {
        self.bytes.get(address).copied().ok_or(TextReadBoundary {
            side: self.side, cursor: self.initial, needed_byte: address })
    }
    fn progress(&mut self, progress: CharacterProgress<usize>) {
        self.initial = progress.initial;
    }
}
fn decode_source_character(bytes: &[u8], cursor: &mut usize, side: Side)
    -> Result<u32, TextReadBoundary>
{
    let mut memory = SpanMemory { bytes, side, initial: *cursor };
    decode_from_memory(&mut memory, cursor)
}

impl<S: Services> Prepared<S> {
    pub fn from_source_setter(services: S, node: S::Node, incoming_by_value: S::Input) -> Self {
        Self { services, node, input: incoming_by_value, incoming: None, compared_old: None,
            old: None, event: None, changed: false, phase: Phase::Read,
            attempted: None, halted: false }
    }
    pub fn phase(&self) -> Phase { self.phase }
    pub fn attempted(&self) -> Option<Phase> { self.attempted }
    pub fn services(&self) -> &S { &self.services }
    pub fn node(&self) -> &S::Node { &self.node }
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
        Poll::Ready(Finished { services: self.services, node: self.node,
            retired_input: RetiredInput { input: self.input }, changed: self.changed })
    }
    fn step(&mut self) -> Result<(), Fault<S::Error>> {
        match self.phase {
            Phase::Read => {

                self.incoming = Some(self.services.input_pointer(&self.input).map_err(Fault::Service)?);
                self.compared_old = Some(self.services.node_name_pointer(&self.node).map_err(Fault::Service)?);
                self.phase = if self.incoming == self.compared_old { Phase::ReleaseInput } else { Phase::Compare };
            }
            Phase::Compare => {
                let old = self.compared_old.as_ref().ok_or(Fault::MissingOwner)?;
                let incoming = self.incoming.as_ref().ok_or(Fault::MissingOwner)?;
                let comparison = self.services.compare_text(old, incoming).map_err(Fault::Service)?;

                self.incoming = Some(self.services.input_pointer(&self.input).map_err(Fault::Service)?);
                self.phase = if comparison == 0 { Phase::ReleaseInput } else { Phase::Retain };
            }
            Phase::Retain => {
                let incoming = self.incoming.as_ref().ok_or(Fault::MissingOwner)?;
                if !self.services.is_empty_sentinel(incoming).map_err(Fault::Service)? {
                    self.services.retain_reference(incoming).map_err(Fault::Service)?;

                    self.incoming = Some(self.services.input_pointer(&self.input).map_err(Fault::Service)?);
                }
                self.phase = Phase::Exchange;
            }
            Phase::Exchange => {
                let incoming = self.incoming.as_ref().ok_or(Fault::MissingOwner)?;
                self.services.exchange_node_name(&self.node, incoming, &mut self.old).map_err(Fault::Service)?;
                if self.old.is_none() { return Err(Fault::MissingOwner); }
                self.changed = true;
                self.phase = Phase::ReleaseOld;
            }
            Phase::ReleaseOld => {
                self.services.release_reference(&mut self.old).map_err(Fault::Service)?;
                if self.old.is_some() { return Err(Fault::UnconsumedReference); }
                self.phase = Phase::AllocateEvent;
            }
            Phase::AllocateEvent => {
                self.services.prepare_name_event(&mut self.event).map_err(Fault::Service)?;
                if self.event.is_none() { return Err(Fault::MissingOwner); }
                self.phase = Phase::Event;
            }
            Phase::Event => {
                self.phase = Phase::RereadInput;
                self.services.dispatch_name_event(&self.node, &mut self.event).map_err(Fault::Service)?;
                if self.event.is_some() { return Err(Fault::UnconsumedEvent); }
            }
            Phase::RereadInput => {

                self.incoming = Some(self.services.input_pointer(&self.input).map_err(Fault::Service)?);
                self.phase = Phase::ReleaseInput;
            }
            Phase::ReleaseInput => {
                self.phase = Phase::Done;
                self.services.release_reference(&mut self.incoming).map_err(Fault::Service)?;
                if self.incoming.is_some() { return Err(Fault::UnconsumedReference); }
            }
            Phase::Done => {}
        }
        Ok(())
    }
}
