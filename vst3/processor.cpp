#ifndef NOMINMAX
#define NOMINMAX
#endif
#include "processor.h"
#include "input_meter.h"
#include "process_admission.h"
#include "pluginterfaces/vst/ivstaudioprocessor.h"
#include "pluginterfaces/vst/ivsthostapplication.h"
#include "pluginterfaces/vst/ivstmessage.h"
#include "pluginterfaces/vst/ivstparameterchanges.h"
#include <algorithm>
#include <array>
#include <atomic>
#include <cstddef>
#include <cmath>
#include <chrono>
#include <cstring>
#include <memory>
#include <optional>
#include <mutex>
#include <objbase.h>
#include <stdexcept>
#include <type_traits>
#include <utility>
#include <vector>

namespace Rocoflex {
namespace {
using namespace Steinberg;
using namespace Steinberg::Vst;
namespace Protocol = ProtocolV4;

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

bool freshEpoch(Protocol::Epoch& epoch) {
    GUID guid{};
    if (FAILED(CoCreateGuid(&guid))) return false;
    static_assert(sizeof(guid) == sizeof(epoch));
    std::memcpy(epoch.data(), &guid, epoch.size());
    return !Protocol::isZero(epoch);
}


struct PublishedState {
    PluginState value;
    QualityChoice quality;
    uint64_t generation = 1;
    uint64_t configuration = 1;
};

class Processor final : public IComponent, public IAudioProcessor,
    public IConnectionPoint, public IProcessContextRequirements {
public:
    Processor() {
        state_ = std::make_shared<const PublishedState>();
        freshEpoch(instance_);
        for (auto& value : automation_) value.store(0.0);
    }
    ~Processor() { terminate(); }

    tresult PLUGIN_API queryInterface(const TUID iid, void** object) override {
        if (!object) return kInvalidArgument;
        *object = nullptr;
        if (!std::memcmp(iid, FUnknown_iid, 16) || !std::memcmp(iid, IPluginBase_iid, 16)
            || !std::memcmp(iid, IComponent_iid, 16)) *object = static_cast<IComponent*>(this);
        else if (!std::memcmp(iid, IAudioProcessor_iid, 16)) *object = static_cast<IAudioProcessor*>(this);
        else if (!std::memcmp(iid, IConnectionPoint_iid, 16)) *object = static_cast<IConnectionPoint*>(this);
        else if (!std::memcmp(iid, IProcessContextRequirements_iid, 16)) *object = static_cast<IProcessContextRequirements*>(this);
        if (!*object) return kNoInterface;
        addRef(); return kResultOk;
    }
    uint32 PLUGIN_API addRef() override { return ++references_; }
    uint32 PLUGIN_API release() override {
        const auto left = --references_;
        if (!left) delete this;
        return left;
    }
    tresult PLUGIN_API initialize(FUnknown* context) override {
        std::lock_guard<std::mutex> lock(control_);
        if (context_ || !context) return kInvalidArgument;
        if (Protocol::isZero(instance_)) return kInternalError;
        context_ = context; context_->addRef();
        bridge_ = roco_plugin_create();
        if (!bridge_) { context_->release(); context_ = nullptr; return kOutOfMemory; }
        multithread_.initialize(roco_plugin_read_multithread_preference()!=0);
        conditionResetRequested_.store(true, std::memory_order_release);
        return kResultOk;
    }
    tresult PLUGIN_API terminate() override {
        if (ProcessAdmission::Scope::any()) return kResultFalse;
        IConnectionPoint* peer = nullptr;
        FUnknown* context = nullptr;
        {
            ProcessAdmission::ControlLease operation(lifecycleOperation_);
            if (!operation) return kResultFalse;
            // Lifecycle writers lock before control; the reverse order only uses try-lock.
            std::lock_guard<std::mutex> lifecycle(bridgeLifecycle_);
            std::lock_guard<std::mutex> lock(control_);
            admission_.close();
            conditionResetRequested_.store(true, std::memory_order_release);
            processing_.store(false); active_.store(false); prepared_.store(false);
            // Final destruction requires the host to have stopped and joined audio.
            if (admission_.admitted()) return kResultFalse;
            peer = peer_; peer_ = nullptr;
            context = context_; context_ = nullptr;
            server_ = {}; client_ = {}; latencyRouteSerial_=0;latencyHandlerReady_=false;clearEditLocked();
            if (bridge_) { roco_plugin_destroy(bridge_); bridge_ = nullptr; }
        }
        if (peer) peer->release();
        if (context) context->release();
        return kResultOk;
    }
    tresult PLUGIN_API getControllerClassId(TUID classId) override {
        if (!classId) return kInvalidArgument;
        kControllerUID.toTUID(classId); return kResultOk;
    }
    tresult PLUGIN_API setIoMode(IoMode) override { return kResultOk; }
    int32 PLUGIN_API getBusCount(MediaType type, BusDirection direction) override {
        return type == kAudio && (direction == kInput || direction == kOutput) ? 1 : 0;
    }
    tresult PLUGIN_API getBusInfo(MediaType type, BusDirection direction, int32 index, BusInfo& bus) override {
        if (!getBusCount(type, direction) || index != 0) return kInvalidArgument;
        bus = {}; bus.mediaType = kAudio; bus.direction = direction;
        bus.channelCount = direction == kInput ? inputChannels_ : outputChannels_;
        bus.busType = kMain; bus.flags = BusInfo::kDefaultActive;
        const char16_t* name = direction == kInput ? u"Input" : u"Output";
        for (size_t i = 0; name[i]; ++i) bus.name[i] = static_cast<TChar>(name[i]);
        return kResultOk;
    }
    tresult PLUGIN_API getRoutingInfo(RoutingInfo& input, RoutingInfo& output) override {
        if (input.mediaType != kAudio || input.busIndex != 0) return kResultFalse;
        output = {kAudio, 0, -1}; return kResultOk;
    }
    tresult PLUGIN_API activateBus(MediaType type, BusDirection direction, int32 index, TBool enabled) override {
        if (!getBusCount(type, direction) || index != 0) return kInvalidArgument;
        if (direction==kInput) {
            if (inputActive_.exchange(enabled!=0)!=(enabled!=0)) advanceMeterEpoch(meterStream_);
        } else {outputActive_.store(enabled!=0);}
        return kResultOk;
    }
    tresult PLUGIN_API setBusArrangements(SpeakerArrangement* inputs, int32 inputCount,
        SpeakerArrangement* outputs, int32 outputCount) override {
        if (active_.load() || inputCount != 1 || outputCount != 1 || !inputs || !outputs) return kResultFalse;
        auto channels = [](SpeakerArrangement arrangement) {
            return arrangement == SpeakerArr::kMono ? 1 : arrangement == SpeakerArr::kStereo ? 2 : 0;
        };
        const auto in = channels(inputs[0]), out = channels(outputs[0]);
        if (!in || !out) return kResultFalse;
        inputChannels_ = in; outputChannels_ = out; return kResultOk;
    }
    tresult PLUGIN_API getBusArrangement(BusDirection direction, int32 index, SpeakerArrangement& result) override {
        if (index != 0 || (direction != kInput && direction != kOutput)) return kInvalidArgument;
        result = (direction == kInput ? inputChannels_ : outputChannels_) == 1 ? SpeakerArr::kMono : SpeakerArr::kStereo;
        return kResultOk;
    }
    tresult PLUGIN_API canProcessSampleSize(int32 size) override { return size == kSample32 ? kResultTrue : kResultFalse; }
    uint32 PLUGIN_API getLatencySamples() override { return reportedLatency_.load(std::memory_order_acquire); }
    uint32 PLUGIN_API getTailSamples() override { return kInfiniteTail; }
    uint32 PLUGIN_API getProcessContextRequirements() override { return 0; }

    tresult PLUGIN_API setupProcessing(ProcessSetup& setup) override {
        if (ProcessAdmission::Scope::any()) return kResultFalse;
        addRef();
        std::unique_ptr<Processor,void(*)(Processor*)> self(this,[](Processor* value){value->release();});
        ProcessAdmission::ControlLease operation(lifecycleOperation_);
        if (!operation) return kResultFalse;
        DrainSnapshot drain;
        {
            std::lock_guard<std::mutex> lifecycle(bridgeLifecycle_);
            std::lock_guard<std::mutex> lock(control_);
            if (!bridge_ || active_.load() || setup.symbolicSampleSize != kSample32 || setup.sampleRate != 48000.0
                || setup.maxSamplesPerBlock <= 0 || setup.maxSamplesPerBlock > 65536) return kInvalidArgument;
            if (!closeForDrainLocked(drain)) return kInternalError;
            processing_.store(false);prepared_.store(false);
        }
        const auto deadline=std::chrono::steady_clock::now()+std::chrono::milliseconds(1000);
        if (!admission_.waitUntil(deadline)) {
            std::lock_guard<std::mutex> lock(control_);
            drainRefusedLocked("Audio processing did not quiesce before setup.");
            return kResultFalse;
        }
        std::lock_guard<std::mutex> lifecycle(bridgeLifecycle_);
        std::lock_guard<std::mutex> lock(control_);
        if (!drainMatchesLocked(drain,true)) {
            drainRefusedLocked("Audio setup became stale while processing drained.");
            return kResultFalse;
        }
        const bool changed = !setupDone_ || setup.processMode != setup_.processMode
            || setup.symbolicSampleSize != setup_.symbolicSampleSize
            || setup.sampleRate != setup_.sampleRate || setup.maxSamplesPerBlock != setup_.maxSamplesPerBlock;
        auto before = std::atomic_load(&state_);
        if (changed && (before->generation == std::numeric_limits<uint64_t>::max()
            || before->configuration == std::numeric_limits<uint64_t>::max())) return kInternalError;
        if (setupGeneration_==std::numeric_limits<uint64_t>::max()) return kInternalError;
        conditionResetRequested_.store(true, std::memory_order_release);
        setupDone_ = false;
        prepared_.store(false); status_.stopped(false);
        try {
            setup_ = setup;
            for (auto& buffer : input_) buffer.assign(setup.maxSamplesPerBlock, 0.f);
            for (auto& buffer : wet_) buffer.assign(setup.maxSamplesPerBlock, 0.f);
            for (auto& buffer : dry_) buffer.assign(setup.maxSamplesPerBlock, 0.f);
            if (changed) {
                auto after = std::make_shared<PublishedState>(*before);
                ++after->generation; ++after->configuration;
                std::shared_ptr<const PublishedState> immutable = std::move(after);
                std::atomic_store(&state_, std::move(immutable));
            }
            advanceMeterEpoch(meterStream_);
            ++setupGeneration_; setupDone_ = true;
            audioBlockLimit_.store(static_cast<uint32>(setup.maxSamplesPerBlock),std::memory_order_release);
            return kResultOk;
        } catch (...) { return kOutOfMemory; }
    }
    tresult PLUGIN_API setActive(TBool enabled) override {
        if (ProcessAdmission::Scope::any()) return kResultFalse;
        addRef();
        std::unique_ptr<Processor,void(*)(Processor*)> self(this,[](Processor* value){value->release();});
        Protocol::Packet notice; bool haveNotice=false;
        tresult answer=kResultFalse;
        {
            ProcessAdmission::ControlLease operation(lifecycleOperation_);
            if (!operation) return kResultFalse;
            answer=prepareActivation(enabled,nullptr,notice,haveNotice);
        }
        if (haveNotice) sendPacket(notice);
        return answer;
    }
    tresult PLUGIN_API setProcessing(TBool enabled) override {
        if (ProcessAdmission::Scope::any()) return kResultFalse;
        std::lock_guard<std::mutex> lock(control_);
        if (enabled && (!active_.load() || admission_.closed())) return kResultFalse;
        if (processingCommandSerial_==std::numeric_limits<uint64_t>::max()) return kInternalError;
        ++processingCommandSerial_;
        if (processing_.load()!=(enabled!=0)) advanceMeterEpoch(meterStream_);
        processing_.store(enabled != 0);
        if (!enabled && !admission_.admitted()) status_.stopped(prepared_.load());
        return kResultOk;
    }

    tresult PLUGIN_API setState(IBStream* stream) override {
        if (ProcessAdmission::Scope::any()) return kResultFalse;
        addRef();
        std::unique_ptr<Processor,void(*)(Processor*)> self(this,[](Processor* value){value->release();});
        try {
            PluginState value;
            if (!readState(stream, value)) return kInvalidArgument;
            Protocol::Packet recall,notice;
            bool connected=false,haveNotice=false;
            {
                ProcessAdmission::ControlLease operation(lifecycleOperation_);
                if (!operation) return kResultFalse;
                std::optional<ActiveReconfiguration> live;
                {
                    std::lock_guard<std::mutex> lock(control_);
                    const auto before=std::atomic_load(&state_);
                    if (!publishLocked(std::move(value))) return kInvalidArgument;
                    ActiveReconfiguration intent;
                    if (captureActiveReconfigurationLocked(before->configuration,intent)) live=intent;
                }
                if (live) prepareActivation(true,&*live,notice,haveNotice);
            }
            {
                std::lock_guard<std::mutex> lock(control_);
                connected=peer_ && !Protocol::isZero(client_);
                if (connected) recall=snapshotLocked(Protocol::Kind::Recall,client_,0);
            }
            if (haveNotice) sendPacket(notice);
            if (connected) sendPacket(recall);
            return kResultOk;
        } catch (...) { return kInternalError; }
    }
    tresult PLUGIN_API getState(IBStream* stream) override {
        try {
            PluginState value;
            { std::lock_guard<std::mutex> lock(control_); value = currentStateLocked(); }
            return writeState(stream, value) ? kResultOk : kInternalError;
        } catch (...) { return kInternalError; }
    }
    tresult PLUGIN_API connect(IConnectionPoint* other) override {
        if (!other) return kInvalidArgument;
        Protocol::Packet ready;
        {
            std::lock_guard<std::mutex> lock(control_);
            if (peer_ || !freshEpoch(server_)) return kInvalidArgument;
            advanceMeterEpoch(meterConnection_);
            peer_ = other; peer_->addRef(); client_ = {}; latencyRouteSerial_=0;latencyHandlerReady_=false;
            clearEditLocked();multithread_.newBinding();
            ready.kind = Protocol::Kind::ConnectionReady;
            ready.processorInstance = instance_; ready.serverEpoch = server_;
        }
        try { sendPacket(ready); } catch (...) {}
        return kResultOk;
    }
    tresult PLUGIN_API disconnect(IConnectionPoint* other) override {
        IConnectionPoint* old = nullptr;
        {
            std::lock_guard<std::mutex> lock(control_);
            if (!peer_ || other != peer_) return kInvalidArgument;
            advanceMeterEpoch(meterConnection_);
            old = peer_; peer_ = nullptr; server_ = {}; client_ = {}; latencyRouteSerial_=0;latencyHandlerReady_=false;
            clearEditLocked();multithread_.newBinding();
        }
        old->release();
        return kResultOk;
    }
    tresult PLUGIN_API notify(IMessage* message) override {
        addRef();
        std::unique_ptr<Processor,void(*)(Processor*)> self(this,[](Processor* value){value->release();});
        if (!message || !message->getMessageID()) return kInvalidArgument;
        if (std::strcmp(message->getMessageID(), Protocol::kMessageId)) return kResultFalse;
        try {
            const void* bytes = nullptr; uint32 size = 0;
            auto* attributes = message->getAttributes();
            Protocol::Packet request;
            if (!attributes || attributes->getBinary(Protocol::kAttribute, bytes, size) != kResultOk
                || !Protocol::decode(bytes, size, request)) return kInvalidArgument;
            std::vector<uint8_t> editBytes;
            if (request.kind == Protocol::Kind::Edit)
                editBytes.assign(static_cast<const uint8_t*>(bytes), static_cast<const uint8_t*>(bytes) + size);
            Protocol::Packet reply,notice;
            bool haveNotice=false;
            std::optional<ProcessAdmission::ControlLease> operation;
            std::optional<ActiveReconfiguration> live;
            {
                std::lock_guard<std::mutex> lock(control_);
                if (!peer_) return kResultFalse;
                if (request.kind == Protocol::Kind::RequestSnapshot) {
                    const bool discovery = Protocol::isZero(request.serverEpoch);
                    if (!discovery) {
                        if (request.processorInstance != instance_ || request.serverEpoch != server_
                            || (!Protocol::isZero(client_) && request.clientEpoch != client_)) return kResultFalse;
                        if (client_!=request.clientEpoch) advanceMeterEpoch(meterConnection_);
                        if (!acceptLatencyRouteLocked(request)) return kResultFalse;
                        client_ = request.clientEpoch;
                    }
                    reply = snapshotLocked(Protocol::Kind::Snapshot, request.clientEpoch, request.correlation);
                } else {
                    if (request.processorInstance != instance_ || request.serverEpoch != server_
                        || Protocol::isZero(client_) || request.clientEpoch != client_) return kResultFalse;
                    if (request.kind == Protocol::Kind::Edit) {
                        const auto before=std::atomic_load(&state_);
                        const bool fresh=request.correlation>lastEditId_ && request.baseRevision==before->generation;
                        if (fresh && exclusiveStateMutationLocked(request.state)) {
                            if (ProcessAdmission::Scope::any()) return kResultFalse;
                            operation.emplace(lifecycleOperation_);
                            if (!*operation) return kResultFalse;
                        }
                        const auto accepted=editLocked(request, std::move(editBytes));
                        reply.editResult=accepted.result;reply.appliedRevision=accepted.applied;
                        if (operation && reply.editResult==Protocol::EditResult::Accepted) {
                            ActiveReconfiguration intent;
                            if (captureActiveReconfigurationLocked(before->configuration,intent)) live=intent;
                        }
                    } else if (request.kind == Protocol::Kind::RequestStatus) {
                        if (!acceptLatencyRouteLocked(request)) return kResultFalse;
                        reply = statusLocked(request);
                    } else if (request.kind == Protocol::Kind::MultithreadRequest) {
                        if (!multithread_.receive(request.correlation,request.multithreadRequested)) return kResultFalse;
                        reply.kind=Protocol::Kind::MultithreadStatus;
                        reply.processorInstance=instance_;reply.serverEpoch=server_;reply.clientEpoch=client_;
                        reply.correlation=request.correlation;
                        reply.multithreadAccepted=multithread_.accepted();
                        reply.multithreadStatusSequence=multithread_.sequence();
                        reply.multithreadRequested=multithread_.requested();
                        const auto current=std::atomic_load(&state_);
                        const bool graphCurrent=prepared_.load() && active_.load()
                            && preparedConfiguration_.load()==current->configuration;
                        reply.multithreadPreparedAttempt=multithread_.preparedAttempt(graphCurrent);
                        reply.multithreadPrepared=multithread_.prepared(graphCurrent);
                    } else return kResultFalse;
                }
            }
            if (live) prepareActivation(true,&*live,notice,haveNotice);
            operation.reset();
            if (request.kind==Protocol::Kind::Edit) {
                std::lock_guard<std::mutex> lock(control_);
                if (!peer_ || request.processorInstance!=instance_ || request.serverEpoch!=server_
                    || request.clientEpoch!=client_) return kResultFalse;
                auto current=snapshotLocked(Protocol::Kind::EditAck,request.clientEpoch,request.correlation);
                current.editResult=reply.editResult;current.appliedRevision=reply.appliedRevision;
                reply=std::move(current);
            }
            if (haveNotice) sendPacket(notice);
            return sendPacket(reply) ? kResultOk : kResultFalse;
        } catch (...) { return kInternalError; }
    }

    tresult PLUGIN_API process(ProcessData& data) override {
        ProcessAdmission::Scope scope(this);
        ProcessAdmission::Call call(admission_);
        if (!call) {
            // When admission is closed, inspect only caller-owned buffers and scalar bounds.
            const auto limit=audioBlockLimit_.load(std::memory_order_acquire);
            if (data.symbolicSampleSize!=kSample32 || data.numSamples<0 || !limit
                || static_cast<uint32>(data.numSamples)>limit) return kResultFalse;
            if (data.numSamples>0 && outputActive_.load()) {
                if (data.numOutputs!=1 || !data.outputs || data.outputs[0].numChannels<1
                    || data.outputs[0].numChannels>2 || !data.outputs[0].channelBuffers32) return kResultFalse;
                for (int32 c=0;c<data.outputs[0].numChannels;++c)
                    if (!data.outputs[0].channelBuffers32[c]) return kResultFalse;
                silence(data);
            }
            return active_.load() && failedStatus_.load()>=0 ? kResultOk : kResultFalse;
        }
        try { return processBlock(data); }
        catch (...) {
            silence(data); lastStatus_.store(kBridgePanic);
            conditionValid_ = false; conditionDirty_ = true;
            failedStatus_.store(kBridgePanic); prepared_.store(false);
            status_.observeProcess(kBridgePanic, false);
            return kInternalError;
        }
    }

private:
    struct EditAcceptance {
        Protocol::EditResult result=Protocol::EditResult::Accepted;
        uint64_t applied=0;
    };
    struct DrainSnapshot {
        uint64_t serial=0,configuration=0,setup=0,routeSerial=0;
        RocoPlugin* bridge=nullptr;
        IConnectionPoint* peer=nullptr;
        Protocol::Epoch server{},client{};
        bool routeReady=false;
    };
    struct ActiveReconfiguration {
        DrainSnapshot context;
        uint64_t processingCommand=0;
        bool processing=false;
    };
    void captureControlContextLocked(DrainSnapshot& value) const {
        value.serial=lifecycleSerial_;
        value.configuration=std::atomic_load(&state_)->configuration;
        value.setup=setupGeneration_;value.bridge=bridge_;
        value.peer=peer_;value.server=server_;value.client=client_;
        value.routeSerial=latencyRouteSerial_;value.routeReady=latencyHandlerReady_;
    }
    bool controlContextMatchesLocked(const DrainSnapshot& value,bool route) const {
        return lifecycleSerial_==value.serial && bridge_==value.bridge && setupGeneration_==value.setup
            && std::atomic_load(&state_)->configuration==value.configuration
            && (!route || (peer_==value.peer && server_==value.server && client_==value.client
                && latencyRouteSerial_==value.routeSerial && latencyHandlerReady_==value.routeReady));
    }
    bool exclusiveStateMutationLocked(const PluginState& value) const {
        const auto before=std::atomic_load(&state_);
        const auto quality=qualityForState(value,before->value,before->quality);
        return graphReprepareRequired(before->value,value,before->quality,quality)
            || hasConditionTarget(before->value.condition.mode)!=hasConditionTarget(value.condition.mode);
    }
    bool captureActiveReconfigurationLocked(uint64_t beforeConfiguration,ActiveReconfiguration& result) const {
        const auto current=std::atomic_load(&state_);
        if (current->configuration==beforeConfiguration || !active_.load() || !setupDone_
            || !hasConditionTarget(current->value.condition.mode)
            || current->value.analysisPath.empty() || current->value.userdataPath.empty()) return false;
        captureControlContextLocked(result.context);
        result.processingCommand=processingCommandSerial_;result.processing=processing_.load();
        return true;
    }

    tresult prepareActivation(TBool enabled, const ActiveReconfiguration* live,
        Protocol::Packet& notice, bool& haveNotice)
    {
        if (ProcessAdmission::Scope::any()) return kResultFalse;
        DrainSnapshot drain;
        {
            std::lock_guard<std::mutex> lifecycle(bridgeLifecycle_);
            std::lock_guard<std::mutex> lock(control_);
            if (!enabled) {
                admission_.close();
                if (lifecycleSerial_!=std::numeric_limits<uint64_t>::max()) ++lifecycleSerial_;
                advanceMeterEpoch(meterStream_);conditionResetRequested_.store(true,std::memory_order_release);
                processing_.store(false);active_.store(false);prepared_.store(false);
                if (!admission_.admitted()) status_.stopped(false);
                return kResultOk;
            }
            if (!bridge_ || !setupDone_ || (live ? !active_.load() : active_.load())) return kResultFalse;
            if (live && !controlContextMatchesLocked(live->context,true)) return kResultFalse;
            if (!closeForDrainLocked(drain)) return kInternalError;
            prepared_.store(false);
            if (!live) {processing_.store(false);active_.store(false);}
        }
        const auto deadline=std::chrono::steady_clock::now()+std::chrono::milliseconds(1000);
        if (!admission_.waitUntil(deadline)) {
            std::lock_guard<std::mutex> lock(control_);
            drainRefusedLocked("Audio processing did not quiesce before activation.",live!=nullptr);
            return kResultFalse;
        }
        // Read saved preferences on inactive activation after drain, never on the audio callback.
        uint32_t savedMultithread=0;
        const bool savedPreference=roco_plugin_try_read_multithread_preference(&savedMultithread)==kBridgeOk
            && savedMultithread<=1;
        std::lock_guard<std::mutex> lifecycle(bridgeLifecycle_);
        std::shared_ptr<const PublishedState> state;
        uint64_t attempt=0,setupGeneration=0,routeSerial=0;
        Protocol::Epoch routeServer{},routeClient{};
        IConnectionPoint* routePeer=nullptr;
        bool routeReady=false,requestedMultithread=true; uint32 oldD=6000;
        {
            std::lock_guard<std::mutex> lock(control_);
            if (!drainMatchesLocked(drain,true)) {
                drainRefusedLocked("Audio activation became stale while processing drained.",live!=nullptr);
                return kResultFalse;
            }
            state=std::atomic_load(&state_);setupGeneration=setupGeneration_;
            requestedMultithread=savedPreference ? savedMultithread!=0 : multithread_.requested();
            if (!live) {processing_.store(false);active_.store(false);}
            prepared_.store(false);
            if (!status_.beginAttempt(state->configuration)) return kInternalError;
            ProcessorStatus::Snapshot preparing;
            if (!status_.snapshot(preparing) || !preparing.attemptId) {
                status_.finishAttempt(kBridgeNotReady,false);return kInternalError;
            }
            attempt=preparing.attemptId;meterAttempt_.store(attempt);
            advanceMeterEpoch(meterStream_);conditionResetRequested_.store(true,std::memory_order_release);
            failedStatus_.store(kBridgeOk);cachedDetail_.clear();cachedDetailAttempt_=0;activationDetail_=nullptr;
            oldD=reportedLatency_.load(std::memory_order_relaxed);
            routeServer=server_;routeClient=client_;routePeer=peer_;routeSerial=latencyRouteSerial_;
            routeReady=peer_ && !Protocol::isZero(client_) && latencyHandlerReady_ && routeSerial;
        }
        auto rejectLocked=[&](int32 code,bool fromBridge,const char* detail) {
            prepared_.store(false);
            if (!live) {active_.store(false);processing_.store(false);}
            lastStatus_.store(code);if (code<0) failedStatus_.store(code);
            activationDetail_=detail;status_.finishAttempt(code,fromBridge);
        };
        auto reject=[&](int32 code,bool fromBridge,const char* detail) {
            std::lock_guard<std::mutex> lock(control_);rejectLocked(code,fromBridge,detail);
        };
        try {
            const bool wet=hasConditionTarget(state->value.condition.mode)
                && !state->value.analysisPath.empty() && !state->value.userdataPath.empty();
            uint32 candidateD=oldD;
            if (wet) {
                auto result=roco_plugin_set_condition(bridge_,&state->value.condition);
                RocoPrepareInfo info{};
                const auto policy=!routeReady && oldD==6000 ? RocoFixed6000 : RocoPreparedCount;
                if (result==kBridgeOk) result=roco_plugin_prepare(bridge_,state->value.analysisPath.c_str(),
                    state->value.userdataPath.c_str(),48000,setup_.maxSamplesPerBlock,0,
                    static_cast<uint32_t>(state->quality.profile),static_cast<uint32_t>(policy),
                    requestedMultithread?1u:0u,&info);
                if (result!=kBridgeOk) {reject(result,true,nullptr);return kResultFalse;}
                if (!info.delay_samples || (info.policy!=1 && info.policy!=2)
                    || (info.policy==1 && info.delay_samples!=6000)
                    || (policy==RocoFixed6000 && (info.delay_samples!=6000 || info.policy!=1))) {
                    reject(kBridgeFailed,false,"Prepared audio delay information was invalid.");return kResultFalse;
                }
                candidateD=info.delay_samples;
            }
            std::array<std::vector<float>,2> rings;
            for (auto& ring:rings) {
                if (candidateD>ring.max_size()) throw std::length_error("Prepared delay capacity");
                ring.assign(candidateD,0.f);
            }
            {
                std::lock_guard<std::mutex> lock(control_);
                auto current=std::atomic_load(&state_);ProcessorStatus::Snapshot actual;
                if (!drainMatchesLocked(drain,live!=nullptr) || !setupDone_ || setupGeneration_!=setupGeneration
                    || current->configuration!=state->configuration || !status_.snapshot(actual) || actual.attemptId!=attempt || actual.attemptConfig!=state->configuration) {
                    rejectLocked(kBridgeNotReady,false,"Audio preparation became stale before activation.");return kResultFalse;
                }
                const bool changed=candidateD!=reportedLatency_.load(std::memory_order_relaxed);
                if (changed && (!routeReady || peer_!=routePeer || server_!=routeServer || client_!=routeClient
                    || !latencyHandlerReady_ || latencyRouteSerial_!=routeSerial)) {
                    rejectLocked(kBridgeNotReady,false,"The host latency notification route changed or is unavailable.");return kResultFalse;
                }
                if (changed && latency_.epoch==std::numeric_limits<uint64_t>::max()) {
                    rejectLocked(kBridgeNotReady,false,"The audio latency declaration identifiers are exhausted.");return kResultFalse;
                }
                static_assert(noexcept(bypassDelay_.swap(rings)));
                bypassDelay_.swap(rings);bypassPosition_=0;audioGeneration_=~uint64_t(0);
                conditionResetRequested_.store(true,std::memory_order_release);
                preparedConfiguration_.store(state->configuration);preparedAttempt_=attempt;
                if (changed) {
                    ++latency_.epoch;latency_.samples=candidateD;
                    latency_.sourceConfig=state->configuration;latency_.sourceSetup=setupGeneration;
                    latencyTransportFailed_=false;
                    notice.kind=Protocol::Kind::LatencyChanged;notice.processorInstance=instance_;
                    notice.serverEpoch=server_;notice.clientEpoch=client_;notice.latency=latency_;haveNotice=true;
                }
                reportedLatency_.store(candidateD,std::memory_order_release);
                automationSequence_.store(0);activationDetail_=nullptr;
                lastStatus_.store(wet?kBridgeOk:kBridgeNotReady);
                multithread_.committed(wet && requestedMultithread,wet?attempt:0);
                if (live && processingCommandSerial_==live->processingCommand)
                    processing_.store(live->processing);
                prepared_.store(wet);status_.finishAttempt(wet?kBridgeOk:kBridgeNotReady,false);active_.store(true);
                admission_.openQuiescent();
            }
            return kResultOk;
        } catch (const std::bad_alloc&) {
            reject(kBridgeFailed,false,"Cannot allocate the prepared audio delay buffers.");return kOutOfMemory;
        } catch (const std::length_error&) {
            reject(kBridgeFailed,false,"Prepared audio delay buffers exceed supported capacity.");return kOutOfMemory;
        } catch (...) {
            reject(kBridgePanic,false,"Audio preparation failed before activation.");return kInternalError;
        }
    }

    bool closeForDrainLocked(DrainSnapshot& value) {
        admission_.close();
        if (lifecycleSerial_==std::numeric_limits<uint64_t>::max()) return false;
        ++lifecycleSerial_;
        captureControlContextLocked(value);
        return true;
    }
    bool drainMatchesLocked(const DrainSnapshot& value,bool route) const {
        return admission_.quiescent() && controlContextMatchesLocked(value,route);
    }
    void drainRefusedLocked(const char* detail,bool preserveActive=false) {
        if (!preserveActive) {processing_.store(false);active_.store(false);}
        prepared_.store(false);
        lastStatus_.store(kBridgeNotReady);activationDetail_=detail;status_.stopped(false);
    }
    bool acceptLatencyRouteLocked(const Protocol::Packet& request) {
        if (!request.latencyRouteSerial || request.latencyRouteSerial<latencyRouteSerial_
            || (request.latencyRouteSerial==latencyRouteSerial_ && request.latencyHandlerReady!=latencyHandlerReady_)) return false;
        latencyRouteSerial_=request.latencyRouteSerial;latencyHandlerReady_=request.latencyHandlerReady;return true;
    }
    static void advanceMeterEpoch(std::atomic<uint64_t>& value) noexcept {
        auto before=value.load();
        for (;;) {
            const auto after=before && before<std::numeric_limits<uint64_t>::max()?before+1:0;
            if (value.compare_exchange_weak(before,after)) return;
        }
    }
    bool publishLocked(PluginState value) {
        if (!validState(value)) return false;
        auto before = std::atomic_load(&state_);
        const auto quality=qualityForState(value,before->value,before->quality);
        const bool reconfigure=graphReprepareRequired(before->value,value,before->quality,quality);
        if (before->generation == std::numeric_limits<uint64_t>::max()
            || (reconfigure && before->configuration == std::numeric_limits<uint64_t>::max())) return false;
        auto after = std::make_shared<PublishedState>(); after->value = std::move(value);
        after->quality=quality;
        after->generation = before->generation + 1;
        after->configuration = before->configuration + (reconfigure ? 1 : 0);
        const auto afterInputChannel=after->value.inputChannel;
        const bool meterModeChanged=(before->value.condition.mode==kConditionEmpty)
            !=(after->value.condition.mode==kConditionEmpty);
        std::shared_ptr<const PublishedState> immutable = std::move(after);
        std::atomic_store(&state_, std::move(immutable));
        if (before->value.inputChannel!=afterInputChannel || meterModeChanged)
            advanceMeterEpoch(meterStream_);
        if (reconfigure) prepared_.store(false);
        return true;
    }
    PluginState currentStateLocked() {
        auto published = std::atomic_load(&state_);
        auto value = published->value;
        for (unsigned attempt = 0; attempt < 3; ++attempt) {
            const auto before = automationSequence_.load();
            if (before & 1) continue;
            const auto generation = automationGeneration_.load();
            const auto mask = automationMask_.load();
            std::array<double,8> snapshot;
            for (size_t id = 0; id < snapshot.size(); ++id) snapshot[id] = automation_[id].load();
            if (before != automationSequence_.load()) continue;
            savedAutomation_ = snapshot; savedAutomationMask_ = mask;
            savedAutomationGeneration_ = generation;
            break;
        }
        if (savedAutomationGeneration_ == published->generation) {
            const auto mask = savedAutomationMask_;
            for (uint32 id = 0; id < automation_.size(); ++id)
                if (mask & (uint32(1) << id)) setParameterValue(value, id, savedAutomation_[id]);
        }
        return value;
    }
    Protocol::Packet snapshotLocked(Protocol::Kind kind, const Protocol::Epoch& client, uint64_t correlation) {
        Protocol::Packet packet;
        packet.kind = kind; packet.processorInstance = instance_;
        packet.serverEpoch = server_; packet.clientEpoch = client; packet.correlation = correlation;
        auto state = std::atomic_load(&state_);
        packet.stateRevision = state->generation; packet.configToken = state->configuration;
        packet.state = currentStateLocked();packet.latency=latency_;
        return packet;
    }
    void clearEditLocked() {
        lastEditId_ = 0; lastEditBase_ = 0; lastAppliedRevision_ = 0;
        lastEditBytes_.clear(); lastEditResult_ = Protocol::EditResult::Accepted;
    }
    EditAcceptance editLocked(const Protocol::Packet& request, std::vector<uint8_t> bytes) {
        auto result = Protocol::EditResult::Accepted;
        uint64_t applied = 0;
        if (request.correlation < lastEditId_) result = Protocol::EditResult::StaleEditId;
        else if (request.correlation == lastEditId_) {
            if (request.baseRevision != lastEditBase_ || bytes != lastEditBytes_)
                result = Protocol::EditResult::ConflictingDuplicate;
            else { result = lastEditResult_; applied = lastAppliedRevision_; }
        } else {
            const auto before = std::atomic_load(&state_);
            if (request.baseRevision != before->generation) result = Protocol::EditResult::StaleBase;
            else {
                if (!publishLocked(request.state)) throw std::runtime_error("State revision exhausted or invalid state");
                applied = std::atomic_load(&state_)->generation;
            }
            lastEditId_ = request.correlation; lastEditBase_ = request.baseRevision;
            lastEditBytes_ = std::move(bytes); lastEditResult_ = result; lastAppliedRevision_ = applied;
        }
        return {result,applied};
    }
    Protocol::Packet statusLocked(const Protocol::Packet& request) {
        Protocol::Packet reply;
        reply.kind = Protocol::Kind::Status; reply.processorInstance = instance_;
        reply.serverEpoch = server_; reply.clientEpoch = client_; reply.correlation = request.correlation;
        auto current = std::atomic_load(&state_);
        reply.stateRevision = current->generation; reply.configToken = current->configuration;reply.latency=latency_;
        if (request.configToken != reply.configToken) {
            reply.statusResult = Protocol::StatusResult::StaleState;
            return reply;
        }
        ProcessorStatus::Snapshot state;
        if (!status_.snapshot(state)) { reply.statusResult = Protocol::StatusResult::Busy; return reply; }
        reply.attemptId = state.attemptId; reply.statusSequence = state.sequence;
        reply.phase = state.phase; reply.code = state.code;
        if (activationDetail_) reply.detail=activationDetail_;
        else if (latencyTransportFailed_) reply.detail="Host latency-change message delivery failed; the declaration remains available for catchup.";
        if (const auto* issue=qualityIssue(current->quality)) {
            reply.phase=Protocol::Phase::NotPrepared;reply.code=kBridgeNotReady;
            reply.detail=issue;return reply;
        }
        const bool emptyMeter=current->value.condition.mode==kConditionEmpty;
        if (state.attemptConfig != current->configuration) {
            reply.phase = Protocol::Phase::NotPrepared; reply.code = kBridgeNotReady;
            if (current->value.condition.mode == kConditionUnavailable)
                reply.detail = kUnavailableConditionDetail;
            if (!emptyMeter || state.phase==Protocol::Phase::Faulted) return reply;
        }
        if (!hasConditionTarget(current->value.condition.mode) && state.phase != Protocol::Phase::Faulted) {
            reply.phase = Protocol::Phase::NotPrepared; reply.code = kBridgeNotReady;
            if (current->value.condition.mode == kConditionUnavailable)
                reply.detail = kUnavailableConditionDetail;
            if (!emptyMeter) return reply;
        }
        if (state.phase != Protocol::Phase::Faulted) {
            const auto meterNow=std::chrono::steady_clock::now();
            InputMeter::Sample meter;
            const bool sampled=inputMeter_.snapshot(meter);
            int64_t age=-1;
            if (sampled && meter.request) for (const auto& marker:meterMarkers_)
                if (marker.id==meter.request) {
                    age=std::chrono::ceil<std::chrono::milliseconds>(meterNow-marker.issued).count();
                    break;
                }
            const bool meterReady=(!emptyMeter && state.phase==Protocol::Phase::Processing
                    && state.code==kBridgeOk)
                || (emptyMeter && reply.phase==Protocol::Phase::NotPrepared
                    && reply.code==kBridgeNotReady && state.attemptId!=0
                    && state.attemptId==meterAttempt_.load()
                    && failedStatus_.load()>=0);
            if (meterReady && processing_.load() && active_.load() && sampled
                && meter.configuration==reply.configToken
                && meter.attempt==reply.attemptId && meter.connection==meterConnection_.load()
                && meter.stream==meterStream_.load() && meterRequest_.load()!=0 && age>=0 && age<=100) {
                reply.meterAvailable=true; reply.meterStream=meter.stream;
                reply.meterOrdinal=meter.ordinal; reply.meterFrames=meter.frames;
                reply.meterEnvelopeBits=meter.envelopeBits;
                const auto busNow=inputActive_.snapshot();
                if ((busNow>>1)!=0 && (busNow&1)!=0 && busNow==meter.sourceBus) {
                    reply.meterPitch=meter.pitch;reply.meterPitchOrdinal=meter.pitchOrdinal;
                }
                reply.meterAgeUpperMs=static_cast<uint32_t>(age);
            }
            advanceMeterEpoch(meterRequest_);
            meterMarkers_[nextMeterMarker_]={meterRequest_.load(),meterNow};
            nextMeterMarker_=(nextMeterMarker_+1)%meterMarkers_.size();
            return reply;
        }
        // Drain the audio callback before reading fatal text; bridgeLifecycle_ excludes mutation.
        if (admission_.admitted()) { reply.statusResult = Protocol::StatusResult::Busy; return reply; }
        std::unique_lock<std::mutex> lifecycle(bridgeLifecycle_, std::try_to_lock);
        if (!lifecycle.owns_lock()) { reply.statusResult = Protocol::StatusResult::Busy; return reply; }
        if (cachedDetailAttempt_ != state.attemptId) {
            cachedDetail_.clear();
            if (state.bridgeDetailReady) {
                const auto needed = roco_plugin_copy_error(bridge_, nullptr, 0);
                constexpr size_t reserve = 32;
                const auto capacity = std::min(needed, Protocol::kMaximumDetailBytes - reserve + 1);
                if (capacity > 1) {
                    std::vector<char> bytes(capacity, '\0');
                    roco_plugin_copy_error(bridge_, bytes.data(), bytes.size());
                    cachedDetail_.assign(bytes.data());
                    if (needed > capacity) {
                        while (!cachedDetail_.empty() && (static_cast<unsigned char>(cachedDetail_.back()) & 0xc0) == 0x80)
                            cachedDetail_.pop_back();
                        if (!cachedDetail_.empty() && static_cast<unsigned char>(cachedDetail_.back()) >= 0x80)
                            cachedDetail_.pop_back();
                        cachedDetail_ += "\n[Message truncated.]";
                    }
                }
            }
            if (cachedDetail_.empty() && activationDetail_) cachedDetail_=activationDetail_;
            if (cachedDetail_.empty()) cachedDetail_ = state.bridgeDetailReady
                ? "The audio engine failed without a diagnostic message."
                : "The processor operation failed before a bridge diagnostic was available.";
            cachedDetailAttempt_ = state.attemptId;
        }
        reply.detail = cachedDetail_;
        return reply;
    }
    bool sendPacket(const Protocol::Packet& packet) {
        addRef();
        std::unique_ptr<Processor,void(*)(Processor*)> heldSelf(this,[](Processor* value){value->release();});
        bool accepted=false;
        const auto deliver=[&]()->bool {
            std::vector<uint8_t> bytes;
            if (!Protocol::encode(packet,bytes)) return false;
            IConnectionPoint* destination=nullptr;FUnknown* context=nullptr;
            {
                std::lock_guard<std::mutex> lock(control_);
                if (!context_ || !peer_) return false;
                destination=peer_;destination->addRef();context=context_;context->addRef();
            }
            std::unique_ptr<IConnectionPoint,void(*)(IConnectionPoint*)> heldPeer(destination,[](IConnectionPoint* v){v->release();});
            std::unique_ptr<FUnknown,void(*)(FUnknown*)> heldContext(context,[](FUnknown* v){v->release();});
            IHostApplication* host=nullptr;
            if (context->queryInterface(IHostApplication_iid,reinterpret_cast<void**>(&host))==kResultOk && host) {
                std::unique_ptr<IHostApplication,void(*)(IHostApplication*)> heldHost(host,[](IHostApplication* v){v->release();});
                IMessage* raw=nullptr;TUID id;std::memcpy(id,IMessage_iid,16);
                if (host->createInstance(id,id,reinterpret_cast<void**>(&raw))==kResultOk && raw) {
                    std::unique_ptr<IMessage,void(*)(IMessage*)> message(raw,[](IMessage* v){v->release();});
                    raw->setMessageID(Protocol::kMessageId);auto* attributes=raw->getAttributes();
                    if (attributes && attributes->setBinary(Protocol::kAttribute,bytes.data(),static_cast<uint32>(bytes.size()))==kResultOk)
                        return destination->notify(raw)==kResultOk;
                }
            }
            return false;
        };
        try {accepted=deliver();} catch (...) {accepted=false;}
        if (packet.latency.epoch) {
            std::lock_guard<std::mutex> lock(control_);
            if (packet.processorInstance==instance_ && packet.serverEpoch==server_ && packet.clientEpoch==client_
                && packet.latency==latency_) latencyTransportFailed_=!accepted;
        }
        return accepted;
    }
    static void silence(ProcessData& data) {
        if (!data.outputs || data.numSamples <= 0) return;
        for (int32 bus = 0; bus < data.numOutputs; ++bus) {
            auto& out = data.outputs[bus];
            if (out.channelBuffers32) for (int32 channel = 0; channel < out.numChannels; ++channel)
                if (out.channelBuffers32[channel]) std::fill_n(out.channelBuffers32[channel], data.numSamples, 0.f);
            out.silenceFlags = out.numChannels == 2 ? 3 : 1;
        }
    }
    int32 publishConditionIfChanged(uint64_t configuration, bool& calledBridge) {
        if (conditionResetRequested_.load(std::memory_order_acquire)
            && conditionResetRequested_.exchange(false, std::memory_order_acq_rel)) {
            conditionValid_ = false; conditionDirty_ = true;
        }
        if (conditionValid_ && lastConditionConfiguration_ != configuration) {
            conditionValid_ = false; conditionDirty_ = true;
        }
        if (conditionValid_ && !conditionDirty_) return kBridgeOk;
        const auto& condition = audioState_.condition;
        if (conditionValid_ && !std::memcmp(lastSuccessfulConditionBits_.data(),
                &condition, sizeof(condition))) {
            conditionDirty_ = false;
            return kBridgeOk;
        }
        calledBridge = true;
        const auto result = roco_plugin_set_condition(bridge_, &condition);
        if (result == kBridgeOk) {
            std::memcpy(lastSuccessfulConditionBits_.data(), &condition, sizeof(condition));
            lastConditionConfiguration_ = configuration;
            conditionValid_ = true; conditionDirty_ = false;
        } else {
            conditionValid_ = false; conditionDirty_ = true;
        }
        return result;
    }
    tresult processBlock(ProcessData& data) {
        const auto meterEntryStream=meterStream_.load();
        const auto meterEntryConnection=meterConnection_.load();
        const auto meterEntryAttempt=meterAttempt_.load();
        const auto meterEntryRequest=meterRequest_.load();
        const bool meterEntryProcessing=processing_.load();
        if (data.symbolicSampleSize != kSample32 || data.numSamples < 0 || data.numSamples > setup_.maxSamplesPerBlock) {
            return kInvalidArgument;
        }
        auto published = std::atomic_load(&state_);
        if (audioGeneration_ != published->generation) {
            audioState_.condition = published->value.condition;
            conditionDirty_ = true;
            audioState_.bypass = published->value.bypass; audioState_.inputChannel = published->value.inputChannel;
            audioAutomationMask_ = 0;
            audioGeneration_ = published->generation;
        }
        std::array<IParamValueQueue*,8> queues{}; std::array<int32,8> points{};
        if (data.inputParameterChanges) {
            for (int32 i = 0; i < data.inputParameterChanges->getParameterCount(); ++i) {
                auto* queue = data.inputParameterChanges->getParameterData(i);
                if (queue && queue->getParameterId() < queues.size() && isHostParameter(queue->getParameterId()))
                    queues[queue->getParameterId()] = queue;
            }
        }
        const auto count = data.numSamples;
        if (count > 0 && (!active_.load() || !processing_.load())) { silence(data); return kResultFalse; }
        InputMeter::Sample meterSample;
        bool meterCollected=false;
        uint64_t pitchBusEntry=0;
        if (count > 0) {
            if (outputActive_.load()) {
                if (data.numOutputs != 1 || !data.outputs || data.outputs[0].numChannels != outputChannels_
                    || !data.outputs[0].channelBuffers32) return kInvalidArgument;
                for (int32 c = 0; c < outputChannels_; ++c) if (!data.outputs[0].channelBuffers32[c]) return kInvalidArgument;
            }
            for (auto& channel : input_) std::fill_n(channel.data(), count, 0.f);
            pitchBusEntry=inputActive_.snapshot();
            if ((pitchBusEntry & 1) != 0) {
                if (data.numInputs != 1 || !data.inputs || data.inputs[0].numChannels != inputChannels_
                    || !data.inputs[0].channelBuffers32) { silence(data); return kInvalidArgument; }
                for (int32 c = 0; c < inputChannels_; ++c) {
                    if (!data.inputs[0].channelBuffers32[c]) { silence(data); return kInvalidArgument; }
                    std::copy_n(data.inputs[0].channelBuffers32[c], count, input_[c].data());
                }
                if (inputChannels_ == 1) std::copy_n(input_[0].data(), count, input_[1].data());
            }
            for (const auto& channel : input_)
                if (std::any_of(channel.begin(), channel.begin() + count, [](float value) { return !std::isfinite(value); })) {
                    silence(data); return kInvalidArgument;
                }
            const bool emptyMeter=audioState_.condition.mode==kConditionEmpty;
            if ((emptyMeter || (hasConditionTarget(audioState_.condition.mode) && prepared_.load()))
                && failedStatus_.load()>=0
                && (emptyMeter ? meterEntryAttempt!=0 : preparedConfiguration_.load()==published->configuration)
                && meterEntryProcessing && meterEntryStream==meterStream_.load()
                && meterEntryConnection==meterConnection_.load()
                && meterEntryAttempt==meterAttempt_.load()
                && audioState_.inputChannel<static_cast<uint32>(inputChannels_)) {
                float gain=1.f;
                meterSample.revision=published->generation;
                meterSample.configuration=published->configuration;
                meterSample.attempt=meterEntryAttempt;
                meterSample.connection=meterEntryConnection;
                meterSample.stream=meterEntryStream;
                meterSample.sourceBus=pitchBusEntry;
                meterSample.request=meterEntryRequest;
                meterSample.frames=static_cast<uint32_t>(count);
                if (emptyMeter || roco_plugin_input_gain(bridge_,&gain)==kBridgeOk)
                    meterCollected=inputMeter_.inputBlock(input_[audioState_.inputChannel].data(),
                        static_cast<uint32_t>(count),gain);
            }
            for (int32 i = 0; i < count; ++i) {
                for (size_t c = 0; c < 2; ++c) {
                    dry_[c][i] = bypassDelay_[c][bypassPosition_];
                    bypassDelay_[c][bypassPosition_] = input_[c][i];
                }
                if (++bypassPosition_ == bypassDelay_[0].size()) bypassPosition_ = 0;
            }
        }
        RocoPitch blockPitch{};
        bool pitchSourceChanged=false;
        int32 cursor = 0;
        do {
            int32 next = count;
            for (size_t id = 0; id < queues.size(); ++id) {
                auto* queue = queues[id];
                if (!queue) continue;
                while (points[id] < queue->getPointCount()) {
                    int32 offset = 0; ParamValue value = 0;
                    if (queue->getPoint(points[id], offset, value) != kResultOk || offset < 0 || offset > count) {
                        silence(data); return kInvalidArgument;
                    }
                    if (offset > cursor) { next = std::min(next, offset); break; }
                    if (!setParameterValue(audioState_, static_cast<ParamID>(id), value)) {
                        silence(data); return kInvalidArgument;
                    }
                    if ((id >= kPregain && id <= kToneHue) || id == kTargetMode)
                        conditionDirty_ = true;
                    if (id==kInputChannel) pitchSourceChanged=true;
                    audioAutomationMask_ |= uint32(1) << id;
                    ++points[id];
                }
            }
            if (cursor == count) break;
            if (next <= cursor) { silence(data); return kInvalidArgument; }
            const auto frames = static_cast<uint32>(next - cursor);
            std::fill_n(wet_[0].data() + cursor, frames, 0.f);
            std::fill_n(wet_[1].data() + cursor, frames, 0.f);
            bool calledBridge = false;
            int32 result = failedStatus_.load() < 0 ? failedStatus_.load() : kBridgeNotReady;
            if (result >= 0 && hasConditionTarget(audioState_.condition.mode)
                && prepared_.load() && preparedConfiguration_.load() == published->configuration) {
                result = publishConditionIfChanged(published->configuration, calledBridge);
                if (result == kBridgeOk) {
                    if (audioState_.inputChannel >= static_cast<uint32>(inputChannels_)) {
                        silence(data); return kInvalidArgument;
                    }
                    const auto selected = audioState_.inputChannel;
                    calledBridge = true;
                    result = roco_plugin_process(bridge_, input_[selected].data() + cursor, nullptr,
                        wet_[0].data() + cursor, wet_[1].data() + cursor, frames, 1, 2, meterEntryStream, selected, pitchBusEntry, &blockPitch);
                }
            }
            lastStatus_.store(result);
            if (result < 0) {
                conditionValid_ = false; conditionDirty_ = true;
                failedStatus_.store(result); prepared_.store(false);
            }
            status_.observeProcess(result, calledBridge);
            if (outputActive_.load()) {
                for (int32 c = 0; c < outputChannels_; ++c) {
                    const bool unloaded = audioState_.condition.mode == kConditionEmpty && result >= 0;
                    const auto* source = audioState_.bypass || unloaded ? dry_[c].data() : wet_[c].data();
                    std::copy_n(source + cursor, frames, data.outputs[0].channelBuffers32[c] + cursor);
                }
            } else if (data.numOutputs > 0 && data.outputs && data.outputs[0].channelBuffers32) {
                for (int32 c = 0; c < data.outputs[0].numChannels; ++c)
                    if (data.outputs[0].channelBuffers32[c]) std::fill_n(data.outputs[0].channelBuffers32[c] + cursor, frames, 0.f);
            }
            cursor = next;
        } while (cursor <= count);
        automationSequence_.fetch_add(1);
        for (uint32 id = 0; id < automation_.size(); ++id) automation_[id].store(parameterValue(audioState_, id));
        automationMask_.store(audioAutomationMask_);
        automationGeneration_.store(audioGeneration_, std::memory_order_release);
        automationSequence_.fetch_add(1);
        if (count > 0 && data.numOutputs > 0 && data.outputs)
            data.outputs[0].silenceFlags = outputActive_.load() ? 0 : (outputChannels_ == 2 ? 3 : 1);
        if (meterCollected && failedStatus_.load()>=0) {
            if (!pitchSourceChanged && (pitchBusEntry>>1)!=0 && (pitchBusEntry&1)!=0
                && pitchBusEntry==inputActive_.snapshot() && meterEntryStream==meterStream_.load()
                && hasConditionTarget(audioState_.condition.mode) && prepared_.load()) {
                meterSample.pitch=blockPitch.word;meterSample.pitchOrdinal=blockPitch.ordinal;
            }
            inputMeter_.publish(meterSample);
        }
        return failedStatus_.load() < 0 ? kResultFalse : kResultOk;
    }

    std::mutex bridgeLifecycle_;
    std::mutex control_;
    Protocol::Epoch instance_{}, server_{}, client_{};
    uint64_t lastEditId_ = 0, lastEditBase_ = 0, lastAppliedRevision_ = 0;
    Protocol::EditResult lastEditResult_ = Protocol::EditResult::Accepted;
    std::vector<uint8_t> lastEditBytes_;
    ProcessorStatus status_;
    Protocol::MultithreadServer multithread_;
    std::atomic<uint32> reportedLatency_{6000};
    Protocol::LatencyDeclaration latency_;
    uint64_t setupGeneration_=0,preparedAttempt_=0,latencyRouteSerial_=0;
    uint64_t processingCommandSerial_=0;
    bool latencyHandlerReady_=false,latencyTransportFailed_=false;
    const char* activationDetail_=nullptr;
    InputMeter inputMeter_;
    std::atomic<uint64_t> meterConnection_{1}, meterStream_{1}, meterAttempt_{0}, meterRequest_{1};
    struct MeterMarker {uint64_t id=0;std::chrono::steady_clock::time_point issued{};};
    std::array<MeterMarker,64> meterMarkers_{};
    size_t nextMeterMarker_=0;
    std::string cachedDetail_;
    uint64_t cachedDetailAttempt_ = 0;
    ProcessAdmission::Gate admission_;
    std::atomic_flag lifecycleOperation_=ATOMIC_FLAG_INIT;
    uint64_t lifecycleSerial_=0;
    std::atomic<uint32> references_{1};
    FUnknown* context_ = nullptr;
    IConnectionPoint* peer_ = nullptr;
    RocoPlugin* bridge_ = nullptr;
    std::shared_ptr<const PublishedState> state_;
    std::atomic<bool> active_{false}, processing_{false}, prepared_{false};
    std::atomic<uint32> audioBlockLimit_{0};
    InputBusState inputActive_{false};
    std::atomic<bool> outputActive_{false};
    std::atomic<uint64_t> preparedConfiguration_{0}, automationGeneration_{~uint64_t(0)};
    std::atomic<uint64_t> automationSequence_{0};
    std::array<std::atomic<double>,8> automation_;
    std::atomic<uint32> automationMask_{0};
    PluginState audioState_;
    std::atomic<bool> conditionResetRequested_{true};
    bool conditionValid_ = false, conditionDirty_ = true;
    uint64_t lastConditionConfiguration_ = 0;
    std::array<unsigned char, sizeof(RocoCondition)> lastSuccessfulConditionBits_{};
    uint64_t audioGeneration_ = ~uint64_t(0);
    uint32 audioAutomationMask_ = 0;
    std::array<double,8> savedAutomation_{};
    uint32 savedAutomationMask_ = 0;
    uint64_t savedAutomationGeneration_ = ~uint64_t(0);
    std::atomic<int32> lastStatus_{kBridgeNotReady};
    std::atomic<int32> failedStatus_{kBridgeOk};
    ProcessSetup setup_{};
    bool setupDone_ = false;
    int32 inputChannels_ = 2, outputChannels_ = 2;
    std::array<std::vector<float>,2> input_, wet_, dry_, bypassDelay_;
    size_t bypassPosition_ = 0;
};
}

Steinberg::FUnknown* createProcessor() {
    return static_cast<Steinberg::Vst::IComponent*>(new Processor());
}
}
