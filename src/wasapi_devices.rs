use crate::wasapi::{self, native, endpoint_id, take_text, Apartment, Error, TaskMemory};
use windows::{
    core::HRESULT,
    Win32::{
        Devices::FunctionDiscovery::PKEY_Device_FriendlyName,
        Foundation::ERROR_NOT_FOUND,
        Media::Audio::{
            eCapture, eMultimedia, eRender, EDataFlow, IAudioClient, IMMDevice,
            IMMDeviceEnumerator, DEVICE_STATE_ACTIVE, WAVEFORMATEX,
        },
        System::{
            Com::{
                CLSCTX_ALL, STGM_READ,
                StructuredStorage::{PropVariantClear, PropVariantToStringAlloc, PROPVARIANT},
            },
            Variant::VT_EMPTY,
        },
    },
};

#[derive(Debug)]
pub struct Catalog {
    pub render: Result<Flow, Error>,
    pub capture: Result<Flow, Error>,
}

#[derive(Debug)]
pub struct Flow {

    pub default_multimedia_endpoint: Result<Option<String>, Error>,
    pub endpoints: Vec<EndpointEntry>,
}

#[derive(Debug)]
pub struct EndpointEntry {
    pub ordinal: u32,

    pub endpoint: Result<Endpoint, Error>,
}

#[derive(Debug)]
pub struct Endpoint {

    pub id: Result<String, Error>,
    pub friendly_name: Result<Option<String>, Error>,
    pub client: Result<ClientProperties, Error>,
}

#[derive(Debug)]
pub struct ClientProperties {
    pub shared_mix_format: Result<MixFormat, Error>,
    pub periods: Result<DevicePeriods, Error>,
}

#[derive(Debug)]
pub struct MixFormat {
    pub format_tag: u16,
    pub channels: u16,
    pub sample_rate: u32,
    pub average_bytes_per_second: u32,
    pub block_align: u16,
    pub bits_per_sample: u16,

    pub extension: Vec<u8>,
}

#[derive(Clone, Copy, Debug)]
pub struct DevicePeriods {

    pub default_shared_100ns: i64,
    pub minimum_exclusive_100ns: i64,
}

pub fn spawn_catalog_query() -> std::io::Result<std::thread::JoinHandle<Result<Catalog, Error>>> {
    std::thread::Builder::new().name("wasapi-device-catalog".into()).spawn(query_in_sta)
}

fn query_in_sta() -> Result<Catalog, Error> {

    let _apartment = Apartment::sta()?;
    let enumerator = wasapi::enumerator()?;
    let render = query_flow(&enumerator, eRender);
    let capture = query_flow(&enumerator, eCapture);
    Ok(Catalog { render, capture })
}

fn query_flow(enumerator: &IMMDeviceEnumerator, direction: EDataFlow) -> Result<Flow, Error> {
    let collection = unsafe { enumerator.EnumAudioEndpoints(direction, DEVICE_STATE_ACTIVE) }
        .map_err(|error| native("EnumAudioEndpoints", error))?;
    let count = unsafe { collection.GetCount() }
        .map_err(|error| native("IMMDeviceCollection::GetCount", error))?;
    let mut endpoints = Vec::new();
    endpoints.try_reserve_exact(count as usize)
        .map_err(|cause| Error::Allocation { operation: "endpoint catalog", cause })?;
    let default_multimedia_endpoint = default_endpoint_id(enumerator, direction);
    for ordinal in 0..count {
        let endpoint = unsafe { collection.Item(ordinal) }
            .map_err(|error| native("IMMDeviceCollection::Item", error))
            .map(|device| Endpoint {
                id: endpoint_id(&device),
                friendly_name: friendly_name(&device),
                client: client_properties(&device),
            });
        endpoints.push(EndpointEntry { ordinal, endpoint });
    }
    Ok(Flow { default_multimedia_endpoint, endpoints })
}

fn default_endpoint_id(enumerator: &IMMDeviceEnumerator, direction: EDataFlow)
    -> Result<Option<String>, Error>
{
    match unsafe { enumerator.GetDefaultAudioEndpoint(direction, eMultimedia) } {
        Ok(device) => endpoint_id(&device).map(Some),
        Err(error) if error.code() == HRESULT::from_win32(ERROR_NOT_FOUND.0) => Ok(None),
        Err(error) => Err(native("GetDefaultAudioEndpoint", error)),
    }
}

struct PropertyValue(PROPVARIANT);
impl Drop for PropertyValue {
    fn drop(&mut self) {

        let _ = unsafe { PropVariantClear(&mut self.0) };
    }
}

fn friendly_name(device: &IMMDevice) -> Result<Option<String>, Error> {
    let store = unsafe { device.OpenPropertyStore(STGM_READ) }
        .map_err(|error| native("IMMDevice::OpenPropertyStore", error))?;
    let value = PropertyValue(unsafe { store.GetValue(&PKEY_Device_FriendlyName) }
        .map_err(|error| native("IPropertyStore::GetValue(FriendlyName)", error))?);

    if unsafe { value.0.Anonymous.Anonymous.vt } == VT_EMPTY {
        return Ok(None);
    }
    let text = unsafe { PropVariantToStringAlloc(&value.0) }
        .map_err(|error| native("PropVariantToStringAlloc", error))?;
    take_text(text, "device friendly name").map(Some)
}

fn client_properties(device: &IMMDevice) -> Result<ClientProperties, Error> {
    let client: IAudioClient = unsafe { device.Activate(CLSCTX_ALL, None) }
        .map_err(|error| native("IMMDevice::Activate(IAudioClient)", error))?;
    let shared_mix_format = mix_format(&client);
    let mut shared = 0;
    let mut exclusive = 0;
    let periods = unsafe { client.GetDevicePeriod(Some(&mut shared), Some(&mut exclusive)) }
        .map(|()| DevicePeriods {
            default_shared_100ns: shared,
            minimum_exclusive_100ns: exclusive,
        })
        .map_err(|error| native("IAudioClient::GetDevicePeriod", error));
    Ok(ClientProperties { shared_mix_format, periods })
}

fn mix_format(client: &IAudioClient) -> Result<MixFormat, Error> {
    let pointer = unsafe { client.GetMixFormat() }
        .map_err(|error| native("IAudioClient::GetMixFormat", error))?;
    let owned = unsafe { TaskMemory::from_raw(pointer, "IAudioClient::GetMixFormat") }?;

    let header = unsafe { owned.as_ptr().read_unaligned() };
    let length = header.cbSize as usize;
    let mut extension = Vec::new();
    extension.try_reserve_exact(length)
        .map_err(|cause| Error::Allocation { operation: "mix format extension", cause })?;
    if length != 0 {
        let bytes = unsafe {
            std::slice::from_raw_parts(
                owned.as_ptr().cast::<u8>().add(std::mem::size_of::<WAVEFORMATEX>()), length,
            )
        };
        extension.extend_from_slice(bytes);
    }
    Ok(MixFormat {
        format_tag: header.wFormatTag,
        channels: header.nChannels,
        sample_rate: header.nSamplesPerSec,
        average_bytes_per_second: header.nAvgBytesPerSec,
        block_align: header.nBlockAlign,
        bits_per_sample: header.wBitsPerSample,
        extension,
    })
}
