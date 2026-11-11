use crate::wasapi::{self, Apartment, EndpointChoice, native};
use windows::{core::PCWSTR, Win32::{
    Foundation::{CloseHandle, HANDLE, RPC_S_CALLPENDING},
    Media::{Audio::{eMultimedia, eRender, IAudioClient, IAudioRenderClient, IMMDevice,
        AUDCLNT_SHAREMODE_SHARED, AUDCLNT_STREAMFLAGS_AUTOCONVERTPCM,
        AUDCLNT_STREAMFLAGS_EVENTCALLBACK, AUDCLNT_STREAMFLAGS_SRC_DEFAULT_QUALITY, WAVEFORMATEX},
        Multimedia::WAVE_FORMAT_IEEE_FLOAT},
    System::{Com::{CLSCTX_ALL, CoWaitForMultipleHandles}, Threading::CreateEventW},
}};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Channels { Mono, Stereo }
impl Channels { pub fn count(self) -> u16 { match self { Self::Mono => 1, Self::Stereo => 2 } } }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Conversion {

    Exact,

    SystemRateAndChannelConversion,
}
#[derive(Clone, Debug)]
pub struct Parts {
    pub endpoint: EndpointChoice,
    pub sample_rate: u32,
    pub channels: Channels,
    pub conversion: Conversion,
}
impl Parts {
    pub(crate) fn format(&self) -> Result<WAVEFORMATEX, Error> {
        let block_align = self.channels.count() * 4;
        let byte_rate = self.sample_rate.checked_mul(u32::from(block_align))
            .filter(|_| self.sample_rate != 0).ok_or(Error::SampleRate(self.sample_rate))?;
        if let EndpointChoice::Id(id) = &self.endpoint && id.contains('\0') {
            return Err(Error::EndpointContainsNul);
        }
        Ok(WAVEFORMATEX { wFormatTag: WAVE_FORMAT_IEEE_FLOAT as u16,
            nChannels: self.channels.count(), nSamplesPerSec: self.sample_rate,
            nAvgBytesPerSec: byte_rate, nBlockAlign: block_align, wBitsPerSample: 32, cbSize: 0 })
    }
}
#[derive(Clone, Debug)]
pub struct Info {
    pub endpoint_id: String,

    pub sample_rate: u32,
    pub channels: Channels,
    pub conversion: Conversion,
    pub buffer_frames: u32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage { Open, Input, Padding, Acquire, Release, Start, Wait, Stop }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PacketStage { Acquiring, Copying, Releasing, Accepted }
#[derive(Clone, Copy, Debug)]
pub struct PacketAttempt { pub first_frame: u64, pub frames: u32, pub stage: PacketStage }
#[derive(Clone, Copy, Debug)]
pub struct Progress {

    pub accepted_frames: u64,
    pub packet: Option<PacketAttempt>,
    pub failure: Option<Stage>,
    pub start_attempted: bool,
    pub stop_attempted: bool,
    pub running: bool,
}
#[derive(Debug)]
pub enum Error {
    Platform { stage: Stage, cause: wasapi::Error },
    SampleRate(u32), EndpointContainsNul, EndpointExtent,
    EndpointAllocation(std::collections::TryReserveError),
    ZeroCapacity, PriorFailure(Stage), Order(&'static str),
    InputChannels { samples: usize, channels: u16 }, Nonfinite { sample: usize }, Extent,
    Padding { actual: u32, capacity: u32 }, WaitIndex(u32),

    NullBuffer { cleanup: Result<(), wasapi::Error> },
}
#[derive(Debug)]
pub struct OpenFailure { pub parts: Parts, pub error: Error }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Wait { Ready, TimedOut }

struct Event(HANDLE);
impl Drop for Event { fn drop(&mut self) { let _ = unsafe { CloseHandle(self.0) }; } }

struct Resources {
    renderer: IAudioRenderClient,
    client: IAudioClient,
    _device: IMMDevice,
    event: Event,
    _apartment: Apartment,
}
pub struct Stream { resources: Resources, parts: Parts, info: Info, progress: Progress }

fn platform(stage: Stage, operation: &'static str, error: windows::core::Error) -> Error {
    Error::Platform { stage, cause: native(operation, error) }
}

fn open(parts: &Parts) -> Result<(Resources, Info), Error> {
    let format = parts.format()?;
    let encoded_id = match &parts.endpoint {
        EndpointChoice::DefaultMultimedia => None,
        EndpointChoice::Id(id) => {
            let length = id.encode_utf16().count().checked_add(1).ok_or(Error::EndpointExtent)?;
            let mut encoded = Vec::new();
            encoded.try_reserve_exact(length).map_err(Error::EndpointAllocation)?;
            encoded.extend(id.encode_utf16()); encoded.push(0);
            Some(encoded)
        }
    };
    let apartment = Apartment::sta().map_err(|cause| Error::Platform { stage: Stage::Open, cause })?;
    let enumerator = wasapi::enumerator().map_err(|cause| Error::Platform { stage: Stage::Open, cause })?;
    let device = match &encoded_id {
        Some(id) => unsafe { enumerator.GetDevice(PCWSTR(id.as_ptr())) },
        None => unsafe { enumerator.GetDefaultAudioEndpoint(eRender, eMultimedia) },
    }.map_err(|error| platform(Stage::Open, "resolve render endpoint", error))?;
    let endpoint_id = wasapi::endpoint_id(&device)
        .map_err(|cause| Error::Platform { stage: Stage::Open, cause })?;

    let event = Event(unsafe { CreateEventW(None, false, false, None) }
        .map_err(|error| platform(Stage::Open, "CreateEventW", error))?);
    let client: IAudioClient = unsafe { device.Activate(CLSCTX_ALL, None) }
        .map_err(|error| platform(Stage::Open, "Activate(IAudioClient)", error))?;
    let flags = AUDCLNT_STREAMFLAGS_EVENTCALLBACK | match parts.conversion {
        Conversion::Exact => 0,
        Conversion::SystemRateAndChannelConversion =>
            AUDCLNT_STREAMFLAGS_AUTOCONVERTPCM | AUDCLNT_STREAMFLAGS_SRC_DEFAULT_QUALITY,
    };

    unsafe { client.Initialize(AUDCLNT_SHAREMODE_SHARED, flags, 0, 0, &format, None) }
        .map_err(|error| platform(Stage::Open, "IAudioClient::Initialize", error))?;
    unsafe { client.SetEventHandle(event.0) }
        .map_err(|error| platform(Stage::Open, "IAudioClient::SetEventHandle", error))?;
    let buffer_frames = unsafe { client.GetBufferSize() }
        .map_err(|error| platform(Stage::Open, "IAudioClient::GetBufferSize", error))?;
    if buffer_frames == 0 { return Err(Error::ZeroCapacity); }
    let renderer = unsafe { client.GetService::<IAudioRenderClient>() }
        .map_err(|error| platform(Stage::Open, "GetService(IAudioRenderClient)", error))?;
    let info = Info { endpoint_id, sample_rate: parts.sample_rate, channels: parts.channels,
        conversion: parts.conversion, buffer_frames };
    Ok((Resources { renderer, client, _device: device, event, _apartment: apartment }, info))
}

impl Stream {

    pub fn open(parts: Parts) -> Result<Self, OpenFailure> {
        match open(&parts) {
            Ok((resources, info)) => Ok(Self { resources, parts, info, progress: Progress {
                accepted_frames: 0, packet: None, failure: None, start_attempted: false,
                stop_attempted: false, running: false,
            } }),
            Err(error) => Err(OpenFailure { parts, error }),
        }
    }
    pub fn parts(&self) -> &Parts { &self.parts }
    pub fn info(&self) -> &Info { &self.info }
    pub fn progress(&self) -> Progress { self.progress }
    fn fail(&mut self, stage: Stage, error: Error) -> Error {
        self.progress.failure.get_or_insert(stage); error
    }
    fn ready(&self) -> Result<(), Error> {
        if let Some(stage) = self.progress.failure { return Err(Error::PriorFailure(stage)); }
        if self.progress.stop_attempted { return Err(Error::Order("stream is terminal")); }
        Ok(())
    }
    pub fn padding(&mut self) -> Result<u32, Error> {
        self.ready()?;
        let padding = unsafe { self.resources.client.GetCurrentPadding() }
            .map_err(|error| self.fail(Stage::Padding, platform(Stage::Padding, "GetCurrentPadding", error)))?;
        if padding > self.info.buffer_frames {
            return Err(self.fail(Stage::Padding, Error::Padding { actual: padding, capacity: self.info.buffer_frames }));
        }
        Ok(padding)
    }

    pub fn write_interleaved(&mut self, samples: &[f32]) -> Result<u32, Error> {
        self.ready()?;
        let channels = usize::from(self.parts.channels.count());
        if samples.len() % channels != 0 {
            return Err(self.fail(Stage::Input, Error::InputChannels { samples: samples.len(), channels: channels as u16 }));
        }
        if samples.is_empty() { return Ok(0); }
        let padding = self.padding()?;
        let available = self.info.buffer_frames - padding;
        let frames = (samples.len() / channels).min(available as usize) as u32;
        if frames == 0 { return Ok(0); }
        let count = frames as usize * channels;
        if let Some(sample) = samples[..count].iter().position(|sample| !sample.is_finite()) {
            return Err(self.fail(Stage::Input, Error::Nonfinite { sample }));
        }
        let bytes = count.checked_mul(4).ok_or_else(|| self.fail(Stage::Input, Error::Extent))?;
        let accepted = self.progress.accepted_frames.checked_add(u64::from(frames))
            .ok_or_else(|| self.fail(Stage::Input, Error::Extent))?;
        self.progress.packet = Some(PacketAttempt { first_frame: self.progress.accepted_frames,
            frames, stage: PacketStage::Acquiring });
        let pointer = unsafe { self.resources.renderer.GetBuffer(frames) }
            .map_err(|error| self.fail(Stage::Acquire, platform(Stage::Acquire, "IAudioRenderClient::GetBuffer", error)))?;
        if pointer.is_null() {
            let cleanup = unsafe { self.resources.renderer.ReleaseBuffer(0, 0) }
                .map_err(|error| native("ReleaseBuffer(0)", error));
            return Err(self.fail(Stage::Acquire, Error::NullBuffer { cleanup }));
        }
        if let Some(packet) = &mut self.progress.packet { packet.stage = PacketStage::Copying; }

        unsafe { std::ptr::copy_nonoverlapping(samples.as_ptr().cast::<u8>(), pointer, bytes) };
        if let Some(packet) = &mut self.progress.packet { packet.stage = PacketStage::Releasing; }
        unsafe { self.resources.renderer.ReleaseBuffer(frames, 0) }
            .map_err(|error| self.fail(Stage::Release, platform(Stage::Release, "IAudioRenderClient::ReleaseBuffer", error)))?;
        self.progress.accepted_frames = accepted;
        if let Some(packet) = &mut self.progress.packet { packet.stage = PacketStage::Accepted; }
        Ok(frames)
    }
    pub fn start(&mut self) -> Result<(), Error> {
        self.ready()?;
        if self.progress.start_attempted { return Err(Error::Order("Start was already attempted")); }
        self.progress.start_attempted = true;
        unsafe { self.resources.client.Start() }
            .map_err(|error| self.fail(Stage::Start, platform(Stage::Start, "IAudioClient::Start", error)))?;
        self.progress.running = true;
        Ok(())
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
        unsafe { self.resources.client.Stop() }
            .map_err(|error| self.fail(Stage::Stop, platform(Stage::Stop, "IAudioClient::Stop", error)))?;
        self.progress.running = false;
        Ok(())
    }
}
impl Drop for Stream {
    fn drop(&mut self) {

        if self.progress.start_attempted && !self.progress.stop_attempted { let _ = self.stop(); }
    }
}
