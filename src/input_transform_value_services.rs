use std::collections::TryReserveError;
use crate::{backend_factory::SourceFloatFactory, conditional_gate::Batch,
    input_transform_4a640::{self as control, Constant, Field32, Field64, HeaderWord, Operation, Reply},
    phase_reallocation::{PhaseAllocationError, PhaseReallocationPlan},
    row_prepare_values, value_row_allocation};

pub trait Providers {
    type Error;
    fn scratch_predicate(&mut self, argument: u32, scratch: Option<&[f32]>) -> Result<u8, Self::Error>;
    fn first_index(&mut self, mode: u32) -> Result<u32, Self::Error>;
    fn second_index(&mut self, mode: u32, width: u32) -> Result<u32, Self::Error>;
    fn constant(&mut self, constant: Constant) -> Result<u32, Self::Error>;
    fn logarithm(&mut self, input: u32) -> Result<u32, Self::Error>;
    fn multiply(&mut self, destination: u32, source: u32) -> Result<u32, Self::Error>;
    fn add(&mut self, destination: u32, source: u32) -> Result<u32, Self::Error>;
    fn transform_in_place(&mut self, receiver: u64, row: &mut [f32], flag: u8) -> Result<(), Self::Error>;
    fn update_history(&mut self, header: u64, history: &mut [u32], input: u32) -> Result<(), Self::Error>;

    fn copy_unequal(&mut self, source: &[f32], destination: &mut Vec<f32>) -> Result<(), Self::Error>;

    fn stack_fill_byte(&mut self) -> Result<u8, Self::Error>;
}

pub struct KeyedRow { pub key: u64, pub values: Vec<f32> }

pub struct Keys {
    pub input_begin: u64, pub output_begin: u64,
    pub history_begin: u64, pub history_capacity_words: usize,
    pub factory: u64, pub transform: u64, pub history_header: u64,
    pub scratch_member: u64, pub checkout_temporary: u64,
    pub input_rows: Vec<u64>, pub output_rows: Vec<u64>,
    pub allocation_rows: Vec<u64>,
}
pub struct ExplicitParts<P> {
    pub input: Batch, pub output: Batch, pub scratch: Option<KeyedRow>,
    pub checkout: Option<KeyedRow>, pub history: Vec<u32>, pub history_count: usize,
    pub output_width: u32, pub transform_width: u32, pub mode: u32,
    pub history_width: i32, pub flag7b: u8,
    pub factory: SourceFloatFactory, pub allocation_word: u32, pub history_spare_word: u32,
    pub keys: Keys, pub providers: P,
}
#[derive(Clone, Copy, Debug)]

pub enum AdmissionError { Width, Count, HistoryExtent, Keys, AddressExtent, Overlap }
pub struct AdmissionFailure<P> { pub cause: AdmissionError, pub parts: ExplicitParts<P> }
pub enum Error<E> {
    PriorFailure, Admission(AdmissionError), Capacity(TryReserveError),
    Allocation(value_row_allocation::LogicalError), Provider(E),
    Key(u64), Address(u64), Index { index: usize, width: usize },
    Operation, MissingCheckout, MissingDonor, DisplacedOwner,
    AllocationKeysExhausted, HistoryPlan(PhaseAllocationError),
    CopyWidth { expected: usize, actual: usize },

    RequiredHistoryFill { byte: u8 },
    HistoryApplication { cause: PhaseAllocationError },
}
pub struct RetiredRow { key: u64, row: Vec<f32> }
impl RetiredRow {
    pub fn key(&self) -> u64 { self.key }
    pub fn values(&self) -> &[f32] { &self.row }
}
pub struct OwnedValueServices<P> {
    parts: ExplicitParts<P>, retired: Vec<RetiredRow>, retired_history: Vec<Vec<u32>>,
    donor: Option<KeyedRow>, displaced: Option<KeyedRow>,
    pending_history: Option<Vec<u32>>, allocation_cursor: usize,
    halted: bool, armed: bool, attempt: Option<(u32, Operation)>, last_fill_byte: Option<u8>,
}
pub struct NormalParts<P> {
    pub actual: ExplicitParts<P>, pub retired: Vec<RetiredRow>,
    pub retired_history: Vec<Vec<u32>>, pub allocation_cursor: usize,

    pub last_fill_byte: Option<u8>,
    pub donor: Option<KeyedRow>, pub displaced: Option<KeyedRow>,
    pub pending_history: Option<Vec<u32>>,
}

pub struct CompletedValue<P: Providers> { services: OwnedValueServices<P>, trace: control::Trace }
impl<P: Providers> CompletedValue<P> {
    pub fn output(&self) -> &Batch { &self.services.parts.output }
    pub fn trace(&self) -> &control::Trace { &self.trace }
    pub fn into_parts(self) -> (NormalParts<P>, control::Trace) {
        let s = self.services;
        (NormalParts { actual: s.parts, retired: s.retired, retired_history: s.retired_history,
            allocation_cursor: s.allocation_cursor, last_fill_byte: s.last_fill_byte,
            donor: s.donor, displaced: s.displaced, pending_history: s.pending_history }, self.trace)
    }
}

impl<P> OwnedValueServices<P> {
    pub fn from_explicit_parts(parts: ExplicitParts<P>) -> Result<Self, AdmissionFailure<P>> {
        if let Err(cause) = admit(&parts) { return Err(AdmissionFailure { cause, parts }); }
        Ok(Self { parts, retired: Vec::new(), retired_history: Vec::new(), donor: None,
            displaced: None, pending_history: None, allocation_cursor: 0, halted: false, armed: false,
            attempt: None, last_fill_byte: None })
    }
    pub fn input(&self) -> &Batch { &self.parts.input }
    pub fn output(&self) -> &Batch { &self.parts.output }
    pub fn factory(&self) -> SourceFloatFactory { self.parts.factory }
    pub fn providers(&self) -> &P { &self.parts.providers }
    pub fn scratch(&self) -> Option<&KeyedRow> { self.parts.scratch.as_ref() }
    pub fn checkout(&self) -> Option<&KeyedRow> { self.parts.checkout.as_ref() }
    pub fn donor(&self) -> Option<&KeyedRow> { self.donor.as_ref() }
    pub fn displaced(&self) -> Option<&KeyedRow> { self.displaced.as_ref() }
    pub fn history(&self) -> &[u32] { &self.parts.history }
    pub fn history_count(&self) -> usize { self.parts.history_count }
    pub fn retired(&self) -> &[RetiredRow] { &self.retired }
    pub fn pending_history(&self) -> Option<&[u32]> { self.pending_history.as_deref() }
    pub fn retired_history(&self) -> &[Vec<u32>] { &self.retired_history }
    pub fn halted(&self) -> bool { self.halted }
    pub fn attempt(&self) -> Option<(u32, Operation)> { self.attempt }

    pub fn last_fill_byte(&self) -> Option<u8> { self.last_fill_byte }
}
impl<P: Providers> OwnedValueServices<P> {
    pub fn run(mut self, budget: usize) -> Result<CompletedValue<P>, control::Failure<Self>> {
        self.armed = true;
        let completed = control::run(self, budget)?;
        let (mut services, trace) = completed.into_parts();
        services.armed = false;
        Ok(CompletedValue { services, trace })
    }
    fn create_donor(&mut self, width: usize) -> Result<(), Error<P::Error>> {
        if self.donor.is_some() { return Err(Error::MissingDonor); }
        let key = *self.parts.keys.allocation_rows.get(self.allocation_cursor)
            .ok_or(Error::AllocationKeysExhausted)?;
        let values = value_row_allocation::allocate_logical_row(width, self.parts.allocation_word)
            .map_err(Error::Allocation)?;
        self.donor = Some(KeyedRow { key, values });
        self.allocation_cursor += 1;
        Ok(())
    }
    fn retired_key(&self, key: u64) -> bool { self.retired.iter().any(|row| row.key == key) }
    fn row(&self, key: u64) -> Result<&[f32], Error<P::Error>> {
        if self.retired_key(key) { return Err(Error::Key(key)); }
        if let Some(index) = self.parts.keys.input_rows.iter().position(|&k| k == key) {
            return Ok(&self.parts.input.rows[index]);
        }
        if let Some(index) = self.parts.keys.output_rows.iter().position(|&k| k == key) {
            return Ok(&self.parts.output.rows[index]);
        }
        for row in [&self.parts.scratch, &self.parts.checkout, &self.donor, &self.displaced] {
            if let Some(row) = row { if row.key == key { return Ok(&row.values); } }
        }
        Err(Error::Key(key))
    }
    fn set(&mut self, key: u64, index: usize, bits: u32) -> Result<(), Error<P::Error>> {
        if self.retired_key(key) { return Err(Error::Key(key)); }
        let values = if let Some(i) = self.parts.keys.output_rows.iter().position(|&k| k == key) {
            &mut self.parts.output.rows[i]
        } else if let Some(row) = self.parts.scratch.as_mut().filter(|row| row.key == key) {
            &mut row.values
        } else { return Err(Error::Key(key)); };
        let width = values.len();
        *values.get_mut(index).ok_or(Error::Index { index, width })? = f32::from_bits(bits);
        Ok(())
    }
    fn retire(&mut self, key: u64) -> Result<(), Error<P::Error>> {
        if self.retired_key(key) { return Err(Error::Key(key)); }
        self.retired.try_reserve(1).map_err(Error::Capacity)?;
        let row = if let Some(index) = self.parts.keys.output_rows.iter().position(|&k| k == key) {

            std::mem::take(&mut self.parts.output.rows[index])
        } else if self.displaced.as_ref().is_some_and(|row| row.key == key) {
            self.displaced.take().expect("retained displaced owner").values
        } else if self.parts.checkout.as_ref().is_some_and(|row| row.key == key) {
            self.parts.checkout.take().expect("retained checkout owner").values
        } else { return Err(Error::Key(key)); };
        self.retired.push(RetiredRow { key, row });
        Ok(())
    }
    fn copy(&mut self, source: u64, destination: u64) -> Result<(), Error<P::Error>> {
        if self.retired_key(source) || self.retired_key(destination) { return Err(Error::Key(destination)); }
        if let Some(index) = self.parts.keys.input_rows.iter().position(|&key| key == source) {
            let scratch = self.parts.scratch.as_mut().filter(|row| row.key == destination)
                .ok_or(Error::Key(destination))?;
            return copy_rows(&mut self.parts.providers, &self.parts.input.rows[index], &mut scratch.values);
        }
        let scratch = self.parts.scratch.as_ref().filter(|row| row.key == source).ok_or(Error::Key(source))?;
        let index = self.parts.keys.output_rows.iter().position(|&key| key == destination)
            .ok_or(Error::Key(destination))?;
        copy_rows(&mut self.parts.providers, &scratch.values, &mut self.parts.output.rows[index])
    }
    fn grow_history(&mut self, count: usize) -> Result<(), Error<P::Error>> {

        let plan = PhaseReallocationPlan::bind(self.parts.history_count, self.parts.history.len(), count)
            .map_err(Error::HistoryPlan)?;
        if plan.new_capacity() > self.parts.keys.history_capacity_words {
            return Err(Error::Admission(AdmissionError::HistoryExtent));
        }
        self.pending_history = Some(plan.allocate_with_explicit_spare_word(self.parts.history_spare_word)
            .map_err(Error::Capacity)?);
        self.retired_history.try_reserve(1).map_err(Error::Capacity)?;
        let allocation = self.pending_history.take().expect("retained history allocation");
        let allocation = match plan.apply_retaining(&self.parts.history, allocation) {
            Ok(allocation) => allocation.into_vec(),
            Err((cause, allocation)) => {
                self.pending_history = Some(allocation);
                return Err(Error::HistoryApplication { cause });
            }
        };
        let old = std::mem::replace(&mut self.parts.history, allocation);
        self.parts.history_count = count;
        self.retired_history.push(old);
        Ok(())
    }
    fn effect(&mut self, operation: Operation) -> Result<Reply, Error<P::Error>> {
        let k = &self.parts.keys;
        match operation {
            Operation::Field32(field) => Ok(Reply::Word32(match field {
                Field32::OutputWidth50 => self.parts.output_width, Field32::Mode58 => self.parts.mode,
                Field32::TransformWidth64 => self.parts.transform_width,
                Field32::HistoryCount74 => self.parts.history_width as u32,
            })),
            Operation::Field64(field) => Ok(Reply::Word64(match field {
                Field64::Factory48 => k.factory, Field64::Transform90 => k.transform,
                Field64::ScratchB0 => self.parts.scratch.as_ref().map_or(0, |row| row.key),
                Field64::HistoryBeginB8 => k.history_begin,
                Field64::HistoryEndC0 => end(k.history_begin, self.parts.history_count, 4).ok_or(Error::Operation)?,
                Field64::HistoryCapacityC8 => end(k.history_begin, self.parts.history.len(), 4).ok_or(Error::Operation)?,
            })),
            Operation::Header(word) => Ok(Reply::Word64(match word {
                HeaderWord::InputBegin => k.input_begin,
                HeaderWord::InputEnd => end(k.input_begin, self.parts.input.rows.len(), 8).ok_or(Error::Operation)?,
                HeaderWord::OutputBegin => k.output_begin,
                HeaderWord::OutputEnd => end(k.output_begin, self.parts.output.rows.len(), 8).ok_or(Error::Operation)?,
            })),
            Operation::InputStatus => Ok(Reply::Byte(self.parts.input.status)),
            Operation::Read64 { address } => {
                if let Some(i) = slot(address, k.input_begin, self.parts.input.rows.len(), 8) {
                    return Ok(Reply::Word64(k.input_rows[i]));
                }
                if let Some(i) = slot(address, k.output_begin, self.parts.output.rows.len(), 8) {
                    return Ok(Reply::Word64(k.output_rows[i]));
                }
                Err(Error::Address(address))
            }
            Operation::Read32 { address } => {
                let i = slot(address, k.history_begin, self.parts.history_count, 4).ok_or(Error::Address(address))?;
                Ok(Reply::Word32(self.parts.history[i]))
            }
            Operation::PrepareOutput30bc0 { factory, count, width } => {
                if factory != k.factory || count != self.parts.input.rows.len() as u64 || width != self.parts.output_width {
                    return Err(Error::Operation);
                }
                row_prepare_values::prepare(&mut Preparation { owner: self }, count as usize, width as usize)?;
                Ok(Reply::Unit)
            }
            Operation::ScratchPredicate33260 { argument } => self.parts.providers
                .scratch_predicate(argument, self.parts.scratch.as_ref().map(|row| row.values.as_slice()))
                .map(Reply::Byte).map_err(Error::Provider),
            Operation::FactoryCheckoutV28 { factory, width, r9, fifth_byte } => {
                if factory != k.factory || width != self.parts.transform_width || r9 != 0 || fifth_byte != 0
                    || self.parts.checkout.is_some() { return Err(Error::Operation); }
                self.create_donor(width as usize)?;
                self.parts.checkout = self.donor.take();
                Ok(Reply::Unit)
            }
            Operation::ScratchMemberIsCheckoutTemporary => Ok(Reply::Predicate(k.scratch_member == k.checkout_temporary)),
            Operation::ReadCheckoutTemporary => Ok(Reply::Word64(self.parts.checkout.as_ref().map_or(0, |row| row.key))),
            Operation::ClearCheckoutTemporary => {
                if self.donor.is_some() { return Err(Error::MissingDonor); }
                self.donor = self.parts.checkout.take();
                Ok(Reply::Unit)
            }
            Operation::SetScratchB0 { word } => {
                if self.displaced.is_some() { return Err(Error::DisplacedOwner); }
                if !self.donor.as_ref().is_some_and(|row| row.key == word) { return Err(Error::MissingDonor); }
                self.displaced = self.parts.scratch.take();
                self.parts.scratch = self.donor.take();
                Ok(Reply::Unit)
            }
            Operation::RetireV0 { row, flag } => {
                if flag != 1 { return Err(Error::Operation); }
                self.retire(row)?; Ok(Reply::Unit)
            }
            Operation::SetOutputEnd { word } => {
                if word != k.output_begin || !k.output_rows.iter().all(|&key| self.retired_key(key)) {
                    return Err(Error::Operation);
                }
                self.parts.output.rows.clear(); self.parts.keys.output_rows.clear(); Ok(Reply::Unit)
            }
            Operation::SetOutputStatus { byte } => { self.parts.output.status = byte; Ok(Reply::Unit) }
            Operation::SetHistoryEndC0 { word } => {
                let count = extent(word, k.history_begin, 4).ok_or(Error::Address(word))?;
                if count > self.parts.history.len() { return Err(Error::Address(word)); }
                self.parts.history_count = count; Ok(Reply::Unit)
            }
            Operation::ZeroFill8e20 { begin, end: limit } => {
                let first = extent(begin, k.history_begin, 4).ok_or(Error::Address(begin))?;
                let last = extent(limit, k.history_begin, 4).ok_or(Error::Address(limit))?;
                if first != self.parts.history_count || first > last || last > self.parts.history.len() {
                    return Err(Error::Operation);
                }
                self.parts.history[first..last].fill(0); Ok(Reply::Unit)
            }
            Operation::GrowHistory1fe50 { count } => {
                let count = usize::try_from(count).map_err(|_| Error::Operation)?;
                self.grow_history(count)?; Ok(Reply::Unit)
            }
            Operation::Helper49080 { mode } => self.parts.providers.first_index(mode).map(Reply::Word32).map_err(Error::Provider),
            Operation::Helper49090 { mode, width } => self.parts.providers.second_index(mode, width).map(Reply::Word32).map_err(Error::Provider),
            Operation::Constant(constant) => self.parts.providers.constant(constant).map(Reply::Word32).map_err(Error::Provider),
            Operation::Flag7b => Ok(Reply::Byte(self.parts.flag7b)),
            Operation::CopyV48 { source, destination } => { self.copy(source, destination)?; Ok(Reply::Unit) }
            Operation::GetV20 { row, index } => {
                let values = self.row(row)?;
                values.get(index as usize).map(|value| Reply::Word32(value.to_bits()))
                    .ok_or(Error::Index { index: index as usize, width: values.len() })
            }
            Operation::SetV28 { row, index, bits } => { self.set(row, index as usize, bits)?; Ok(Reply::Unit) }
            Operation::WidthV18 { row } => Ok(Reply::Word32(self.row(row)?.len() as u32)),
            Operation::Helper5834c0 { input } => self.parts.providers.logarithm(input).map(Reply::Word32).map_err(Error::Provider),
            Operation::TransformV10 { receiver, input, output, flag } => {
                if receiver != k.transform || input != output { return Err(Error::Operation); }
                let row = self.parts.scratch.as_mut().filter(|row| row.key == input).ok_or(Error::Key(input))?;
                self.parts.providers.transform_in_place(receiver, &mut row.values, flag).map_err(Error::Provider)?;
                Ok(Reply::Unit)
            }
            Operation::Mulss { destination, source } => self.parts.providers.multiply(destination, source).map(Reply::Word32).map_err(Error::Provider),
            Operation::Addss { destination, source } => self.parts.providers.add(destination, source).map(Reply::Word32).map_err(Error::Provider),
            Operation::Helper3d7f0 { input } => {
                self.parts.providers.update_history(k.history_header,
                    &mut self.parts.history[..self.parts.history_count], input).map_err(Error::Provider)?;
                Ok(Reply::Unit)
            }
        }
    }
}

impl<P: Providers> control::Services for OwnedValueServices<P> {
    type Error = Error<P::Error>;
    type Output = Batch;
    fn perform(&mut self, pc: u32, operation: Operation) -> Result<Reply, Self::Error> {
        if self.halted || !self.armed {
            self.halted = true;
            return Err(Error::PriorFailure);
        }

        self.halted = true;
        self.attempt = Some((pc, operation));
        let reply = self.effect(operation)?;
        self.halted = false;
        Ok(reply)
    }
    fn output(&self) -> &Batch { &self.parts.output }
}
struct Preparation<'a, P> { owner: &'a mut OwnedValueServices<P> }
impl<P: Providers> row_prepare_values::Services for Preparation<'_, P> {
    type Error = Error<P::Error>;
    fn current_count(&self) -> usize { self.owner.parts.output.rows.len() }
    fn row_matches(&self, index: usize, width: usize) -> bool {
        self.owner.parts.output.rows.get(index).is_some_and(|row| row.len() == width)
    }
    fn retire_current_back(&mut self) -> Result<(), Self::Error> {
        let s = &mut self.owner;
        s.retired.try_reserve(1).map_err(Error::Capacity)?;
        let key = s.parts.keys.output_rows.pop().ok_or(Error::Operation)?;
        let row = s.parts.output.rows.pop().expect("admitted output/header count");
        s.retired.push(RetiredRow { key, row }); Ok(())
    }
    fn create_admit_install(&mut self, index: usize, width: usize) -> Result<(), Self::Error> {
        let s = &mut self.owner;
        s.create_donor(width)?;
        if index > s.parts.output.rows.len() { return Err(Error::Operation); }
        if index == s.parts.output.rows.len() {
            s.parts.output.rows.try_reserve(1).map_err(Error::Capacity)?;
            s.parts.keys.output_rows.try_reserve(1).map_err(Error::Capacity)?;
            let donor = s.donor.take().expect("retained allocation donor");
            s.parts.output.rows.push(donor.values); s.parts.keys.output_rows.push(donor.key);
        } else {
            if s.displaced.is_some() { return Err(Error::DisplacedOwner); }
            let donor = s.donor.take().expect("retained allocation donor");
            let key = std::mem::replace(&mut s.parts.keys.output_rows[index], donor.key);
            let values = std::mem::replace(&mut s.parts.output.rows[index], donor.values);
            s.displaced = Some(KeyedRow { key, values });
            s.retire(key)?;
        }
        Ok(())
    }
}

fn copy_rows<P: Providers>(providers: &mut P, source: &[f32], destination: &mut Vec<f32>) -> Result<(), Error<P::Error>> {
    if source.len() == destination.len() { destination.copy_from_slice(source); Ok(()) }
    else {
        providers.copy_unequal(source, destination).map_err(Error::Provider)?;
        if destination.len() != source.len() {
            return Err(Error::CopyWidth { expected: source.len(), actual: destination.len() });
        }
        Ok(())
    }
}
fn end(begin: u64, count: usize, stride: u64) -> Option<u64> {
    begin.checked_add(u64::try_from(count).ok()?.checked_mul(stride)?)
}
fn extent(address: u64, begin: u64, stride: u64) -> Option<usize> {
    let bytes = address.checked_sub(begin)?;
    if bytes % stride != 0 { return None; }
    usize::try_from(bytes / stride).ok()
}
fn slot(address: u64, begin: u64, count: usize, stride: u64) -> Option<usize> {
    extent(address, begin, stride).filter(|&index| index < count)
}
fn admit<P>(p: &ExplicitParts<P>) -> Result<(), AdmissionError> {
    let k = &p.keys;
    let valid_width = |width: usize| width != 0 && width <= i32::MAX as usize;
    if !valid_width(p.output_width as usize) || !valid_width(p.transform_width as usize)
        || p.input.rows.iter().chain(&p.output.rows).any(|row| !valid_width(row.len()))
        || [&p.scratch, &p.checkout].iter().any(|row| row.as_ref().is_some_and(|row| !valid_width(row.values.len()))) {
        return Err(AdmissionError::Width);
    }
    if p.input.rows.len() > i32::MAX as usize || p.output.rows.len() > i32::MAX as usize
        || k.input_rows.len() != p.input.rows.len() || k.output_rows.len() != p.output.rows.len() {
        return Err(AdmissionError::Count);
    }
    if p.history_width < 0 || p.history_count > p.history.len() || p.history.len() > k.history_capacity_words {
        return Err(AdmissionError::HistoryExtent);
    }
    let needed = p.history_width as usize;
    if needed > p.history.len() {
        let plan = PhaseReallocationPlan::bind(p.history_count, p.history.len(), needed)
            .map_err(|_| AdmissionError::HistoryExtent)?;
        if plan.new_capacity() > k.history_capacity_words { return Err(AdmissionError::HistoryExtent); }
    }
    let ranges = [
        (k.input_begin, end(k.input_begin, p.input.rows.len(), 8)),
        (k.output_begin, end(k.output_begin, p.input.rows.len().max(p.output.rows.len()), 8)),
        (k.history_begin, end(k.history_begin, k.history_capacity_words, 4)),
    ];
    for (i, &(begin, limit)) in ranges.iter().enumerate() {
        let limit = limit.ok_or(AdmissionError::AddressExtent)?;
        if begin == 0 || begin % 8 != 0 || limit - begin > i64::MAX as u64 { return Err(AdmissionError::AddressExtent); }
        for &(other, other_limit) in &ranges[..i] {
            let other_limit = other_limit.ok_or(AdmissionError::AddressExtent)?;
            if begin == other || (begin < other_limit && other < limit) { return Err(AdmissionError::Overlap); }
        }
    }
    let fixed = [k.factory, k.transform, k.history_header, k.scratch_member, k.checkout_temporary];
    let extra = [p.scratch.as_ref().map(|row| row.key), p.checkout.as_ref().map(|row| row.key)];
    let keys = || fixed.iter().copied().chain(k.input_rows.iter().copied())
        .chain(k.output_rows.iter().copied()).chain(k.allocation_rows.iter().copied()).chain(extra.iter().filter_map(|key| *key));

    for (i, key) in keys().enumerate() {
        if key == 0 || keys().take(i).any(|earlier| earlier == key) { return Err(AdmissionError::Keys); }
        for &(begin, limit) in &ranges {
            if key >= begin && key < limit.ok_or(AdmissionError::AddressExtent)? { return Err(AdmissionError::Overlap); }
        }
    }
    Ok(())
}
