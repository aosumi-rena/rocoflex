use crate::wasapi::{self, Apartment, EndpointChoice, native};

pub use crate::wasapi_render::{Channels, Conversion, Parts};
use crate::wasapi_render;
use windows::{core::{Interface, PCWSTR}, Win32::{
    Foundation::{CloseHandle, HANDLE, RPC_S_CALLPENDING},
    Media::Audio::{eCapture, eRender, eMultimedia, IAudioCaptureClient, IAudioClient, IMMDevice,
        IMMEndpoint, AUDCLNT_BUFFERFLAGS_DATA_DISCONTINUITY, AUDCLNT_BUFFERFLAGS_SILENT,
        AUDCLNT_BUFFERFLAGS_TIMESTAMP_ERROR, AUDCLNT_S_BUFFER_EMPTY,
        AUDCLNT_SHAREMODE_SHARED, AUDCLNT_STREAMFLAGS_AUTOCONVERTPCM,
        AUDCLNT_STREAMFLAGS_EVENTCALLBACK, AUDCLNT_STREAMFLAGS_LOOPBACK, AUDCLNT_STREAMFLAGS_SRC_DEFAULT_QUALITY},
    System::{Com::{CLSCTX_ALL, CoWaitForMultipleHandles}, Threading::CreateEventW},
}};

#[derive(Clone, Debug)]
pub struct Info {
    pub endpoint_id: String,

    pub sample_rate: u32,
    pub channels: Channels,
    pub conversion: Conversion,
    pub buffer_frames: u32,
    pub mode: Mode,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode { Capture, RenderLoopback }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage { Open, Acquire, Copy, Release, Start, Wait, Stop }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PacketStage { Acquiring, Empty, Copying, ReturningUnconsumed, Releasing, Released }
#[derive(Clone, Copy, Debug)]
pub struct Header {
    pub ordinal: u64,

    pub first_frame: u64,
    pub frames: u32,

    pub flags: u32,
    pub device_position: u64,
    pub qpc_position_100ns: u64,
}
impl Header {
    pub fn silent(self) -> bool { self.flags & AUDCLNT_BUFFERFLAGS_SILENT.0 as u32 != 0 }
    pub fn discontinuity(self) -> bool { self.flags & AUDCLNT_BUFFERFLAGS_DATA_DISCONTINUITY.0 as u32 != 0 }

    pub fn timestamps_valid(self) -> bool { self.flags & AUDCLNT_BUFFERFLAGS_TIMESTAMP_ERROR.0 as u32 == 0 }
}
#[derive(Debug)]
pub enum Payload {

    Interleaved(Vec<f32>),

    Silent { sample_count: usize },
}
#[derive(Debug)]
pub struct Packet { pub header: Header, pub payload: Payload }
#[derive(Clone, Copy, Debug)]
pub struct PacketAttempt {
    pub header: Option<Header>,
    pub stage: PacketStage,
    pub release_frames: Option<u32>,
    pub release_succeeded: bool,
}
#[derive(Clone, Copy, Debug)]
pub struct Progress {

    pub released_frames: u64,
    pub released_packets: u64,
    pub get_buffer_status: Option<i32>,
    pub packet: Option<PacketAttempt>,
    pub failure: Option<Stage>,
    pub start_attempted: bool,
    pub stop_attempted: bool,
    pub running: bool,
}
#[derive(Debug)]
pub enum PacketFailure {
    ZeroFrames, Extent, ExceedsCapacity { frames: u32, capacity: u32 }, NullData,
    Allocation(std::collections::TryReserveError),
}
#[derive(Debug)]
pub enum Error {
    Configuration(wasapi_render::Error),
    Platform { stage: Stage, cause: wasapi::Error },
    EndpointExtent, EndpointAllocation(std::collections::TryReserveError),

    EndpointFlow(i32), ZeroCapacity, PriorFailure(Stage), Order(&'static str), WaitIndex(u32),
    LoopbackEndpointRequired,

    Packet { header: Header, cause: PacketFailure, release: Result<(), wasapi::Error> },

    Release { packet: Packet, cause: wasapi::Error },
}
#[derive(Debug)]
pub struct OpenFailure { pub parts: Parts, pub error: Error }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Wait { Ready, TimedOut }

struct Event(HANDLE);
impl Drop for Event { fn drop(&mut self) { let _ = unsafe { CloseHandle(self.0) }; } }

struct Resources {
    capture: IAudioCaptureClient,
    client: IAudioClient,
    _device: IMMDevice,
    event: Event,
    _apartment: Apartment,
}
pub struct Stream { resources: Resources, parts: Parts, info: Info, progress: Progress }

struct Lease<'a> { capture: &'a IAudioCaptureClient, attempted: bool }
impl Lease<'_> {
    fn release(&mut self, frames: u32) -> Result<(), wasapi::Error> {
        self.attempted = true;
        unsafe { self.capture.ReleaseBuffer(frames) }.map_err(|error| native("IAudioCaptureClient::ReleaseBuffer", error))
    }
}
impl Drop for Lease<'_> {
    fn drop(&mut self) { if !self.attempted { let _ = self.release(0); } }
}
fn platform(stage: Stage, operation: &'static str, error: windows::core::Error) -> Error {
    Error::Platform { stage, cause: native(operation, error) }
}
fn open(parts: &Parts, mode: Mode) -> Result<(Resources, Info), Error> {
    let format = parts.format().map_err(Error::Configuration)?;
    if mode == Mode::RenderLoopback && matches!(parts.endpoint, EndpointChoice::DefaultMultimedia) {
        return Err(Error::LoopbackEndpointRequired);
    }
    let required_flow = match mode { Mode::Capture => eCapture, Mode::RenderLoopback => eRender };
    let encoded_id = match &parts.endpoint {
        EndpointChoice::DefaultMultimedia => None,
        EndpointChoice::Id(id) => {
            let length = id.encode_utf16().count().checked_add(1).ok_or(Error::EndpointExtent)?;
            let mut encoded = Vec::new();
            encoded.try_reserve_exact(length).map_err(Error::EndpointAllocation)?;
            encoded.extend(id.encode_utf16()); encoded.push(0); Some(encoded)
        }
    };
    let apartment = Apartment::sta().map_err(|cause| Error::Platform { stage: Stage::Open, cause })?;
    let enumerator = wasapi::enumerator().map_err(|cause| Error::Platform { stage: Stage::Open, cause })?;
    let device = match &encoded_id {
        Some(id) => unsafe { enumerator.GetDevice(PCWSTR(id.as_ptr())) },
        None => unsafe { enumerator.GetDefaultAudioEndpoint(required_flow, eMultimedia) },
    }.map_err(|error| platform(Stage::Open, "resolve capture endpoint", error))?;
    let endpoint: IMMEndpoint = device.cast().map_err(|error| platform(Stage::Open, "QueryInterface(IMMEndpoint)", error))?;
    let flow = unsafe { endpoint.GetDataFlow() }.map_err(|error| platform(Stage::Open, "IMMEndpoint::GetDataFlow", error))?;
    if flow != required_flow { return Err(Error::EndpointFlow(flow.0)); }
    drop(endpoint);
    let endpoint_id = wasapi::endpoint_id(&device).map_err(|cause| Error::Platform { stage: Stage::Open, cause })?;

    let event = Event(unsafe { CreateEventW(None, false, false, None) }
        .map_err(|error| platform(Stage::Open, "CreateEventW", error))?);
    let client: IAudioClient = unsafe { device.Activate(CLSCTX_ALL, None) }
        .map_err(|error| platform(Stage::Open, "Activate(IAudioClient)", error))?;

    let flags = AUDCLNT_STREAMFLAGS_EVENTCALLBACK
        | if mode == Mode::RenderLoopback { AUDCLNT_STREAMFLAGS_LOOPBACK } else { 0 }
        | match parts.conversion {
        Conversion::Exact => 0,
        Conversion::SystemRateAndChannelConversion => AUDCLNT_STREAMFLAGS_AUTOCONVERTPCM | AUDCLNT_STREAMFLAGS_SRC_DEFAULT_QUALITY,
    };
    unsafe { client.Initialize(AUDCLNT_SHAREMODE_SHARED, flags, 0, 0, &format, None) }
        .map_err(|error| platform(Stage::Open, "IAudioClient::Initialize(capture)", error))?;
    unsafe { client.SetEventHandle(event.0) }
        .map_err(|error| platform(Stage::Open, "IAudioClient::SetEventHandle", error))?;
    let buffer_frames = unsafe { client.GetBufferSize() }
        .map_err(|error| platform(Stage::Open, "IAudioClient::GetBufferSize", error))?;
    if buffer_frames == 0 { return Err(Error::ZeroCapacity); }
    let capture = unsafe { client.GetService::<IAudioCaptureClient>() }
        .map_err(|error| platform(Stage::Open, "GetService(IAudioCaptureClient)", error))?;
    let info = Info { endpoint_id, sample_rate: parts.sample_rate, channels: parts.channels,
        conversion: parts.conversion, buffer_frames, mode };
    Ok((Resources { capture, client, _device: device, event, _apartment: apartment }, info))
}

impl Stream {

    pub fn open(parts: Parts) -> Result<Self, OpenFailure> {
        Self::open_with_mode(parts, Mode::Capture)
    }

    pub fn open_with_mode(parts: Parts, mode: Mode) -> Result<Self, OpenFailure> {
        match open(&parts, mode) {
            Ok((resources, info)) => Ok(Self { resources, parts, info, progress: Progress {
                released_frames: 0, released_packets: 0, get_buffer_status: None, packet: None,
                failure: None, start_attempted: false, stop_attempted: false, running: false,
            } }),
            Err(error) => Err(OpenFailure { parts, error }),
        }
    }
    pub fn parts(&self) -> &Parts { &self.parts }
    pub fn info(&self) -> &Info { &self.info }
    pub fn progress(&self) -> Progress { self.progress }
    fn fail(&mut self, stage: Stage, error: Error) -> Error { self.progress.failure.get_or_insert(stage); error }
    fn ready(&self) -> Result<(), Error> {
        if let Some(stage) = self.progress.failure { return Err(Error::PriorFailure(stage)); }
        if self.progress.stop_attempted { return Err(Error::Order("stream is terminal")); }
        Ok(())
    }

    pub fn read_packet(&mut self) -> Result<Option<Packet>, Error> {
        self.ready()?;
        if !self.progress.running { return Err(Error::Order("stream is not running")); }
        self.progress.packet = Some(PacketAttempt { header: None, stage: PacketStage::Acquiring,
            release_frames: None, release_succeeded: false });
        let (mut data, mut frames, mut flags, mut device_position, mut qpc_position) = (std::ptr::null_mut(), 0, 0, 0, 0);

        let status = unsafe { (self.resources.capture.vtable().GetBuffer)(self.resources.capture.as_raw(),
            &mut data, &mut frames, &mut flags, &mut device_position, &mut qpc_position) };
        self.progress.get_buffer_status = Some(status.0);
        if status == AUDCLNT_S_BUFFER_EMPTY {
            if let Some(attempt) = &mut self.progress.packet { attempt.stage = PacketStage::Empty; }
            return Ok(None);
        }
        if let Err(error) = status.ok() { return Err(self.fail(Stage::Acquire, platform(Stage::Acquire, "IAudioCaptureClient::GetBuffer", error))); }
        let header = Header { ordinal: self.progress.released_packets, first_frame: self.progress.released_frames,
            frames, flags, device_position, qpc_position_100ns: qpc_position };
        let mut lease = Lease { capture: &self.resources.capture, attempted: false };
        if let Some(attempt) = &mut self.progress.packet { attempt.header = Some(header); attempt.stage = PacketStage::Copying; }
        let copied = (|| {
            if frames == 0 { return Err(PacketFailure::ZeroFrames); }
            if frames > self.info.buffer_frames { return Err(PacketFailure::ExceedsCapacity { frames, capacity: self.info.buffer_frames }); }
            let total = self.progress.released_frames.checked_add(u64::from(frames)).ok_or(PacketFailure::Extent)?;
            let packets = self.progress.released_packets.checked_add(1).ok_or(PacketFailure::Extent)?;
            let count = (frames as usize).checked_mul(usize::from(self.parts.channels.count())).ok_or(PacketFailure::Extent)?;
            let bytes = count.checked_mul(4).filter(|bytes| *bytes <= isize::MAX as usize).ok_or(PacketFailure::Extent)?;
            let payload = if header.silent() { Payload::Silent { sample_count: count } } else {
                if data.is_null() { return Err(PacketFailure::NullData); }
                let mut samples = Vec::<f32>::new();
                samples.try_reserve_exact(count).map_err(PacketFailure::Allocation)?;

                unsafe { std::ptr::copy_nonoverlapping(data, samples.as_mut_ptr().cast::<u8>(), bytes); samples.set_len(count); }
                Payload::Interleaved(samples)
            };
            Ok((Packet { header, payload }, total, packets))
        })();
        let (packet, total, packets) = match copied {
            Ok(copied) => copied,
            Err(cause) => {
                if let Some(attempt) = &mut self.progress.packet { attempt.stage = PacketStage::ReturningUnconsumed; attempt.release_frames = Some(0); }
                let release = lease.release(0); drop(lease);
                if let Some(attempt) = &mut self.progress.packet { attempt.release_succeeded = release.is_ok(); }
                return Err(self.fail(Stage::Copy, Error::Packet { header, cause, release }));
            }
        };
        if let Some(attempt) = &mut self.progress.packet { attempt.stage = PacketStage::Releasing; attempt.release_frames = Some(frames); }
        let release = lease.release(frames); drop(lease);
        if let Err(cause) = release { return Err(self.fail(Stage::Release, Error::Release { packet, cause })); }
        self.progress.released_frames = total; self.progress.released_packets = packets;
        if let Some(attempt) = &mut self.progress.packet { attempt.stage = PacketStage::Released; attempt.release_succeeded = true; }
        Ok(Some(packet))
    }
    pub fn start(&mut self) -> Result<(), Error> {
        self.ready()?;
        if self.progress.start_attempted { return Err(Error::Order("Start was already attempted")); }
        self.progress.start_attempted = true;
        unsafe { self.resources.client.Start() }.map_err(|error| self.fail(Stage::Start, platform(Stage::Start, "IAudioClient::Start", error)))?;
        self.progress.running = true; Ok(())
    }

    pub(crate) fn wait_handle(&self) -> Result<std::os::windows::io::BorrowedHandle<'_>, Error> {
        self.ready()?;
        if !self.progress.running { return Err(Error::Order("stream is not running")); }

        Ok(unsafe { std::os::windows::io::BorrowedHandle::borrow_raw(self.resources.event.0.0) })
    }
    pub fn wait(&mut self, timeout_millis: u32) -> Result<Wait, Error> {
        self.ready()?;
        if !self.progress.running { return Err(Error::Order("stream is not running")); }
        match unsafe { CoWaitForMultipleHandles(0, timeout_millis, &[self.resources.event.0]) } {
            Ok(0) => Ok(Wait::Ready),
            Ok(index) => Err(self.fail(Stage::Wait, Error::WaitIndex(index))),
            Err(error) if error.code() == RPC_S_CALLPENDING => Ok(Wait::TimedOut),
            Err(error) => Err(self.fail(Stage::Wait, platform(Stage::Wait, "CoWaitForMultipleHandles", error))),
        }
    }

    pub fn stop(&mut self) -> Result<(), Error> {
        if self.progress.stop_attempted { return Err(Error::Order("Stop was already attempted")); }
        self.progress.stop_attempted = true;
        if !self.progress.start_attempted { return Ok(()); }
        unsafe { self.resources.client.Stop() }.map_err(|error| self.fail(Stage::Stop, platform(Stage::Stop, "IAudioClient::Stop", error)))?;
        self.progress.running = false; Ok(())
    }
}
impl Drop for Stream {
    fn drop(&mut self) { if self.progress.start_attempted && !self.progress.stop_attempted { let _ = self.stop(); } }
}
