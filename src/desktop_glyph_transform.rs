use super::{empty_background::literal::path::Path, glyph_fit, glyph_math, glyph_transform as graph};
#[path = "desktop_glyph_raster_owner.rs"] mod raster_owner;
pub(super) use self::raster_owner::{RasterCompleted, RasterFailure, RasterGraph};
const DESCRIPTOR: u64 = 0x1000;
const AFFINE: u64 = 0x2000;
const MARKERS: u64 = 0x3000;
const WORDS: u64 = 0x1_0000;
const MARKER_BITS: [u32; 4] = [0x47c35100, 0x47c35080, 0x47c35180, 0x47c35200];

pub(super) enum StartCause { Count, Allocation(std::collections::TryReserveError) }
pub(super) struct StartFailure { pub cause: StartCause, pub math: glyph_math::Provider }
pub(super) enum Cause { Address, Math(glyph_math::Stopped) }
pub(super) struct ServiceFailure { pub request: graph::Request, pub cause: Cause }
#[derive(Clone, Copy, Debug)]
pub(super) enum Stopped { Failed, AlreadyFailed }
pub(super) struct Prepared {
    path: Path, affine: [u32; 6], math: glyph_math::Provider, failure: Option<ServiceFailure>,
}
pub(super) struct Completed { owner: Prepared, progress: graph::Progress }
pub(super) struct Failure { owner: Prepared, pub progress: graph::Progress, pub cause: graph::Cause<Stopped> }
impl Failure {

    pub fn owner(&self) -> &Prepared { &self.owner }
}
impl Prepared {
    pub fn from_values(path: &Path, affine: [u32; 6], math: glyph_math::Provider) -> Result<Self, StartFailure> {
        if path.words.len() > i32::MAX as usize { return Err(StartFailure { cause: StartCause::Count, math }); }
        let mut words = Vec::new();
        if let Err(error) = words.try_reserve_exact(path.words.len()) {
            return Err(StartFailure { cause: StartCause::Allocation(error), math });
        }
        words.extend_from_slice(&path.words);
        Ok(Self { path: Path { words, source_capacity: path.source_capacity, bounds: path.bounds,
            source_flag_32: path.source_flag_32 }, affine, math, failure: None })
    }
    pub fn path(&self) -> &Path { &self.path }
    pub fn math(&self) -> &glyph_math::Provider { &self.math }
    pub fn failure(&self) -> Option<&ServiceFailure> { self.failure.as_ref() }
    pub fn run(mut self) -> Result<Completed, Failure> {
        let bindings = graph::Bindings { path_descriptor: DESCRIPTOR, affine: AFFINE,
            marker_addresses: [MARKERS, MARKERS + 4, MARKERS + 8, MARKERS + 12] };
        let result = {
            match graph::transform_with_services(&mut self, bindings) {
                Ok(completed) => Ok(completed.into_parts().1),
                Err(failure) => Err((failure.progress, failure.cause)),
            }
        };
        match result {
            Ok(progress) => Ok(Completed { owner: self, progress }),
            Err((progress, cause)) => Err(Failure { owner: self, progress, cause }),
        }
    }
    fn ready(&self) -> Result<(), Stopped> {
        if self.failure.is_some() { Err(Stopped::AlreadyFailed) } else { Ok(()) }
    }
    fn stop(&mut self, request: graph::Request, cause: Cause) -> Stopped {
        self.failure = Some(ServiceFailure { request, cause }); Stopped::Failed
    }
    fn index(address: u64, base: u64, count: usize) -> Option<usize> {
        let offset = address.checked_sub(base)?;
        if offset & 3 != 0 { return None; }
        let index = usize::try_from(offset / 4).ok()?;
        (index < count).then_some(index)
    }
}
impl Completed {
    pub fn path(&self) -> &Path { &self.owner.path }
    pub fn math(&self) -> &glyph_math::Provider { &self.owner.math }
    pub fn progress(&self) -> &graph::Progress { &self.progress }
}
impl graph::Services for Prepared {
    type Error = Stopped;
    fn read_u32(&mut self, pc: u32, address: u64) -> Result<u32, Stopped> {
        self.ready()?;
        if address == DESCRIPTOR + 12 { return Ok(self.path.words.len() as u32); }
        if let Some(index) = Self::index(address, DESCRIPTOR + 16, 4) { return Ok(self.path.bounds[index].to_bits()); }
        if let Some(index) = Self::index(address, AFFINE, 6) { return Ok(self.affine[index]); }
        if let Some(index) = Self::index(address, MARKERS, 4) { return Ok(MARKER_BITS[index]); }
        if let Some(index) = Self::index(address, WORDS, self.path.words.len()) { return Ok(self.path.words[index]); }
        Err(self.stop(graph::Request::Read32 { pc, address }, Cause::Address))
    }
    fn read_u64(&mut self, pc: u32, address: u64) -> Result<u64, Stopped> {
        self.ready()?;
        if address == DESCRIPTOR { Ok(WORDS) }
        else { Err(self.stop(graph::Request::Read64 { pc, address }, Cause::Address)) }
    }
    fn write_u32(&mut self, pc: u32, address: u64, bits: u32) -> Result<(), Stopped> {
        self.ready()?;
        if let Some(index) = Self::index(address, DESCRIPTOR + 16, 4) {
            self.path.bounds[index] = f32::from_bits(bits); return Ok(());
        }
        if let Some(index) = Self::index(address, WORDS, self.path.words.len()) {
            self.path.words[index] = bits; return Ok(());
        }
        Err(self.stop(graph::Request::Write32 { pc, address, bits }, Cause::Address))
    }
    fn clear_16(&mut self, pc: u32, address: u64) -> Result<(), Stopped> {
        self.ready()?;
        if address != DESCRIPTOR + 16 { return Err(self.stop(graph::Request::Clear16 { pc, address }, Cause::Address)); }

        self.path.bounds = [f32::from_bits(0); 4];
        Ok(())
    }
    fn f32(&mut self, pc: u32, operation: glyph_fit::F32Operation) -> Result<u32, Stopped> {
        self.ready()?;
        match glyph_fit::Services::arithmetic(&mut self.math, pc, operation) {
            Ok(bits) => Ok(bits), Err(error) => Err(self.stop(graph::Request::F32 { pc, operation }, Cause::Math(error))),
        }
    }
    fn ucomiss(&mut self, pc: u32, left: u32, right: u32) -> Result<glyph_fit::CompareFlags, Stopped> {
        self.ready()?;
        match glyph_fit::Services::ucomiss(&mut self.math, pc, left, right) {
            Ok(flags) => Ok(flags), Err(error) => Err(self.stop(graph::Request::Ucomiss { pc, left, right }, Cause::Math(error))),
        }
    }
}
