#include "controller.h"
#include "process_admission.h"
#include "pluginterfaces/gui/iplugview.h"
#include "pluginterfaces/gui/iplugviewcontentscalesupport.h"
#include "pluginterfaces/vst/ivsthostapplication.h"

#include <algorithm>
#include <array>
#include <cmath>
#include <cstddef>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <limits>
#include <memory>
#include <mutex>
#include <new>
#include <type_traits>
#include <utility>
#ifdef _WIN32
#define NOMINMAX
#include <windows.h>
#include <objbase.h>
#endif

namespace Rocoflex {
using namespace Steinberg;
using namespace Steinberg::Vst;
namespace {
bool iidIs(const TUID a, const FUID& b) { return std::memcmp(a, b.toTUID(), 16) == 0; }
constexpr const char* kLatencyRouteError="The host latency notification route could not be confirmed.";
constexpr const char* kLatencyNotifyError="The host did not accept the audio latency-change notification. It remains pending.";
constexpr const char* kLatencyConflictError="The host connection supplied conflicting audio latency information.";
bool sameEditorTarget(const RocoCondition& a, const RocoCondition& b) {
    if (a.mode!=b.mode) return false;
    if (a.mode==1) return std::memcmp(a.canvas,b.canvas,sizeof(a.canvas))==0;
    if (a.mode==2) return std::memcmp(a.tone,b.tone,sizeof(a.tone))==0;
    return a.mode==0 || a.mode==3;
}
struct ControllerHold {
    Controller& value;
    explicit ControllerHold(Controller& controller) : value(controller) { value.addRef(); }
    ~ControllerHold() { value.release(); }
    ControllerHold(const ControllerHold&) = delete;
    ControllerHold& operator=(const ControllerHold&) = delete;
};
void append32(std::vector<uint8_t>& out, uint32_t value) {
    for (unsigned n = 0; n < 4; ++n) out.push_back(static_cast<uint8_t>(value >> (n * 8)));
}
void appendFloat(std::vector<uint8_t>& out, float value) {
    uint32_t bits; std::memcpy(&bits, &value, sizeof(bits)); append32(out, bits);
}
struct Reader {
    const uint8_t* data; size_t size; size_t at = 0;
    bool word(uint32_t& value) {
        if (size - at < 4) return false;
        value = uint32_t(data[at]) | uint32_t(data[at+1]) << 8
            | uint32_t(data[at+2]) << 16 | uint32_t(data[at+3]) << 24;
        at += 4; return true;
    }
    bool number(float& value) {
        uint32_t bits; if (!word(bits)) return false;
        std::memcpy(&value, &bits, sizeof(value)); return true;
    }
    bool string(std::string& value) {
        uint32_t length; if (!word(length) || length > size - at) return false;
        value.assign(reinterpret_cast<const char*>(data + at), length); at += length;
        return value.find('\0') == std::string::npos;
    }
};
bool transfer(IBStream* stream, void* data, size_t size, bool writing) {
    auto* bytes = static_cast<uint8_t*>(data);
    while (size) {
        int32 count = 0;
        const auto chunk = static_cast<int32>(std::min<size_t>(size, 1 << 20));
        const auto result = writing ? stream->write(bytes, chunk, &count) : stream->read(bytes, chunk, &count);
        if (result != kResultOk || count <= 0 || count > chunk) return false;
        bytes += count; size -= static_cast<size_t>(count);
    }
    return true;
}
IMessage* newMessage(FUnknown* context) {
    if (!context) return nullptr;
    IHostApplication* host = nullptr;
    if (context->queryInterface(IHostApplication::iid, reinterpret_cast<void**>(&host)) != kResultOk) return nullptr;
    auto* message = allocateMessage(host); host->release(); return message;
}
void text16(TChar* out, size_t capacity, const char* source) {
    std::fill_n(out, capacity, TChar{});
    for (size_t i = 0; source[i] && i + 1 < capacity; ++i) out[i] = static_cast<unsigned char>(source[i]);
}
template<size_t N> void text16(TChar (&out)[N], const char* source) { text16(out,N,source); }
bool newEpoch(ProtocolV3::Epoch& result) {
#ifdef _WIN32
    GUID guid{};
    if (CoCreateGuid(&guid) != S_OK) return false;
    static_assert(sizeof(guid) == 16);
    std::memcpy(result.data(), &guid, sizeof(guid));
    return !ProtocolV3::isZero(result);
#else
    (void)result; return false;
#endif
}
std::string readEditorError(const char* fallback) {
    const auto length = roco_editor_last_error(nullptr, 0);
    if (!length || length > ProtocolV3::kMaximumDetailBytes + 1) return fallback;
    std::vector<char> text(length);
    if (roco_editor_last_error(text.data(), text.size()) != length || text.back() != 0 || length == 1) return fallback;
    return std::string(text.data(), length - 1);
}
bool nativeError(void* parent, const std::string& text) {
#ifdef _WIN32
    const int count = MultiByteToWideChar(CP_UTF8, MB_ERR_INVALID_CHARS, text.data(), static_cast<int>(text.size()), nullptr, 0);
    if (count <= 0) return false;
    std::wstring wide(static_cast<size_t>(count), L'\0');
    if (MultiByteToWideChar(CP_UTF8, MB_ERR_INVALID_CHARS, text.data(), static_cast<int>(text.size()), wide.data(), count) != count) return false;
    auto window = static_cast<HWND>(parent);
    if (window && !IsWindow(window)) window = nullptr;
    return MessageBoxW(window, wide.c_str(), L"Rocoflex", MB_OK | MB_ICONERROR) == IDOK;
#else
    (void)parent; (void)text; return false;
#endif
}
double unit(double value) { return std::clamp(value, 0.0, 1.0); }
struct ParameterSpec { ParamID id; const char* title; const char* units; double low, high, initial; int32 steps, flags; };
constexpr std::array<ParameterSpec, 0> kParameters {};
const ParameterSpec* parameter(ParamID id) {
    for (const auto& p : kParameters) if (p.id == id) return &p;
    return nullptr;
}
}

#include "controller_units.inl"

bool validState(const PluginState& state) {
    const auto& c = state.condition;
    if (c.mode > kConditionUnavailable || state.inputChannel > 1 || !std::isfinite(c.pregain_db)
        || !std::isfinite(c.pitch_semitones)) return false;
    for (float value : c.tone) if (!std::isfinite(value)) return false;
    for (float value : c.canvas) if (!std::isfinite(value)) return false;
    size_t total = 568;
    for (const auto* s : {&state.analysisPath, &state.userdataPath, &state.documentJson}) {
        if (s->size() > kMaximumStateBytes - total || s->find('\0') != std::string::npos) return false;
        total += s->size();
    }
    return total <= kMaximumStateBytes;
}

bool encodeState(const PluginState& state, std::vector<uint8_t>& bytes) {
    if (!validState(state)) return false;
    try {
        std::vector<uint8_t> result;
        result.reserve(568 + state.analysisPath.size() + state.userdataPath.size() + state.documentJson.size());
        append32(result, 0x58464352); append32(result, kStateVersion);
        append32(result, state.bypass ? 1 : 0); append32(result, state.inputChannel);
        append32(result, state.condition.mode);
        appendFloat(result, state.condition.pregain_db); appendFloat(result, state.condition.pitch_semitones);
        for (float value : state.condition.tone) appendFloat(result, value);
        for (float value : state.condition.canvas) appendFloat(result, value);
        for (const auto* s : {&state.analysisPath, &state.userdataPath, &state.documentJson}) {
            append32(result, static_cast<uint32_t>(s->size())); result.insert(result.end(), s->begin(), s->end());
        }
        bytes.swap(result); return true;
    } catch (...) { return false; }
}

bool decodeState(const void* data, size_t size, PluginState& state) {
    if (!data || size > kMaximumStateBytes || size < 564) return false;
    try {
        Reader r {static_cast<const uint8_t*>(data), size};
        PluginState result; uint32_t magic, version, bypass;
        if (!r.word(magic) || magic != 0x58464352 || !r.word(version) || version != kStateVersion
            || !r.word(bypass) || bypass > 1 || !r.word(result.inputChannel) || !r.word(result.condition.mode)
            || !r.number(result.condition.pregain_db) || !r.number(result.condition.pitch_semitones)) return false;
        result.bypass = bypass != 0;
        for (float& value : result.condition.tone) if (!r.number(value)) return false;
        for (float& value : result.condition.canvas) if (!r.number(value)) return false;
        if (!r.string(result.analysisPath) || !r.string(result.userdataPath) || !r.string(result.documentJson)
            || r.at != size || !validState(result)) return false;
        state = std::move(result); return true;
    } catch (...) { return false; }
}

bool writeState(IBStream* stream, const PluginState& state) {
    if (!stream) return false;
    std::vector<uint8_t> bytes; if (!encodeState(state, bytes)) return false;
    uint8_t length[4]; const auto size = static_cast<uint32_t>(bytes.size());
    for (unsigned i = 0; i < 4; ++i) length[i] = static_cast<uint8_t>(size >> (8 * i));
    return transfer(stream, length, 4, true) && transfer(stream, bytes.data(), bytes.size(), true);
}
bool readState(IBStream* stream, PluginState& state) {
    if (!stream) return false;
    uint8_t length[4]; if (!transfer(stream, length, 4, false)) return false;
    Reader r {length, 4}; uint32_t size; if (!r.word(size) || size > kMaximumStateBytes) return false;
    try {
        std::vector<uint8_t> bytes(size);
        return transfer(stream, bytes.data(), bytes.size(), false) && decodeState(bytes.data(), bytes.size(), state);
    } catch (...) { return false; }
}

bool isHostParameter(ParamID id) { return parameter(id) != nullptr; }

double parameterValue(const PluginState& state, ParamID id) {
    switch (id) {
        case kBypass: return state.bypass ? 1.0 : 0.0;
        case kPregain: return unit((state.condition.pregain_db + 36.0) / 60.0);
        case kPitch: return unit((state.condition.pitch_semitones + 12.0) / 24.0);
        case kToneVertical: case kToneHorizontal: case kToneHue: return unit(state.condition.tone[id-kToneVertical]);
        case kInputChannel: return state.inputChannel;
        case kTargetMode: return state.condition.mode / 3.0;
        default: return 0.0;
    }
}
bool setParameterValue(PluginState& state, ParamID id, double value) {
    if (!std::isfinite(value)) return false;
    value = unit(value);
    switch (id) {
        case kBypass: state.bypass = value >= 0.5; break;
        case kPregain: state.condition.pregain_db = static_cast<float>(value * 60.0 - 36.0); break;
        case kPitch: state.condition.pitch_semitones = static_cast<float>(value * 24.0 - 12.0); break;
        case kToneVertical: case kToneHorizontal: case kToneHue: state.condition.tone[id-kToneVertical] = static_cast<float>(value); break;
        case kInputChannel: state.inputChannel = value >= 0.5 ? 1 : 0; break;
        case kTargetMode: state.condition.mode = static_cast<uint32_t>(std::floor(value * 3.0 + 0.5)); break;
        default: return false;
    }
    return true;
}

Controller::~Controller() {
    terminate();
    // Views retain the controller and its borrowed editor bridge.
    if (bridge_) roco_plugin_destroy(bridge_);
}
tresult PLUGIN_API Controller::queryInterface(const TUID iid, void** object) {
    if (!iid || !object) return kInvalidArgument;
    *object = nullptr;
    if (iidIs(iid, FUnknown::iid) || iidIs(iid, IPluginBase::iid) || iidIs(iid, IEditController::iid))
        *object = static_cast<IEditController*>(this);
    else if (iidIs(iid, IConnectionPoint::iid)) *object = static_cast<IConnectionPoint*>(this);
    else if (std::memcmp(iid, INLINE_UID_OF(IUnitInfo), 16) == 0)
        *object = static_cast<IUnitInfo*>(this);
    else return kNoInterface;
    addRef(); return kResultOk;
}
uint32 PLUGIN_API Controller::addRef() { return references_.fetch_add(1, std::memory_order_relaxed) + 1; }
uint32 PLUGIN_API Controller::release() {
    const auto left = references_.fetch_sub(1, std::memory_order_acq_rel) - 1;
    if (!left) delete this;
    return left;
}
tresult PLUGIN_API Controller::initialize(FUnknown* context) {
    if (!context || context_ || bridge_) return kInvalidArgument;
    bridge_ = roco_plugin_create(); if (!bridge_) return kOutOfMemory;
    multithread_.initialize(roco_plugin_read_multithread_preference()!=0);
    context_ = context; context_->addRef(); return kResultOk;
}
tresult PLUGIN_API Controller::terminate() {
    if (bridge_) roco_plugin_invalidate_editor_session(bridge_);
    unitBackingSeen_ = false;
    ++connectionSerial_; cancelPending(); resetLatencyBinding(); multithread_.newBinding(); link_ = LinkStage::Disconnected;
    auto* peer = peer_; peer_ = nullptr;
    auto* handler = handler_; handler_ = nullptr;
    auto* context = context_; context_ = nullptr;
    // A Release callback may destroy this object.
    if (peer) peer->release();
    if (handler) handler->release();
    if (context) context->release();
    return kResultOk;
}

uint64_t Controller::nextId() {
    if (nextId_ == std::numeric_limits<uint64_t>::max()) {
        link_ = LinkStage::Disconnected;
        queueError("The plugin connection exhausted its request identifiers. Reopen the plugin instance.");
        return 0;
    }
    return ++nextId_;
}
void Controller::cancelPending() {
    ++wireGeneration_;
    flight_.reset(); unsent_ = false; queuedReload_ = false; reloadPending_ = false;
    statusRequest_ = 0;
}
void Controller::beginSnapshot(bool newConnection) {
    const bool carryReload=!newConnection && peer_
        && (link_==LinkStage::Bound || link_==LinkStage::Confirming)
        && instance_==statusInstance_ && !ProtocolV3::isZero(instance_)
        && !ProtocolV3::isZero(server_) && !ProtocolV3::isZero(client_)
        && reloadPending_ && reloadRevision_==processorRevision_;
    cancelPending();
    if (!peer_) { link_ = LinkStage::Disconnected; return; }
    if (newConnection) {
        resetLatencyBinding();multithread_.newBinding();
        lastMultithreadPoll_={};lastPreferenceCheck_={};nextMultithreadRetry_={};
        instance_ = {}; server_ = {};
        if (!newEpoch(client_)) {
            link_ = LinkStage::Disconnected;
            queueError("The plugin could not create a connection identifier."); return;
        }
    }
    snapshotRequest_ = {};
    snapshotRequest_.kind = ProtocolV3::Kind::RequestSnapshot;
    snapshotRequest_.processorInstance = instance_;
    snapshotRequest_.serverEpoch = server_;
    snapshotRequest_.clientEpoch = client_;
    snapshotRequest_.correlation = nextId();
    if (!snapshotRequest_.correlation) return;
    if (carryReload) {reloadPending_=true;reloadRevision_=processorRevision_;}
    link_ = ProtocolV3::isZero(server_) ? LinkStage::Discovering : LinkStage::Confirming;
    lastWireAttempt_ = {};
}
bool Controller::installState(PluginState state) {
    const auto quality=qualityForState(state,state_,quality_);
    return installState(std::move(state),quality);
}
bool Controller::installState(PluginState state, QualityChoice quality) {
    if (!validState(state)) return false;
    if (bridge_ && roco_plugin_set_condition(bridge_, &state.condition) != kBridgeOk) return false;
    if (state.condition.mode != kConditionUnavailable) {
        if (error_ && !error_->failure && !error_->restoreCandidate
            && error_->text == kUnavailableConditionDetail) error_.reset();
        if (!lastLocalCandidate_ && lastLocalError_ == kUnavailableConditionDetail)
            lastLocalError_.clear();
    }
    const bool documentChanged = state.analysisPath != state_.analysisPath
        || state.userdataPath != state_.userdataPath || state.documentJson != state_.documentJson;
    const bool meterModeChanged=(state.condition.mode==kConditionEmpty)
        !=(state_.condition.mode==kConditionEmpty);
    if ((!hasConditionTarget(state.condition.mode) && state.condition.mode!=kConditionEmpty)
        || meterModeChanged || state.inputChannel!=state_.inputChannel) {
        meterAvailable_=false;statusRequest_=0;
    }
    state_ = std::move(state);
    const auto* priorIssue=qualityIssue(quality_);
    const auto* nextIssue=qualityIssue(quality);
    if (priorIssue && (!nextIssue || std::strcmp(priorIssue,nextIssue)!=0)) {
        if (error_ && !error_->failure && !error_->restoreCandidate
            && error_->text==priorIssue) error_.reset();
        if (!lastLocalCandidate_ && lastLocalError_==priorIssue) lastLocalError_.clear();
    }
    quality_=quality;
    if (nextIssue) queueError(nextIssue);
    ++changeSerial_;
    if (documentChanged) ++stateRevision_;
    return true;
}
bool Controller::applyState(PluginState state, bool notifyPeer, bool hostRecall) {
    ControllerHold keep(*this);LatencyTurn turn(*this);
    const auto quality=qualityForState(state,state_,quality_);
    const bool reload=graphReprepareRequired(state_,state,quality_,quality);
    if (!installState(std::move(state),quality)) return false;
    if (hostRecall && !invalidateEditorSession()) return false;
    const auto accepted = changeSerial_;
    if (notifyPeer) {
        if (link_ == LinkStage::Bound) {
            unsent_ = true; queuedReload_ = queuedReload_ || reload; statusRequest_ = 0;
        } else {
            queueError("The plugin is waiting for its processor state. Please make the edit again after it connects.");
        }
        markDirty();
        if (changeSerial_ == accepted) serviceMessages(true);
    }
    return true;
}
bool Controller::invalidateEditorSession() {
    if (bridge_ && roco_plugin_invalidate_editor_session(bridge_)!=kBridgeOk) {
        queueError("The retained editor session could not be retired.");
        return false;
    }
    ++stateRevision_;
    return true;
}

tresult PLUGIN_API Controller::setComponentState(IBStream* stream) {
    ControllerHold keep(*this);LatencyTurn turn(*this);
    try {
        PluginState value;
        if (!readState(stream, value)) return kResultFalse;
        cancelPending();
        if (!installState(std::move(value)) || !invalidateEditorSession()) return kResultFalse;
        beginSnapshot(false); serviceMessages(true); return kResultOk;
    } catch (...) { return kInternalError; }
}
tresult PLUGIN_API Controller::setState(IBStream* stream) {
    ControllerHold keep(*this);LatencyTurn turn(*this);
    try {
        PluginState value;
        if (!readState(stream, value) || !applyState(std::move(value), true, true)) return kResultFalse;
        return kResultOk;
    } catch (...) { return kInternalError; }
}
tresult PLUGIN_API Controller::getState(IBStream* stream) {
    ControllerHold keep(*this);LatencyTurn turn(*this);
    return writeState(stream, state_) ? kResultOk : kResultFalse;
}


int32 PLUGIN_API Controller::getParameterCount() { return static_cast<int32>(kParameters.size()); }
tresult PLUGIN_API Controller::getParameterInfo(int32 index, ParameterInfo& info) {
    if (index < 0 || static_cast<size_t>(index) >= kParameters.size()) return kInvalidArgument;
    const auto& p = kParameters[static_cast<size_t>(index)]; info = {};
    info.id = p.id; text16(info.title, p.title); text16(info.shortTitle, p.title); text16(info.units, p.units);
    info.stepCount = p.steps; info.defaultNormalizedValue = p.initial; info.unitId = 0; info.flags = p.flags;
    return kResultOk;
}
tresult PLUGIN_API Controller::getParamStringByValue(ParamID id, ParamValue value, String128 text) {
    if (!text || !parameter(id) || !std::isfinite(value)) return kInvalidArgument;
    char buffer[64]; std::snprintf(buffer, sizeof(buffer), "%.3f", normalizedParamToPlain(id, value));
    text16(text, 128, buffer); return kResultOk;
}
tresult PLUGIN_API Controller::getParamValueByString(ParamID id, TChar* text, ParamValue& value) {
    if (!parameter(id) || !text) return kInvalidArgument;
    char buffer[128]{}; size_t i = 0;
    for (; i < 127 && text[i]; ++i) { if (text[i] > 127) return kResultFalse; buffer[i] = static_cast<char>(text[i]); }
    if (i == 127 && text[i]) return kResultFalse;
    char* end; const auto plain = std::strtod(buffer, &end);
    if (end == buffer || !std::isfinite(plain)) return kResultFalse;
    value = plainParamToNormalized(id, plain); return kResultOk;
}
ParamValue PLUGIN_API Controller::normalizedParamToPlain(ParamID id, ParamValue value) {
    const auto* p = parameter(id); return p ? p->low + unit(value) * (p->high-p->low) : 0.0;
}
ParamValue PLUGIN_API Controller::plainParamToNormalized(ParamID id, ParamValue value) {
    const auto* p = parameter(id); return p ? unit((value-p->low)/(p->high-p->low)) : 0.0;
}
ParamValue PLUGIN_API Controller::getParamNormalized(ParamID id) { return parameterValue(state_, id); }
tresult PLUGIN_API Controller::setParamNormalized(ParamID id, ParamValue value) {
    if (!parameter(id) || !setParameterValue(state_, id, value)) return kInvalidArgument;
    meterAvailable_=false;
    if (bridge_) roco_plugin_set_condition(bridge_, &state_.condition);
    ++changeSerial_;
    return kResultOk;
}
tresult PLUGIN_API Controller::setComponentHandler(IComponentHandler* handler) {
    ControllerHold keep(*this);LatencyTurn turn(*this);
    if (handler!=handler_) {
        if (latencyRouteSerial_==std::numeric_limits<uint64_t>::max()) return kInternalError;
        const auto serial=latencyRouteSerial_,connection=connectionSerial_;
        if (handler) handler->addRef();
        if (serial!=latencyRouteSerial_ || connection!=connectionSerial_) {
            if (handler) handler->release();return kResultFalse;
        }
        auto* old=handler_;handler_=handler;++latencyRouteSerial_;
        multithread_.routeChanged();nextMultithreadRetry_={};
        latencyRouteDirty_=true;
        if (old) old->release();
    }
    advertiseLatencyRoute();maybeNotifyLatency();serviceMessages(true);return kResultOk;
}
bool Controller::sendPacket(const Packet& packet) {
    ControllerHold keep(*this);LatencyTurn turn(*this);
    auto* peer = peer_;
    if (!peer) return false;
    peer->addRef();
    std::unique_ptr<IConnectionPoint, void(*)(IConnectionPoint*)> heldPeer(
        peer, [](IConnectionPoint* value) { value->release(); });
    const auto connection = connectionSerial_, generation = wireGeneration_;
    try {
        std::vector<uint8_t> bytes;
        if (!ProtocolV3::encode(packet, bytes)) return false;
        auto* raw = newMessage(context_);
        if (!raw) return false;
        std::unique_ptr<IMessage, void(*)(IMessage*)> message(raw, [](IMessage* value) { value->release(); });
        message->setMessageID(ProtocolV3::kMessageId);
        auto* attributes = message->getAttributes();
        if (!attributes || attributes->setBinary(ProtocolV3::kAttribute, bytes.data(),
            static_cast<uint32>(bytes.size())) != kResultOk) return false;
        if (connection != connectionSerial_ || generation != wireGeneration_ || peer_ != peer) return false;
        return peer->notify(message.get()) == kResultOk;
    } catch (...) { return false; }
}
void Controller::retireLatencyError(const char* text) {
    if (error_ && !error_->failure && !error_->restoreCandidate && error_->text==text) error_.reset();
    if (!lastLocalCandidate_ && lastLocalError_==text) lastLocalError_.clear();
}
void Controller::resetLatencyBinding() {
    meterAvailable_=false;meterConfiguration_=0;meterAttempt_=0;
    meterStream_=0;meterOrdinal_=0;
    retireLatencyError(kLatencyRouteError);retireLatencyError(kLatencyNotifyError);retireLatencyError(kLatencyConflictError);
    latency_.reset();latencyDelivered_.reset();latencyRouteRequest_=0;
    latencyRouteDirty_=true;latencyDispatchRequested_=true;
}
bool Controller::adoptLatency(const Packet& packet) {
    if (latency_ && packet.latency.epoch<latency_->epoch) return true;
    if (latency_ && packet.latency.epoch==latency_->epoch) {
        if (packet.latency!=*latency_) {
            queueError(kLatencyConflictError);return false;
        }
        return true;
    }
    retireLatencyError(kLatencyNotifyError);
    latency_=packet.latency;latencyDispatchRequested_=true;return true;
}
void Controller::advertiseLatencyRoute() {
    if (latencyAdvertising_ || !latencyRouteDirty_ || !peer_ || link_!=LinkStage::Bound) return;
    latencyAdvertising_=true;
    struct Reset {bool& flag;~Reset(){flag=false;}} reset{latencyAdvertising_};
    Packet request;request.kind=ProtocolV3::Kind::RequestStatus;
    request.processorInstance=instance_;request.serverEpoch=server_;request.clientEpoch=client_;
    request.correlation=nextId();if (!request.correlation) return;
    request.stateRevision=processorRevision_;request.configToken=configToken_;
    request.latencyHandlerReady=handler_!=nullptr;request.latencyRouteSerial=latencyRouteSerial_;
    const auto connection=connectionSerial_,serial=latencyRouteSerial_;
    latencyRouteRequest_=request.correlation;latencyRouteDirty_=false;
    const bool sent=sendPacket(request);
    if (!sent && connection==connectionSerial_ && serial==latencyRouteSerial_) {
        latencyRouteDirty_=true;queueError(kLatencyRouteError);
    }
}
void Controller::maybeNotifyLatency() {
    ControllerHold keep(*this);LatencyTurn turn(*this);
    if (latencyDispatching_) {latencyDispatchRequested_=true;return;}
    latencyDispatching_=true;
    struct Reset {bool& flag;~Reset(){flag=false;}} reset{latencyDispatching_};
    while (latencyAttempts_<2 && !latencyFailedThisTurn_) {
        latencyDispatchRequested_=false;
        if (link_!=LinkStage::Bound || !handler_ || !latency_ || !latency_->epoch
            || (latencyDelivered_ && *latencyDelivered_==*latency_)) return;
        ++latencyAttempts_;
        const auto declaration=*latency_;const auto connection=connectionSerial_,serial=latencyRouteSerial_;
        const auto instance=instance_,server=server_,client=client_;
        auto* handler=handler_;tresult result=kInternalError;
        {
            handler->addRef();
            std::unique_ptr<IComponentHandler,void(*)(IComponentHandler*)> held(handler,[](IComponentHandler* v){v->release();});
            if (connection!=connectionSerial_ || serial!=latencyRouteSerial_ || handler_!=handler
                || !latency_ || *latency_!=declaration || link_!=LinkStage::Bound) continue;
            try {result=handler->restartComponent(kLatencyChanged);} catch (...) {}
            const bool currentDeclaration=connection==connectionSerial_
                && instance==instance_ && server==server_ && client==client_ && link_==LinkStage::Bound
                && latency_ && *latency_==declaration;
            if (currentDeclaration && result==kResultOk) {latencyDelivered_=declaration;retireLatencyError(kLatencyNotifyError);}
            const bool currentRoute=currentDeclaration && serial==latencyRouteSerial_ && handler_==handler;
            if (result!=kResultOk && currentRoute) {
                latencyFailedThisTurn_=true;queueError(kLatencyNotifyError);
            }
        }
        if (!latencyDispatchRequested_) return;
    }
    if (latency_ && latency_->epoch && (!latencyDelivered_ || *latencyDelivered_!=*latency_))
        latencyDispatchRequested_=true;
}
bool Controller::adoptSnapshot(const Packet& packet) {
    if (packet.stateRevision < processorRevision_ && packet.processorInstance == statusInstance_) return adoptLatency(packet);
    const bool differentInstance = packet.processorInstance != statusInstance_;
    const bool differentConfig = differentInstance || packet.configToken != configToken_;
    const bool replaceEditor = differentConfig
        || packet.state.analysisPath!=state_.analysisPath || packet.state.userdataPath!=state_.userdataPath
        || packet.state.documentJson!=state_.documentJson
        || !sameEditorTarget(packet.state.condition,state_.condition);
    if ((link_!=LinkStage::Bound && link_!=LinkStage::Confirming)
        || packet.processorInstance!=instance_ || packet.processorInstance!=statusInstance_
        || packet.serverEpoch!=server_ || packet.clientEpoch!=client_
        || packet.configToken!=configToken_) meterAvailable_=false;
    const auto quality=qualityForState(packet.state,state_,quality_);
    cancelPending();
    if (!installState(packet.state,quality)) return false;
    if (replaceEditor && !invalidateEditorSession()) return false;
    acceptedQuality_=quality;
    instance_ = packet.processorInstance; server_ = packet.serverEpoch; statusInstance_ = instance_;
    processorRevision_ = packet.stateRevision; configToken_ = packet.configToken;
    if (differentInstance) {
        latestAttempt_ = 0; latestStatusSequence_ = 0;
        meterStream_=0; meterOrdinal_=0;
    }
    if (differentConfig && error_ && error_->failure) error_.reset();
    lastStatusPoll_ = {};
    return adoptLatency(packet);
}
bool Controller::receivePacket(const Packet& packet) {
    using namespace ProtocolV3;
    if (packet.kind == Kind::ConnectionReady) {
        if (link_ == LinkStage::Discovering) lastWireAttempt_ = {};
        return true;
    }
    if (packet.clientEpoch != client_ || link_ == LinkStage::Disconnected) return false;
    if (packet.kind==Kind::LatencyChanged) {
        if ((link_==LinkStage::Bound || link_==LinkStage::Confirming)
            && packet.processorInstance==instance_ && packet.serverEpoch==server_) return adoptLatency(packet);
        return true;
    }
    if (packet.kind == Kind::Snapshot) {
        if ((link_ != LinkStage::Discovering && link_ != LinkStage::Confirming)
            || packet.correlation != snapshotRequest_.correlation) return true;
        if (link_ == LinkStage::Discovering) {
            instance_ = packet.processorInstance; server_ = packet.serverEpoch;
            snapshotRequest_.processorInstance = instance_; snapshotRequest_.serverEpoch = server_;
            snapshotRequest_.correlation = nextId();
            if (!snapshotRequest_.correlation) return false;
            ++wireGeneration_; link_ = LinkStage::Confirming; lastWireAttempt_ = {};
            return true;
        }
        if (packet.processorInstance != instance_ || packet.serverEpoch != server_) return true;
        if (!adoptSnapshot(packet)) return false;
        link_ = LinkStage::Bound; return true;
    }
    if (link_ != LinkStage::Bound || packet.processorInstance != instance_ || packet.serverEpoch != server_) return true;
    if (packet.kind == Kind::MultithreadStatus) {
        return multithread_.observe(packet.correlation,packet.multithreadAccepted,packet.multithreadStatusSequence,
            packet.multithreadRequested,packet.multithreadPreparedAttempt,packet.multithreadPrepared);
    }
    if (packet.kind == Kind::Recall) {
        if (packet.stateRevision <= processorRevision_) return adoptLatency(packet);
        return adoptSnapshot(packet);
    }
    if (packet.kind == Kind::EditAck) {
        if (!flight_ || packet.correlation != flight_->packet.correlation) return true;
        if (packet.stateRevision < processorRevision_) { flight_.reset(); return true; }
        if (packet.editResult == EditResult::Accepted
            && (flight_->packet.baseRevision == std::numeric_limits<uint64_t>::max()
                || packet.appliedRevision != flight_->packet.baseRevision + 1)) return false;
        const bool accepted = packet.editResult == EditResult::Accepted
            && packet.appliedRevision == packet.stateRevision;
        if (!accepted) return adoptSnapshot(packet);
        bool installAcceptedState = false;
        if (!unsent_) {
            static_assert(std::is_standard_layout_v<RocoCondition>
                && std::is_trivially_copyable_v<RocoCondition>);
            static_assert(sizeof(uint32_t) == 4 && sizeof(float) == 4);
            static_assert(sizeof(RocoCondition) == 536
                && sizeof(RocoCondition) == sizeof(uint32_t) + 133 * sizeof(float));
            static_assert(offsetof(RocoCondition, mode) == 0);
            static_assert(offsetof(RocoCondition, pregain_db) == 4);
            static_assert(offsetof(RocoCondition, pitch_semitones) == 8);
            static_assert(offsetof(RocoCondition, tone) == 12);
            static_assert(offsetof(RocoCondition, canvas) == 24);
            const auto& acceptedState = packet.state;
            installAcceptedState = state_.bypass != acceptedState.bypass
                || state_.inputChannel != acceptedState.inputChannel
                || std::memcmp(&state_.condition, &acceptedState.condition, sizeof(RocoCondition)) != 0
                || state_.analysisPath != acceptedState.analysisPath
                || state_.userdataPath != acceptedState.userdataPath
                || state_.documentJson != acceptedState.documentJson;
        }
        flight_.reset(); ++wireGeneration_;
        processorRevision_ = packet.stateRevision;
        if (configToken_ != packet.configToken && error_ && error_->failure) error_.reset();
        configToken_ = packet.configToken; statusRequest_ = 0;
        acceptedQuality_=qualityForState(packet.state,state_,quality_);
        if (installAcceptedState && !installState(packet.state)) return false;
        lastWireAttempt_ = {}; lastStatusPoll_ = {};
        return adoptLatency(packet);
    }
    if (packet.kind == Kind::Status) {
        const bool routeReply=latencyRouteRequest_ && packet.correlation==latencyRouteRequest_;
        const bool ordinaryReply=statusRequest_ && packet.correlation==statusRequest_;
        if (!routeReply && !ordinaryReply) return true;
        if (!adoptLatency(packet)) return false;
        if (routeReply) {latencyRouteRequest_=0;retireLatencyError(kLatencyRouteError);return true;}
        if (!statusRequest_ || packet.correlation != statusRequest_) return true;
        statusRequest_ = 0;
        const bool meterStatus=(hasConditionTarget(state_.condition.mode)
                && packet.phase==Phase::Processing && packet.code==kBridgeOk)
            || (state_.condition.mode==kConditionEmpty
                && packet.phase==Phase::NotPrepared && packet.code==kBridgeNotReady);
        if (packet.statusResult!=StatusResult::Busy && packet.meterAvailable
            && meterStatus
            && packet.configToken==statusConfig_ && packet.configToken==configToken_
            && packet.attemptId>=latestAttempt_ && packet.attemptId>=meterAttempt_
            && (packet.meterStream>meterStream_ || (packet.meterStream==meterStream_
                && packet.meterOrdinal>meterOrdinal_))) {
            const auto now=std::chrono::steady_clock::now();
            const auto upperAge=(now-statusIssued_)+std::chrono::milliseconds(packet.meterAgeUpperMs);
            meterAvailable_=upperAge<=std::chrono::milliseconds(100);
            meterStream_=packet.meterStream;meterOrdinal_=packet.meterOrdinal;
            meterConfiguration_=packet.configToken;meterAttempt_=packet.attemptId;
            if (meterAvailable_) {
                std::memcpy(&meterEnvelope_,&packet.meterEnvelopeBits,sizeof(meterEnvelope_));
                meterObserved_=now-upperAge;
            }
        }
        if (packet.statusResult == StatusResult::Busy) return true;
        if (packet.statusResult == StatusResult::StaleState) { beginSnapshot(false); return true; }
        if (packet.stateRevision>processorRevision_) {beginSnapshot(false);return true;}
        if (packet.stateRevision != statusRevision_ || packet.stateRevision != processorRevision_
            || packet.configToken != statusConfig_ || packet.configToken != configToken_) return true;
        if (packet.attemptId < latestAttempt_ || (packet.attemptId == latestAttempt_
            && packet.statusSequence < latestStatusSequence_)) return true;
        latestAttempt_ = packet.attemptId; latestStatusSequence_ = packet.statusSequence;
        if (!packet.meterAvailable || !meterStatus)
            meterAvailable_=false;
        if (error_ && error_->failure && (error_->failure->config != configToken_
            || error_->failure->attempt < latestAttempt_)) error_.reset();
        if (packet.code < 0 && packet.phase == Phase::Faulted) {
            const FailureKey key{instance_, configToken_, packet.attemptId, packet.statusSequence};
            if (!lastFailure_ || !(*lastFailure_ == key)) {
                if (queueError(packet.detail.empty() ? "The audio processor failed (code "
                    + std::to_string(packet.code) + ")." : packet.detail, key)) lastFailure_ = key;
            }
        } else if (const auto* issue=qualityIssue(quality_);
            issue && packet.code==kBridgeNotReady && packet.phase==Phase::NotPrepared
            && packet.detail==issue) {
            queueError(packet.detail);
        } else if (state_.condition.mode == kConditionUnavailable
            && packet.code == kBridgeNotReady && packet.phase == Phase::NotPrepared
            && packet.detail == kUnavailableConditionDetail) {
            queueError(packet.detail);
        }
        return true;
    }
    return false;
}
void Controller::syncMultithreadEdits() {
    if (!bridge_) return;
    uint32_t value=0;
    const auto result=roco_plugin_take_multithread_edit(bridge_,&value);
    if (result==1 && value<=1) {
        uint32_t saved=0;
        if (roco_plugin_try_read_multithread_preference(&saved)==kBridgeOk && saved<=1) value=saved;
        if (multithread_.edit(value!=0)) {lastMultithreadPoll_={};nextMultithreadRetry_={};}
        lastPreferenceCheck_=std::chrono::steady_clock::now();
    }
    refreshMultithreadPreference();
}
void Controller::refreshMultithreadPreference() {
    if (!bridge_) return;
    const auto now=std::chrono::steady_clock::now();
    if (lastPreferenceCheck_!=std::chrono::steady_clock::time_point{}
        && now-lastPreferenceCheck_<std::chrono::milliseconds(500)) return;
    lastPreferenceCheck_=now;
    uint32_t saved=0;
    if (roco_plugin_try_read_multithread_preference(&saved)==kBridgeOk && saved<=1
        && multithread_.edit(saved!=0)) {lastMultithreadPoll_={};nextMultithreadRetry_={};}
}
void Controller::deliverMultithread(RocoEditor* editor) {
    if (!editor) return;
    roco_editor_set_multithread(editor,multithread_.desired()?1u:0u);
}
void Controller::serviceMessages(bool immediately, bool editorOwner, bool policyReply) {
    ControllerHold keep(*this);LatencyTurn turn(*this);
    // Reject editor-owner or policy-reply reentry on audio callbacks; disk/mailbox work is editor-owner only.
    if ((editorOwner || policyReply) && ProcessAdmission::Scope::any()) return;
    if (editorOwner) syncMultithreadEdits();
    advertiseLatencyRoute();maybeNotifyLatency();
    if (servicing_ || !peer_ || link_ == LinkStage::Disconnected) return;
    servicing_ = true;
    struct Reset { bool& flag; ~Reset() { flag = false; } } reset{servicing_};
    using namespace ProtocolV3;
    const auto now = std::chrono::steady_clock::now();
    const bool due = immediately || now - lastWireAttempt_ >= std::chrono::milliseconds(250);
    if (link_ != LinkStage::Bound) {
        if (!due) return;
        for (unsigned leg = 0; leg < 2 && link_ != LinkStage::Bound; ++leg) {
            const auto before = link_;
            lastWireAttempt_ = now; auto packet = snapshotRequest_;
            packet.latencyHandlerReady=handler_!=nullptr;packet.latencyRouteSerial=latencyRouteSerial_;
            sendPacket(packet);
            if (link_ == before) return;
        }
        if (link_ != LinkStage::Bound) return;
    }
    if (!policyReply) {
        if (!multithread_.request()) {
            const auto request=nextId();
            if (!request || !multithread_.assign(request)) return;
            lastMultithreadPoll_={};
        }
        if (lastMultithreadPoll_==std::chrono::steady_clock::time_point{}
            || now-lastMultithreadPoll_>=std::chrono::milliseconds(250)) {
            Packet policy;
            policy.kind=Kind::MultithreadRequest;policy.processorInstance=instance_;
            policy.serverEpoch=server_;policy.clientEpoch=client_;policy.correlation=multithread_.request();
            policy.multithreadRequested=multithread_.desired();lastMultithreadPoll_=now;
            sendPacket(policy);
            if (link_!=LinkStage::Bound || !peer_) return;
        }
    }
    if (!flight_ && unsent_) {
        Packet edit;
        edit.kind = Kind::Edit; edit.processorInstance = instance_; edit.serverEpoch = server_;
        edit.clientEpoch = client_; edit.correlation = nextId(); edit.baseRevision = processorRevision_;
        if (!edit.correlation) return;
        edit.state = state_;
        flight_ = Flight{std::move(edit), queuedReload_};
        unsent_ = false; queuedReload_ = false; lastWireAttempt_ = {};
    }
    if (flight_ && (immediately || now - lastWireAttempt_ >= std::chrono::milliseconds(250))) {
        const auto packet = flight_->packet; lastWireAttempt_ = now; sendPacket(packet);
    }
    if (link_ != LinkStage::Bound || flight_ || unsent_) return;
    if (reloadPending_ && reloadRevision_ == processorRevision_ && handler_) {
        auto* handler = handler_; handler->addRef();
        std::unique_ptr<IComponentHandler, void(*)(IComponentHandler*)> held(
            handler, [](IComponentHandler* value) { value->release(); });
        const auto revision = processorRevision_, generation = wireGeneration_;
        const auto reloadInstance=instance_,reloadServer=server_,reloadClient=client_;
        const auto reloadConnection=connectionSerial_;
        const auto reloadProfile=acceptedQuality_.profile;
        const bool policyAdvised=(editorOwner || policyReply) && multithread_.needsReload()
            && (nextMultithreadRetry_==std::chrono::steady_clock::time_point{} || now>=nextMultithreadRetry_);
        const auto policyRequest=multithread_.request();
        if (policyAdvised) multithread_.markReload();
        reloadPending_ = false;
        tresult result = kInternalError;
        try { result = handler->restartComponent(kReloadComponent); } catch (...) {}
        if (policyAdvised && result!=kResultOk && generation==wireGeneration_
            && connectionSerial_==reloadConnection && handler_==handler
            && multithread_.retryAfterRefusal(policyRequest))
            nextMultithreadRetry_=std::chrono::steady_clock::now()+std::chrono::seconds(1);
        if ((link_==LinkStage::Bound || (link_==LinkStage::Confirming
                && statusInstance_==reloadInstance)) && connectionSerial_==reloadConnection
            && instance_==reloadInstance && server_==reloadServer && client_==reloadClient
            && acceptedQuality_.profile==reloadProfile
            && (result!=kResultOk || revision!=processorRevision_)) {
            reloadPending_=true;reloadRevision_=processorRevision_;
        }
        if (generation != wireGeneration_ || link_ != LinkStage::Bound || flight_ || unsent_) return;
    }
    if ((editorOwner || policyReply) && multithread_.needsReload() && handler_
        && (nextMultithreadRetry_==std::chrono::steady_clock::time_point{} || now>=nextMultithreadRetry_)) {
        auto* handler=handler_;handler->addRef();
        std::unique_ptr<IComponentHandler,void(*)(IComponentHandler*)> held(
            handler,[](IComponentHandler* value){value->release();});
        const auto generation=wireGeneration_,connection=connectionSerial_,request=multithread_.request();
        multithread_.markReload();
        tresult result=kInternalError;
        try {result=handler->restartComponent(kReloadComponent);} catch (...) {}
        if (generation==wireGeneration_ && connection==connectionSerial_ && handler_==handler
            && result!=kResultOk && multithread_.retryAfterRefusal(request))
            nextMultithreadRetry_=std::chrono::steady_clock::now()+std::chrono::seconds(1);
        if (generation!=wireGeneration_ || connection!=connectionSerial_
            || link_!=LinkStage::Bound || !peer_) return;
    }
    if (!policyReply && (immediately || now - lastStatusPoll_ >= std::chrono::milliseconds(33))) {
        Packet request;
        request.kind = Kind::RequestStatus; request.processorInstance = instance_;
        request.serverEpoch = server_; request.clientEpoch = client_;
        if (!statusRequest_) {
            statusRequest_ = nextId(); statusRevision_ = processorRevision_; statusConfig_ = configToken_;
            statusIssued_=now;
        }
        if (!statusRequest_) return;
        request.correlation = statusRequest_;
        request.stateRevision = statusRevision_; request.configToken = statusConfig_;
        request.latencyHandlerReady=handler_!=nullptr;request.latencyRouteSerial=latencyRouteSerial_;
        lastStatusPoll_ = now; sendPacket(request);
    }
}
bool Controller::freshInputMeter() const {
    return meterAvailable_ && link_==LinkStage::Bound
        && (hasConditionTarget(state_.condition.mode) || state_.condition.mode==kConditionEmpty)
        && meterConfiguration_==configToken_ && meterAttempt_>=latestAttempt_
        && std::chrono::steady_clock::now()-meterObserved_<=std::chrono::milliseconds(100);
}
void Controller::deliverInputMeter(RocoEditor* editor) {
    const bool fresh=freshInputMeter();
    roco_editor_set_input_meter(editor,fresh?1u:0u,fresh?meterEnvelope_:0.f);
}
tresult PLUGIN_API Controller::connect(IConnectionPoint* peer) {
    ControllerHold keep(*this);LatencyTurn turn(*this);
    if (!peer || peer_) return kInvalidArgument;
    peer_ = peer; unitBackingSeen_ = true;
    peer_->addRef(); ++connectionSerial_;
    beginSnapshot(true); serviceMessages(true); return kResultOk;
}
tresult PLUGIN_API Controller::disconnect(IConnectionPoint* peer) {
    ControllerHold keep(*this);LatencyTurn turn(*this);
    if (!peer_ || peer != peer_) return kInvalidArgument;
    ++connectionSerial_; cancelPending(); resetLatencyBinding();multithread_.newBinding();
    lastMultithreadPoll_={};lastPreferenceCheck_={};nextMultithreadRetry_={};link_ = LinkStage::Disconnected;
    auto* previous = peer_; peer_ = nullptr; previous->release(); return kResultOk;
}
tresult PLUGIN_API Controller::notify(IMessage* message) {
    ControllerHold keep(*this);LatencyTurn turn(*this);
    if (!message || !message->getMessageID() || std::strcmp(message->getMessageID(), ProtocolV3::kMessageId))
        return kInvalidArgument;
    try {
        const void* data = nullptr; uint32 size = 0;
        auto* attributes = message->getAttributes(); Packet packet;
        if (!attributes || attributes->getBinary(ProtocolV3::kAttribute, data, size) != kResultOk
            || !ProtocolV3::decode(data, size, packet)) return kInvalidArgument;
        const bool currentPeer = packet.clientEpoch == client_ && packet.processorInstance == instance_
            && packet.serverEpoch == server_;
        const bool advance = (packet.kind == ProtocolV3::Kind::ConnectionReady && link_ == LinkStage::Discovering)
            || (packet.kind == ProtocolV3::Kind::Snapshot
                && (link_ == LinkStage::Discovering || link_ == LinkStage::Confirming)
                && packet.correlation == snapshotRequest_.correlation)
            || (packet.kind == ProtocolV3::Kind::EditAck && currentPeer && flight_
                && packet.correlation == flight_->packet.correlation)
            || (packet.kind == ProtocolV3::Kind::Recall && currentPeer && link_ == LinkStage::Bound
                && packet.stateRevision > processorRevision_)
            || (packet.kind == ProtocolV3::Kind::MultithreadStatus && currentPeer
                && packet.correlation==multithread_.request());
        const bool accepted = receivePacket(packet);
        if (accepted) {advertiseLatencyRoute();maybeNotifyLatency();}
        if (accepted && advance) {
            if (packet.kind==ProtocolV3::Kind::MultithreadStatus) {
                if (link_==LinkStage::Bound && packet.clientEpoch==client_
                    && packet.processorInstance==instance_ && packet.serverEpoch==server_
                    && packet.correlation==multithread_.request()) serviceMessages(true,false,true);
            } else serviceMessages(true);
        }
        return accepted ? kResultOk : kResultFalse;
    } catch (...) { return kInternalError; }
}
bool Controller::queueError(std::string text, std::optional<FailureKey> failure,
    std::optional<uint64_t> restoreCandidate) {
    if (text.empty() || (!failure && text == lastLocalError_ && restoreCandidate == lastLocalCandidate_)) return false;
    if (error_) return false;
    if (text.size() > ProtocolV3::kMaximumDetailBytes) {
        size_t end = ProtocolV3::kMaximumDetailBytes;
        while (end && (static_cast<unsigned char>(text[end]) & 0xc0) == 0x80) --end;
        text.resize(end);
    }
    if (!failure) { lastLocalError_ = text; lastLocalCandidate_ = restoreCandidate; }
    if (nextErrorId_ == std::numeric_limits<uint64_t>::max()) return false;
    error_ = ErrorEvent{++nextErrorId_, std::move(text), std::move(failure), restoreCandidate};
    return true;
}
void Controller::deliverError(RocoEditor* editor, void* parent) {
    ControllerHold keep(*this);LatencyTurn turn(*this);
    if (!error_) return;
    const auto event = *error_;
    if (event.id <= acknowledgedError_) { error_.reset(); return; }
    if (event.failure && link_ != LinkStage::Bound) return;
    if (event.failure && (event.failure->instance != instance_ || event.failure->config != configToken_)) {
        error_.reset(); return;
    }
    const auto result = editor ? roco_editor_report_error(editor, event.id, event.text.data(), event.text.size()) : -2;
    if (result == 2) return;
    if (result != 0 && (result >= 0 || !nativeError(parent, event.text))) return;
    acknowledgedError_ = std::max(acknowledgedError_, event.id);
    if (error_ && error_->id == event.id) error_.reset();
}
void Controller::editorFailure(void* parent, const char* fallback, bool useRustError) {
    ControllerHold keep(*this);LatencyTurn turn(*this);
    queueError(useRustError ? readEditorError(fallback) : std::string(fallback));
    deliverError(nullptr, parent);
}

void Controller::captureEditorError(const char* fallback, std::optional<uint64_t> restoreCandidate) {
    queueError(readEditorError(fallback), std::nullopt, restoreCandidate);
}
void Controller::clearEditorError(uint64_t restoreCandidate) {
    if (error_ && error_->restoreCandidate == restoreCandidate) error_.reset();
    if (lastLocalCandidate_ == restoreCandidate) {
        lastLocalError_.clear(); lastLocalCandidate_.reset();
    }
}

void Controller::markDirty() {
    ControllerHold keep(*this);LatencyTurn turn(*this);
    if (!handler_) return;
    IComponentHandler2* extended = nullptr;
    if (handler_->queryInterface(IComponentHandler2::iid, reinterpret_cast<void**>(&extended)) == kResultOk) {
        extended->setDirty(true); extended->release();
    }
}

bool Controller::syncEditor(RocoEditor* editor, uint64_t& revision) {
    ControllerHold keep(*this);LatencyTurn turn(*this);
    if (!editor || !bridge_) return false;
    try {
        PluginState next = state_;
        bool changed = false;
        const uint64_t current = roco_editor_state_revision(editor);
        if (current != revision) {
            auto readString = [&](auto getter, std::string& out) {
                const size_t length = getter(nullptr, 0);
                if (!length || length > kMaximumStateBytes) return false;
                std::vector<char> data(length);
                if (getter(data.data(), data.size()) != length || data.back() != '\0') return false;
                out.assign(data.data(), length-1); return true;
            };
            if (!readString([&](char* p, size_t n) { return roco_editor_get_state(editor,p,n); }, next.documentJson)
                || !readString([&](char* p, size_t n) { return roco_editor_get_model_path(editor,0,p,n); }, next.analysisPath)
                || !readString([&](char* p, size_t n) { return roco_editor_get_model_path(editor,1,p,n); }, next.userdataPath)) return false;
            changed = next.documentJson != state_.documentJson || next.analysisPath != state_.analysisPath
                || next.userdataPath != state_.userdataPath;
            revision = current;
        }
        RocoCondition condition; uint64_t conditionRevision = 0;
        if (roco_plugin_get_condition(bridge_, &condition, &conditionRevision) == kBridgeOk
            && conditionRevision != conditionRevision_) {
            conditionRevision_ = conditionRevision;
            if (std::memcmp(&condition, &next.condition, sizeof(condition)) != 0) {
                next.condition = condition; changed = true;
            }
        }
        if (!changed) return true;
        const auto prior = state_;
        const auto expectedSerial = changeSerial_ + 1;
        if (!applyState(std::move(next), true) || changeSerial_ != expectedSerial) return false;
        // Retain the handler across callbacks that may replace it or recall state.
        auto* handler = handler_;
        if (handler) handler->addRef();
        bool unchanged = true;
        if (handler) for (const auto& p : kParameters) {
            const auto before = parameterValue(prior, p.id), after = parameterValue(state_, p.id);
            if (before != after && (p.flags & ParameterInfo::kCanAutomate)) {
                handler->beginEdit(p.id); handler->performEdit(p.id, after); handler->endEdit(p.id);
                if (changeSerial_ != expectedSerial) { unchanged = false; break; }
            }
        }
        if (handler) handler->release();
        return unchanged;
    } catch (...) { return false; }
}

namespace {
class EditorView;
thread_local unsigned editorCallDepth = 0;
thread_local EditorView* deferredRemovals = nullptr;
thread_local bool drainingRemovals = false;
void drainEditorRemovals();
struct EditorCallScope {
    ControllerHold controllerHold;
    Controller::LatencyTurn latencyTurn;
    explicit EditorCallScope(Controller& controller):controllerHold(controller),latencyTurn(controller) { ++editorCallDepth; }
    ~EditorCallScope() { if (--editorCallDepth == 0) drainEditorRemovals(); }
    EditorCallScope(const EditorCallScope&) = delete;
    EditorCallScope& operator=(const EditorCallScope&) = delete;
};
class EditorView final : public IPlugView, public IPlugViewContentScaleSupport {
public:
    explicit EditorView(Controller& controller) : controller_(controller) { controller_.addRef(); }
    ~EditorView() { closeNative(); setFrame(nullptr); controller_.release(); }
    tresult PLUGIN_API queryInterface(const TUID iid, void** object) override {
        if (!iid || !object) return kInvalidArgument; *object = nullptr;
        if (iidIs(iid,FUnknown::iid) || iidIs(iid,IPlugView::iid))
            *object = static_cast<IPlugView*>(this);
        else if (std::memcmp(iid,INLINE_UID_OF(IPlugViewContentScaleSupport),16)==0)
            *object = static_cast<IPlugViewContentScaleSupport*>(this);
        else return kNoInterface;
        addRef(); return kResultOk;
    }
    uint32 PLUGIN_API addRef() override { return ++references_; }
    uint32 PLUGIN_API release() override {
        auto before=references_.load();
        for (;;) {
            if (before==1 && editorCallDepth && hasEditorResources() && !removalQueued_) {
                // Acquire the deferred queue reference before consuming the final external reference.
                if (!references_.compare_exchange_weak(before,2)) continue;
                deferRemoval(true);
                return references_.fetch_sub(1)-1;
            }
            if (!references_.compare_exchange_weak(before,before-1)) continue;
            const auto left=before-1;
            if (!left) delete this;
            return left;
        }
    }
    tresult PLUGIN_API isPlatformTypeSupported(FIDString type) override {
#ifdef _WIN32
        return type && std::strcmp(type,kPlatformTypeHWND)==0 ? kResultTrue : kResultFalse;
#else
        (void)type; return kResultFalse;
#endif
    }
    tresult PLUGIN_API attached(void* parent, FIDString type) override {
#ifdef _WIN32
        if (!parent || container_ || isPlatformTypeSupported(type)!=kResultTrue) return kInvalidArgument;
        ControllerHold outerControllerHold(controller_);Controller::LatencyTurn outerTurn(controller_);
        addRef();
        std::unique_ptr<EditorView,void(*)(EditorView*)> held(this,[](EditorView* view) { view->release(); });
        EditorCallScope calls(controller_);
        attaching_=true;
        struct AttachGuard {
            EditorView& view;
            ~AttachGuard() {
                view.attaching_=false;
                if (view.removalPending_) view.removed();
            }
        } attachment{*this};
        HMODULE module = nullptr;
        if (!GetModuleHandleExW(GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
            reinterpret_cast<LPCWSTR>(&windowProcedure), &module)) return kResultFalse;
        WNDCLASSW cls{}; cls.lpfnWndProc = windowProcedure; cls.hInstance = module;
        cls.lpszClassName = L"Rocoflex.Vst3.Editor"; cls.hCursor = LoadCursor(nullptr, IDC_ARROW);
        if (!RegisterClassW(&cls) && GetLastError()!=ERROR_CLASS_ALREADY_EXISTS) return kResultFalse;
        container_ = CreateWindowExW(0,cls.lpszClassName,L"Rocoflex",WS_CHILD | WS_VISIBLE | WS_CLIPCHILDREN,
            0,0,rect_.getWidth(),rect_.getHeight(),static_cast<HWND>(parent),nullptr,module,this);
        if (!container_) return kResultFalse;
        if (auto* frame=frame_) {
            frame->addRef();
            {
                std::unique_ptr<IPlugFrame,void(*)(IPlugFrame*)> heldFrame(frame,[](IPlugFrame* value) { value->release(); });
                ViewRect requested=rect_;
                frame->resizeView(this,&requested);
            }
        }
        // resizeView may synchronously remove the view, frame, or native parent.
        if (removalPending_ || removing_ || !container_ || !IsWindow(container_)
            || reinterpret_cast<EditorView*>(GetWindowLongPtrW(container_,GWLP_USERDATA))!=this)
            return kResultFalse;
        hasRestoredState_=false;
        awaitingFirstHealthyIdle_=true;
        editor_ = roco_editor_open(controller_.editorBridge(),container_,rect_.getWidth(),rect_.getHeight());
        if (removalPending_) return kResultFalse;
        if (!editor_) {
            controller_.editorFailure(parent, "The embedded editor could not be opened.", true);
            removed(); return kResultFalse;
        }
        if (!publishContentScale(contentScale_,hasContentScale_)) {
            controller_.captureEditorError("The editor content scale could not be restored.");
            removed(); return kResultFalse;
        }
        const auto wanted = controller_.stateRevision();
        const auto wantedSerial = controller_.stateSerial();
        const auto restoreResult = restoreSnapshot(controller_.state());
        const bool sameCandidate=wanted==controller_.stateRevision() && wantedSerial==controller_.stateSerial();
        if (sameCandidate && restoreResult!=0 && restoreResult!=2) {
            controller_.editorFailure(parent, "The saved editor state could not be restored.", restoreResult<0);
            removed(); return kResultFalse;
        }
        if (restoreResult==0 && sameCandidate) {
            hasRestoredState_=true;
            restoredRevision_=wanted;
            restoredSerial_=wantedSerial;
            editorRevision_=roco_editor_state_revision(editor_);
        }
        if (!SetTimer(container_,1,16,nullptr)) { removed(); return kResultFalse; }
        pump(); return kResultOk;
#else
        (void)parent; (void)type; return kResultFalse;
#endif
    }
    tresult PLUGIN_API removed() override {
        ControllerHold controllerHold(controller_);Controller::LatencyTurn turn(controller_);
        addRef();
        std::unique_ptr<EditorView,void(*)(EditorView*)> held(this,[](EditorView* view) { view->release(); });
#ifdef _WIN32
        if (container_) KillTimer(container_,1);
#endif
        if (removing_) return kResultOk;
        if (!hasEditorResources()) { removalPending_=false; return kResultOk; }
        if (pumping_ || attaching_ || editorCallDepth) { deferRemoval(false); return kResultOk; }
        closeNative();
        return kResultOk;
    }
    tresult PLUGIN_API onWheel(float) override { return kResultFalse; }
    tresult PLUGIN_API onKeyDown(char16 key,int16 code,int16 modifiers) override {
        return hostKey(key,code,modifiers,true);
    }
    tresult PLUGIN_API onKeyUp(char16 key,int16 code,int16 modifiers) override {
        return hostKey(key,code,modifiers,false);
    }
    tresult PLUGIN_API getSize(ViewRect* rect) override { if (!rect) return kInvalidArgument; *rect=rect_; return kResultOk; }
    tresult PLUGIN_API setContentScaleFactor(ScaleFactor factor) override {
        ControllerHold outerControllerHold(controller_);Controller::LatencyTurn outerTurn(controller_);
        addRef();
        std::unique_ptr<EditorView,void(*)(EditorView*)> held(this,[](EditorView* view) { view->release(); });
        EditorCallScope calls(controller_);
        ViewRect requested;
        if (!std::isfinite(factor) || factor<=0.f || factor>8.f
            || !scaledRect(factor,requested)) return kInvalidArgument;
        if (factor==contentScale_ && hasContentScale_) return kResultOk;
        if (changingScale_ || removalPending_ || removing_) return kResultFalse;
        if (factor==contentScale_) {
            if (!publishContentScale(factor,true)) return kResultFalse;
            hasContentScale_=true; return kResultOk;
        }
        if (!frame_) {
            if (hasEditorResources()) return kResultFalse;
            contentScale_=factor; hasContentScale_=true; rect_=requested; ++geometrySerial_;
            return kResultOk;
        }
        const auto oldScale=contentScale_;
        const auto oldExplicit=hasContentScale_;
        const auto beforeGeometry=geometrySerial_;
        const auto beforeSizeAttempt=onSizeSerial_;
        const bool sizeChanged=rect_.getWidth()!=requested.getWidth() || rect_.getHeight()!=requested.getHeight();
        changingScale_=true;
        struct ChangingGuard { bool& flag; ~ChangingGuard() { flag=false; } } changing{changingScale_};
        scaleTarget_=requested;
        scaleLogicalWidth_=logicalWidth_; scaleLogicalHeight_=logicalHeight_;
        contentScale_=factor;
        hasContentScale_=true;
        if (!publishContentScale(factor,true)) {
            contentScale_=oldScale; hasContentScale_=oldExplicit;
            return kResultFalse;
        }
        tresult result;
        auto* frame=frame_;
        frame->addRef();
        {
            std::unique_ptr<IPlugFrame,void(*)(IPlugFrame*)> heldFrame(frame,[](IPlugFrame* value) { value->release(); });
            result=frame->resizeView(static_cast<IPlugView*>(this),&requested);
        }
        const bool applied=rect_.getWidth()==scaleTarget_.getWidth()
            && rect_.getHeight()==scaleTarget_.getHeight();
        const bool resizeAccepted=onSizeSerial_!=beforeSizeAttempt ? lastOnSizeSucceeded_ : !sizeChanged;
        if (result!=kResultOk || !applied || !resizeAccepted || removalPending_ || removing_) {
            if (geometrySerial_==beforeGeometry) {
                contentScale_=oldScale; hasContentScale_=oldExplicit;
                publishContentScale(oldScale,oldExplicit);
            }
            return result!=kResultOk ? result : kResultFalse;
        }
        return kResultOk;
    }
    tresult PLUGIN_API onSize(ViewRect* rect) override {
        ControllerHold outerControllerHold(controller_);Controller::LatencyTurn outerTurn(controller_);
        addRef();
        std::unique_ptr<EditorView,void(*)(EditorView*)> held(this,[](EditorView* view) { view->release(); });
        EditorCallScope calls(controller_);
        const auto sizeAttempt=++onSizeSerial_;
        lastOnSizeSucceeded_=false;
        if (!rect) return kInvalidArgument;
        if (removalPending_ || removing_) return kResultFalse;
        const ViewRect desired=*rect;
        const auto width=int64_t(desired.right)-desired.left, height=int64_t(desired.bottom)-desired.top;
        int32 minWidth,minHeight,maxWidth,maxHeight;
        if (!physicalBounds(contentScale_,minWidth,minHeight,maxWidth,maxHeight)
            || width<minWidth || height<minHeight || width>maxWidth || height>maxHeight) return kInvalidArgument;
        ViewRect actual=desired;
        bool nativeAccepted=true;
#ifdef _WIN32
        if (container_) {
            auto window=container_;
            const auto before=geometrySerial_;
            const auto beforeScale=contentScale_;
            nativeAccepted=SetWindowPos(window,nullptr,0,0,static_cast<int>(width),static_cast<int>(height),
                SWP_NOZORDER | SWP_NOACTIVATE)!=FALSE;
            if (removalPending_ || removing_ || container_!=window || geometrySerial_!=before || contentScale_!=beforeScale
                || onSizeSerial_!=sizeAttempt
                || !IsWindow(window) || reinterpret_cast<EditorView*>(GetWindowLongPtrW(window,GWLP_USERDATA))!=this)
                return kResultFalse;
            RECT client{};
            if (!GetClientRect(window,&client)) return kResultFalse;
            const auto actualWidth=int64_t(client.right)-client.left, actualHeight=int64_t(client.bottom)-client.top;
            if (actualWidth<minWidth || actualHeight<minHeight || actualWidth>maxWidth || actualHeight>maxHeight
                || !rectWithSize(desired,actualWidth,actualHeight,actual)) return kResultFalse;
        }
#endif
        commitRect(actual);
        const auto committed=geometrySerial_;
        const auto resizeResult=editor_ ? roco_editor_resize(editor_,rect_.getWidth(),rect_.getHeight()) : 0;
        if (removalPending_ || removing_ || geometrySerial_!=committed || onSizeSerial_!=sizeAttempt) return kResultFalse;
        lastOnSizeSucceeded_=nativeAccepted && resizeResult==0 && actual.getWidth()==width && actual.getHeight()==height;
        return lastOnSizeSucceeded_ ? kResultOk : kResultFalse;
    }
    tresult PLUGIN_API onFocus(TBool focused) override {
        ControllerHold outerControllerHold(controller_);Controller::LatencyTurn outerTurn(controller_);
        addRef();
        std::unique_ptr<EditorView,void(*)(EditorView*)> held(this,[](EditorView* view) { view->release(); });
        EditorCallScope calls(controller_);
        if (!editor_ || removalPending_ || removing_) return kResultFalse;
#ifdef _WIN32
        intptr_t result=0;
        return roco_editor_window_message(editor_,focused ? WM_SETFOCUS : WM_KILLFOCUS,
            0,0,&result)>=0 ? kResultOk : kResultFalse;
#else
        return kResultFalse;
#endif
    }
    tresult PLUGIN_API setFrame(IPlugFrame* frame) override {
        ControllerHold controllerHold(controller_);Controller::LatencyTurn turn(controller_);
        if (frame) frame->addRef(); if (frame_) frame_->release(); frame_=frame; return kResultOk;
    }
    tresult PLUGIN_API canResize() override { return kResultTrue; }
    tresult PLUGIN_API checkSizeConstraint(ViewRect* rect) override {
        if (!rect) return kInvalidArgument;
        int32 minWidth,minHeight,maxWidth,maxHeight;
        if (!physicalBounds(contentScale_,minWidth,minHeight,maxWidth,maxHeight)) return kInvalidArgument;
        const auto width=std::clamp<int64_t>(int64_t(rect->right)-rect->left,minWidth,maxWidth);
        const auto height=std::clamp<int64_t>(int64_t(rect->bottom)-rect->top,minHeight,maxHeight);
        ViewRect constrained;
        if (!rectWithSize(*rect,width,height,constrained)) return kInvalidArgument;
        *rect=constrained; return kResultOk;
    }
private:
    bool publishContentScale(float factor,bool explicitFactor) {
        return !editor_ || roco_editor_set_content_scale(editor_,explicitFactor ? factor : 0.f)==0;
    }
    static bool rectWithSize(const ViewRect& origin,int64_t width,int64_t height,ViewRect& out) {
        const auto right=int64_t(origin.left)+width, bottom=int64_t(origin.top)+height;
        if (right<std::numeric_limits<int32>::min() || right>std::numeric_limits<int32>::max()
            || bottom<std::numeric_limits<int32>::min() || bottom>std::numeric_limits<int32>::max()) return false;
        out=ViewRect(origin.left,origin.top,static_cast<int32>(right),static_cast<int32>(bottom)); return true;
    }
    static bool physicalBounds(float factor,int32& minWidth,int32& minHeight,int32& maxWidth,int32& maxHeight) {
        if (!std::isfinite(factor) || factor<=0.f || factor>8.f) return false;
        const auto loWidth=std::max(1.,std::round(860.*factor));
        const auto loHeight=std::max(1.,std::round(560.*factor));
        const auto maximum=std::min(8192.,std::round(8192.*factor));
        if (maximum<loWidth || maximum<loHeight) return false;
        minWidth=static_cast<int32>(loWidth); minHeight=static_cast<int32>(loHeight);
        maxWidth=maxHeight=static_cast<int32>(maximum); return true;
    }
    bool scaledRect(float factor,ViewRect& out) const {
        int32 minWidth,minHeight,maxWidth,maxHeight;
        if (!physicalBounds(factor,minWidth,minHeight,maxWidth,maxHeight)) return false;
        const auto width=std::round(logicalWidth_*factor), height=std::round(logicalHeight_*factor);
        if (!std::isfinite(width) || !std::isfinite(height)
            || width<minWidth || height<minHeight || width>maxWidth || height>maxHeight) return false;
        return rectWithSize(rect_,static_cast<int64_t>(width),static_cast<int64_t>(height),out);
    }
    void commitRect(const ViewRect& actual) {
        rect_=actual;
        if (changingScale_ && actual.getWidth()==scaleTarget_.getWidth() && actual.getHeight()==scaleTarget_.getHeight()) {
            logicalWidth_=scaleLogicalWidth_; logicalHeight_=scaleLogicalHeight_;
        } else {
            logicalWidth_=double(actual.getWidth())/contentScale_;
            logicalHeight_=double(actual.getHeight())/contentScale_;
        }
        ++geometrySerial_;
    }
    int32_t restoreSnapshot(const PluginState& state) {
        const auto snapshot=state;
        const auto paths=roco_editor_set_model_paths(editor_,snapshot.analysisPath.c_str(),snapshot.userdataPath.c_str());
        if (paths!=0) return paths;
        return roco_editor_set_state(editor_,snapshot.documentJson.empty() ? nullptr : snapshot.documentJson.data(),snapshot.documentJson.size(), &snapshot.condition);
    }
    tresult hostKey(char16 key,int16 code,int16 modifiers,bool down) {
        ControllerHold outerControllerHold(controller_);Controller::LatencyTurn outerTurn(controller_);
        addRef();
        std::unique_ptr<EditorView,void(*)(EditorView*)> held(this,[](EditorView* view) { view->release(); });
        EditorCallScope calls(controller_);
        if (!editor_ || removalPending_ || removing_) return kResultFalse;
        return roco_editor_host_key(editor_,static_cast<uint16_t>(key),code,modifiers,
            down ? 1u : 0u)==1 ? kResultTrue : kResultFalse;
    }
    bool hasEditorResources() const {
#ifdef _WIN32
        return editor_ || container_;
#else
        return editor_ != nullptr;
#endif
    }
    void deferRemoval(bool referenceAlreadyHeld) {
        removalPending_=true;
        // Cancellation is non-dropping while a Rust callback is live.
        if (editor_) roco_editor_cancel(editor_);
        if (!removalQueued_) {
            if (!referenceAlreadyHeld) addRef();
            removalQueued_=true; nextRemoval_=deferredRemovals; deferredRemovals=this;
        }
    }
    void closeNative() {
        // Defer the final reference while native resources outlive a Rust call.
        if (!hasEditorResources()) { removalPending_=false; return; }
        removing_=true;
        EditorCallScope calls(controller_);
        if (editor_) {
            auto* closing=editor_; editor_=nullptr;
            roco_editor_close(closing);
        }
#ifdef _WIN32
        if (container_) {
            auto window=container_; container_=nullptr;
            if (IsWindow(window) && reinterpret_cast<EditorView*>(GetWindowLongPtrW(window,GWLP_USERDATA))==this) {
                SetWindowLongPtrW(window,GWLP_USERDATA,0);
                DestroyWindow(window);
            }
        }
#endif
        removalPending_=false; removing_=false;
    }
    void pump() {
        if (!editor_ || pumping_ || changingScale_ || removing_ || removalPending_) return;
        ControllerHold outerControllerHold(controller_);Controller::LatencyTurn outerTurn(controller_);
        // Retain the view across host callbacks that may close it.
        addRef();
        {
        EditorCallScope calls(controller_);
        pumping_=true;
        try {
            controller_.syncMultithreadEdits();
            controller_.deliverMultithread(editor_);
            bool restored=true;
            if (!hasRestoredState_ || restoredRevision_ != controller_.stateRevision()
                || (awaitingFirstHealthyIdle_ && restoredSerial_ != controller_.stateSerial())) {
                const auto wanted=controller_.stateRevision();
                const auto wantedSerial=controller_.stateSerial();
                const auto restoreResult=restoreSnapshot(controller_.state());
                const bool sameCandidate=controller_.stateRevision()==wanted && controller_.stateSerial()==wantedSerial;
                restored=restoreResult==0 && sameCandidate;
                if (restored) {
                    roco_plugin_set_condition(controller_.editorBridge(), &controller_.state().condition);
                    hasRestoredState_=true;
                    restoredRevision_=wanted;
                    restoredSerial_=wantedSerial;
                    editorRevision_=roco_editor_state_revision(editor_);
                    if (restoreFailed_) controller_.clearEditorError(failedRestoreRevision_);
                    restoreFailed_=false;
                } else if (sameCandidate && restoreResult!=0 && restoreResult!=2) {
                    if (failedRestoreRevision_!=wanted) {
                        if (restoreFailed_) controller_.clearEditorError(failedRestoreRevision_);
                        failedRestoreRevision_=wanted;
                    }
                    if (restoreResult<0)
                        controller_.captureEditorError("The saved editor state could not be restored.",wanted);
                    restoreFailed_=true;
                }
            }
            if (!removalPending_ && !removing_) controller_.deliverInputMeter(editor_);
            const auto beforeIdle=controller_.stateRevision();
            const auto beforeSerial=controller_.stateSerial();
            const bool currentBootstrap=restored && restoredRevision_==beforeIdle && restoredSerial_==beforeSerial;
            const auto idleResult=hasRestoredState_ && (!awaitingFirstHealthyIdle_ || currentBootstrap)
                ? roco_editor_idle(editor_) : 1;
            if (idleResult<0 && !removalPending_ && !removing_)
                controller_.captureEditorError("The embedded editor stopped.");
            controller_.syncMultithreadEdits();
            if (!removalPending_) {
                if (beforeIdle!=controller_.stateRevision() || beforeSerial!=controller_.stateSerial()) {
                    roco_plugin_set_condition(controller_.editorBridge(), &controller_.state().condition);
                } else if (restored && idleResult==0) {
                    awaitingFirstHealthyIdle_=false;
                    if (controller_.syncEditor(editor_,editorRevision_)) {
                        restoredRevision_=controller_.stateRevision();
                    } else {
                        roco_plugin_set_condition(controller_.editorBridge(), &controller_.state().condition);
                    }
                }
                if (!removalPending_) {
                    controller_.serviceMessages(false,true);
                    if (!removalPending_) controller_.deliverInputMeter(editor_);
                    if (!removalPending_) controller_.deliverMultithread(editor_);
                    if (!removalPending_ && idleResult<=0) {
#ifdef _WIN32
                        controller_.deliverError(editor_,container_);
#else
                        controller_.deliverError(editor_,nullptr);
#endif
                    }
                }
            } else {
                controller_.serviceMessages(false,true);
            }
        } catch (...) {
            controller_.syncMultithreadEdits();
        }
        pumping_=false;
        if (removalPending_) removed();
        }
        release();
    }
    friend void drainEditorRemovals();
#ifdef _WIN32
    static LRESULT CALLBACK windowProcedure(HWND window, UINT message, WPARAM w, LPARAM l) {
        auto* self=reinterpret_cast<EditorView*>(GetWindowLongPtrW(window,GWLP_USERDATA));
        if (message==WM_NCCREATE) {
            self=static_cast<EditorView*>(reinterpret_cast<CREATESTRUCTW*>(l)->lpCreateParams);
            SetWindowLongPtrW(window,GWLP_USERDATA,reinterpret_cast<LONG_PTR>(self));
        }
        if (!self) return DefWindowProcW(window,message,w,l);
        // Teardown callbacks must not resurrect or reenter a closing view.
        if (self->removing_) return DefWindowProcW(window,message,w,l);
        ControllerHold outerControllerHold(self->controller_);Controller::LatencyTurn outerTurn(self->controller_);
        self->addRef();
        std::unique_ptr<EditorView,void(*)(EditorView*)> held(self,[](EditorView* view) { view->release(); });
        EditorCallScope calls(self->controller_);
        if (message==WM_NCDESTROY) {
            if (self->editor_) {
                intptr_t ignored=0;
                roco_editor_window_message(self->editor_,message,w,l,&ignored);
            }
            SetWindowLongPtrW(window,GWLP_USERDATA,0);
            if (self->container_==window) self->container_=nullptr;
            self->deferRemoval(false);
            return DefWindowProcW(window,message,w,l);
        }
        if (message==WM_TIMER && w==1) { self->pump(); return 0; }
        if (message==WM_PAINT) {
            PAINTSTRUCT paint{};
            BeginPaint(window,&paint);
            if (self->editor_ && !self->removalPending_ && !self->removing_) {
                intptr_t ignored=0;
                roco_editor_window_message(self->editor_,message,w,l,&ignored);
            }
            EndPaint(window,&paint);
            return 0;
        }
        if (self->editor_ && !self->removalPending_ && !self->removing_) {
            intptr_t result=0;
            if (roco_editor_window_message(self->editor_,message,w,l,&result)==1) return result;
        }
        return DefWindowProcW(window,message,w,l);
    }
    HWND container_=nullptr;
#endif
    std::atomic<uint32> references_{1}; Controller& controller_; IPlugFrame* frame_=nullptr;
    RocoEditor* editor_=nullptr; ViewRect rect_{0,0,860,560};
    float contentScale_=1.f;
    bool hasContentScale_=false;
    double logicalWidth_=860.,logicalHeight_=560.,scaleLogicalWidth_=860.,scaleLogicalHeight_=560.;
    ViewRect scaleTarget_{};
    uint64_t geometrySerial_=0;
    uint64_t onSizeSerial_=0;
    bool lastOnSizeSucceeded_=true;
    bool changingScale_=false;
    uint64_t editorRevision_=0,restoredRevision_=0,restoredSerial_=0,failedRestoreRevision_=~uint64_t(0);
    bool pumping_=false,attaching_=false,removalPending_=false,restoreFailed_=false,hasRestoredState_=false;
    bool awaitingFirstHealthyIdle_=true;
    bool removing_=false,removalQueued_=false;
    EditorView* nextRemoval_=nullptr;
};
void drainEditorRemovals() {
    if (editorCallDepth || drainingRemovals) return;
    drainingRemovals=true;
    auto** next=&deferredRemovals;
    while (*next) {
        auto* view=*next;
        if (view->pumping_ || view->attaching_) { next=&view->nextRemoval_; continue; }
        *next=view->nextRemoval_;
        view->nextRemoval_=nullptr; view->removalQueued_=false;
        view->removed();
        view->release();
        next=&deferredRemovals;
    }
    drainingRemovals=false;
}
}

IPlugView* PLUGIN_API Controller::createView(FIDString name) {
    if (!context_ || !bridge_ || !name || std::strcmp(name,ViewType::kEditor)!=0) return nullptr;
    return new (std::nothrow) EditorView(*this);
}

}
