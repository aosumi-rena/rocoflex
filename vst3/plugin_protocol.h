#pragma once

#include "plugin.h"
#include <array>
#include <limits>

namespace Rocoflex::ProtocolV4 {

inline constexpr char kMessageId[] = "Rocoflex.Protocol.v4";
inline constexpr char kAttribute[] = "packet";
inline constexpr uint32_t kMagic = 0x34564352;
inline constexpr uint32_t kVersion = 4;
inline constexpr size_t kMaximumDetailBytes = 16384;
inline constexpr size_t kMaximumPacketBytes = kMaximumStateBytes + 192;
using Epoch = std::array<uint8_t, 16>;

enum class Kind : uint32_t {
    ConnectionReady = 1, RequestSnapshot, Snapshot, Recall,
    Edit, EditAck, RequestStatus, Status, LatencyChanged,
    MultithreadRequest, MultithreadStatus
};
enum class EditResult : uint32_t {
    Accepted = 0, StaleBase = 1, StaleEditId = 2, ConflictingDuplicate = 3
};
enum class StatusResult : uint32_t { Current = 0, StaleState = 1, Busy = 2 };
enum class Phase : uint32_t {
    NotPrepared = 0, Preparing = 1, Ready = 2, Processing = 3, Faulted = 4
};

struct LatencyDeclaration {
    uint64_t epoch = 0;
    uint32_t samples = 6000;
    uint64_t sourceConfig = 0, sourceSetup = 0;
    bool operator==(const LatencyDeclaration& b) const noexcept {
        return epoch==b.epoch && samples==b.samples && sourceConfig==b.sourceConfig && sourceSetup==b.sourceSetup;
    }
    bool operator!=(const LatencyDeclaration& b) const noexcept { return !(*this==b); }
};
struct Packet {
    Kind kind = Kind::ConnectionReady;
    Epoch processorInstance{};
    Epoch serverEpoch{};
    Epoch clientEpoch{};
    uint64_t correlation = 0;

    LatencyDeclaration latency;
    bool latencyHandlerReady = false;
    uint64_t latencyRouteSerial = 0;

    uint64_t multithreadAccepted = 0, multithreadStatusSequence = 0, multithreadPreparedAttempt = 0;
    bool multithreadRequested = true, multithreadPrepared = false;

    uint64_t stateRevision = 0;
    uint64_t configToken = 0;
    PluginState state;

    uint64_t baseRevision = 0;
    uint64_t appliedRevision = 0;
    EditResult editResult = EditResult::Accepted;

    StatusResult statusResult = StatusResult::Current;
    uint64_t attemptId = 0;
    uint64_t statusSequence = 0;
    Phase phase = Phase::NotPrepared;
    int32_t code = 1;
    std::string detail;

    bool meterAvailable = false;
    uint64_t meterStream = 0, meterOrdinal = 0;
    uint32_t meterFrames = 0, meterEnvelopeBits = 0, meterAgeUpperMs = 0;
    uint64_t meterPitch = 0, meterPitchOrdinal = 0;
};

class MultithreadServer final {
public:
    void initialize(bool value) noexcept { requested_=value; accepted_=attempt_=sequence_=0; prepared_=false; }
    void newBinding() noexcept { accepted_=sequence_=0; }
    bool receive(uint64_t id, bool value) noexcept {
        if (!id || sequence_==std::numeric_limits<uint64_t>::max()
            || (id==accepted_ && value!=requested_)) return false;
        ++sequence_;
        if (id>accepted_) { accepted_=id; requested_=value; }
        return true;
    }
    bool requested() const noexcept { return requested_; }
    uint64_t accepted() const noexcept { return accepted_; }
    uint64_t sequence() const noexcept { return sequence_; }
    void committed(bool value, uint64_t attempt) noexcept { prepared_=value; attempt_=attempt; }
    uint64_t preparedAttempt(bool current) const noexcept { return current ? attempt_ : 0; }
    bool prepared(bool current) const noexcept { return current && attempt_ && prepared_; }
private:
    uint64_t accepted_=0, attempt_=0, sequence_=0;
    bool requested_=true, prepared_=false;
};

class MultithreadClient final {
public:
    void initialize(bool value) noexcept { desired_=value; newBinding(); }
    void newBinding() noexcept {
        request_=reloadIssued_=sequence_=observedAttempt_=0;
        accepted_=false; pending_=true; different_=false; refusalRetries_=0;
    }
    bool edit(bool value) noexcept {
        if (desired_==value) return false;
        desired_=value; newBinding(); return true;
    }
    bool assign(uint64_t id) noexcept {
        if (!id || request_) return false;
        request_=id; return true;
    }
    bool observe(uint64_t correlation,uint64_t accepted,uint64_t sequence,bool requested,
        uint64_t preparedAttempt,bool prepared) noexcept {
        if (!request_ || correlation!=request_) return true;
        if (!sequence || accepted!=request_ || requested!=desired_ || (!preparedAttempt && prepared)) return false;
        if (sequence<=sequence_) return true;
        if (accepted_ && reloadIssued_==request_ && preparedAttempt
            && preparedAttempt!=observedAttempt_ && prepared!=desired_) reloadIssued_=0;
        sequence_=sequence; observedAttempt_=preparedAttempt; accepted_=true;
        different_=preparedAttempt && prepared!=desired_;
        pending_=!preparedAttempt || different_; return true;
    }
    bool desired() const noexcept { return desired_; }
    bool pending() const noexcept { return pending_; }
    bool different() const noexcept { return different_; }
    uint64_t request() const noexcept { return request_; }
    bool needsReload() const noexcept { return request_ && accepted_ && different_ && reloadIssued_!=request_; }
    void markReload() noexcept { reloadIssued_=request_; }
    bool retryAfterRefusal(uint64_t attempted) noexcept {
        if (!attempted || request_!=attempted || reloadIssued_!=attempted
            || !different_ || refusalRetries_!=0) return false;
        ++refusalRetries_; reloadIssued_=0; return true;
    }
    void routeChanged() noexcept { reloadIssued_=0; refusalRetries_=0; }
private:
    uint64_t request_=0,reloadIssued_=0,sequence_=0,observedAttempt_=0;
    unsigned refusalRetries_=0;
    bool desired_=true,accepted_=false,pending_=true,different_=false;
};

bool isZero(const Epoch& epoch);
bool encode(const Packet& packet, std::vector<uint8_t>& bytes);
bool decode(const void* data, size_t size, Packet& packet);

}
