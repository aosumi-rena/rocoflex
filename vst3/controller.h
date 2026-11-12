#pragma once

#include "plugin.h"
#include "plugin_protocol.h"
#include "input_meter.h"
#include "pluginterfaces/vst/ivsteditcontroller.h"
#include "pluginterfaces/vst/ivstmessage.h"
#include "pluginterfaces/vst/ivstunits.h"
#include <atomic>
#include <chrono>
#include <optional>

namespace Rocoflex {

class Controller final : public Steinberg::Vst::IEditController,
                         public Steinberg::Vst::IConnectionPoint,
                         public Steinberg::Vst::IUnitInfo {
public:
    Controller() = default;
    ~Controller();
    Steinberg::tresult PLUGIN_API queryInterface(const Steinberg::TUID iid, void** object) override;
    Steinberg::uint32 PLUGIN_API addRef() override;
    Steinberg::uint32 PLUGIN_API release() override;
    Steinberg::tresult PLUGIN_API initialize(Steinberg::FUnknown* context) override;
    Steinberg::tresult PLUGIN_API terminate() override;
    Steinberg::tresult PLUGIN_API setComponentState(Steinberg::IBStream* stream) override;
    Steinberg::tresult PLUGIN_API setState(Steinberg::IBStream* stream) override;
    Steinberg::tresult PLUGIN_API getState(Steinberg::IBStream* stream) override;
    Steinberg::int32 PLUGIN_API getParameterCount() override;
    Steinberg::tresult PLUGIN_API getParameterInfo(Steinberg::int32 index,
        Steinberg::Vst::ParameterInfo& info) override;
    Steinberg::tresult PLUGIN_API getParamStringByValue(Steinberg::Vst::ParamID id,
        Steinberg::Vst::ParamValue value, Steinberg::Vst::String128 text) override;
    Steinberg::tresult PLUGIN_API getParamValueByString(Steinberg::Vst::ParamID id,
        Steinberg::Vst::TChar* text, Steinberg::Vst::ParamValue& value) override;
    Steinberg::Vst::ParamValue PLUGIN_API normalizedParamToPlain(Steinberg::Vst::ParamID id,
        Steinberg::Vst::ParamValue value) override;
    Steinberg::Vst::ParamValue PLUGIN_API plainParamToNormalized(Steinberg::Vst::ParamID id,
        Steinberg::Vst::ParamValue value) override;
    Steinberg::Vst::ParamValue PLUGIN_API getParamNormalized(Steinberg::Vst::ParamID id) override;
    Steinberg::tresult PLUGIN_API setParamNormalized(Steinberg::Vst::ParamID id,
        Steinberg::Vst::ParamValue value) override;
    Steinberg::tresult PLUGIN_API setComponentHandler(Steinberg::Vst::IComponentHandler* handler) override;
    Steinberg::IPlugView* PLUGIN_API createView(Steinberg::FIDString name) override;
    Steinberg::tresult PLUGIN_API connect(Steinberg::Vst::IConnectionPoint* other) override;
    Steinberg::tresult PLUGIN_API disconnect(Steinberg::Vst::IConnectionPoint* other) override;
    Steinberg::tresult PLUGIN_API notify(Steinberg::Vst::IMessage* message) override;

    Steinberg::int32 PLUGIN_API getUnitCount() override;
    Steinberg::tresult PLUGIN_API getUnitInfo(Steinberg::int32 index, Steinberg::Vst::UnitInfo& info) override;
    Steinberg::int32 PLUGIN_API getProgramListCount() override;
    Steinberg::tresult PLUGIN_API getProgramListInfo(Steinberg::int32 index, Steinberg::Vst::ProgramListInfo& info) override;
    Steinberg::tresult PLUGIN_API getProgramName(Steinberg::Vst::ProgramListID list, Steinberg::int32 index, Steinberg::Vst::String128 name) override;
    Steinberg::tresult PLUGIN_API getProgramInfo(Steinberg::Vst::ProgramListID list, Steinberg::int32 index, Steinberg::Vst::CString attribute, Steinberg::Vst::String128 value) override;
    Steinberg::tresult PLUGIN_API hasProgramPitchNames(Steinberg::Vst::ProgramListID list, Steinberg::int32 index) override;
    Steinberg::tresult PLUGIN_API getProgramPitchName(Steinberg::Vst::ProgramListID list, Steinberg::int32 index, Steinberg::int16 pitch, Steinberg::Vst::String128 name) override;
    Steinberg::Vst::UnitID PLUGIN_API getSelectedUnit() override;
    Steinberg::tresult PLUGIN_API selectUnit(Steinberg::Vst::UnitID unit) override;
    Steinberg::tresult PLUGIN_API getUnitByBus(Steinberg::Vst::MediaType type, Steinberg::Vst::BusDirection direction, Steinberg::int32 bus, Steinberg::int32 channel, Steinberg::Vst::UnitID& unit) override;
    Steinberg::tresult PLUGIN_API setUnitProgramData(Steinberg::int32 listOrUnit, Steinberg::int32 index, Steinberg::IBStream* stream) override;

    struct LatencyTurn {
        Controller& owner;
        explicit LatencyTurn(Controller& value):owner(value) {
            if (!owner.latencyUiDepth_++) {owner.latencyAttempts_=0;owner.latencyFailedThisTurn_=false;}
        }
        ~LatencyTurn() {--owner.latencyUiDepth_;}
        LatencyTurn(const LatencyTurn&) = delete;
        LatencyTurn& operator=(const LatencyTurn&) = delete;
    };

    RocoPlugin* editorBridge() const { return bridge_; }
    const PluginState& state() const { return state_; }
    uint64_t stateRevision() const { return stateRevision_; }
    uint64_t stateSerial() const { return changeSerial_; }
    bool syncEditor(RocoEditor* editor, uint64_t& editorRevision);
    void serviceMessages(bool immediately = false, bool editorOwner = false, bool policyReply = false);
    void syncMultithreadEdits();
    void deliverMultithread(RocoEditor* editor);
    void deliverInputMeter(RocoEditor* editor);
    void deliverError(RocoEditor* editor, void* parent);
    void editorFailure(void* parent, const char* fallback, bool useRustError);
    void captureEditorError(const char* fallback, std::optional<uint64_t> restoreCandidate = std::nullopt);
    void clearEditorError(uint64_t restoreCandidate);

private:
    using Packet = ProtocolV4::Packet;
    using Epoch = ProtocolV4::Epoch;
    enum class LinkStage { Disconnected, Discovering, Confirming, Bound };
    struct Flight { Packet packet; bool reload = false; };
    struct FailureKey {
        Epoch instance{}; uint64_t config = 0, attempt = 0, sequence = 0;
        bool operator==(const FailureKey& b) const {
            return instance == b.instance && config == b.config && attempt == b.attempt && sequence == b.sequence;
        }
    };
    struct ErrorEvent {
        uint64_t id = 0; std::string text; std::optional<FailureKey> failure;
        std::optional<uint64_t> restoreCandidate;
    };
    bool applyState(PluginState state, bool notifyPeer, bool hostRecall = false);
    bool invalidateEditorSession();
    bool installState(PluginState state);
    bool installState(PluginState state, QualityChoice quality);
    bool sendPacket(const Packet& packet);
    bool receivePacket(const Packet& packet);
    bool adoptSnapshot(const Packet& packet);
    bool freshInputMeter() const;
    bool adoptLatency(const Packet& packet);
    void resetLatencyBinding();
    void retireLatencyError(const char* text);
    void advertiseLatencyRoute();
    void maybeNotifyLatency();
    void refreshMultithreadPreference();
    void beginSnapshot(bool newConnection);
    void cancelPending();
    bool queueError(std::string text, std::optional<FailureKey> failure = std::nullopt,
        std::optional<uint64_t> restoreCandidate = std::nullopt);
    uint64_t nextId();
    void markDirty();
    std::atomic<Steinberg::uint32> references_ {1};
    Steinberg::FUnknown* context_ = nullptr;
    Steinberg::Vst::IComponentHandler* handler_ = nullptr;
    Steinberg::Vst::IConnectionPoint* peer_ = nullptr;
    RocoPlugin* bridge_ = nullptr;
    bool unitBackingSeen_ = false;
    PluginState state_;
    QualityChoice quality_;
    QualityChoice acceptedQuality_;
    ProtocolV4::MultithreadClient multithread_;
    std::chrono::steady_clock::time_point lastMultithreadPoll_{},lastPreferenceCheck_{},nextMultithreadRetry_{};
    uint64_t stateRevision_ = 0, changeSerial_ = 0, conditionRevision_ = 0;
    LinkStage link_ = LinkStage::Disconnected;
    Epoch instance_{}, server_{}, client_{}, statusInstance_{};
    uint64_t connectionSerial_ = 0, wireGeneration_ = 0, nextId_ = 0;
    uint64_t processorRevision_ = 0, configToken_ = 0;
    Packet snapshotRequest_;
    std::optional<Flight> flight_;
    bool unsent_ = false, queuedReload_ = false, reloadPending_ = false;
    uint64_t reloadRevision_ = 0;
    bool servicing_ = false;
    std::optional<ProtocolV4::LatencyDeclaration> latency_,latencyDelivered_;
    uint64_t latencyRouteSerial_=1,latencyRouteRequest_=0;
    bool latencyRouteDirty_=true,latencyAdvertising_=false,latencyDispatching_=false;
    bool latencyDispatchRequested_=false,latencyFailedThisTurn_=false;
    unsigned latencyUiDepth_=0,latencyAttempts_=0;
    std::chrono::steady_clock::time_point lastWireAttempt_{}, lastStatusPoll_{};
    uint64_t statusRequest_ = 0, statusRevision_ = 0, statusConfig_ = 0;
    uint64_t latestAttempt_ = 0, latestStatusSequence_ = 0;
    bool meterAvailable_ = false;
    float meterEnvelope_ = 0.f;
    PitchObservation pitchObservation_;
    uint64_t meterStream_ = 0, meterOrdinal_ = 0;
    uint64_t meterConfiguration_ = 0, meterAttempt_ = 0;
    std::chrono::steady_clock::time_point meterObserved_{}, statusIssued_{};
    std::optional<FailureKey> lastFailure_;
    std::optional<ErrorEvent> error_;
    uint64_t nextErrorId_ = 0, acknowledgedError_ = 0;
    std::string lastLocalError_;
    std::optional<uint64_t> lastLocalCandidate_;
};

}
