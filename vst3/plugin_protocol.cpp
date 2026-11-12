#include "plugin_protocol.h"
#include <algorithm>
#include <cstring>
#include <cmath>
#include <utility>

namespace Rocoflex::ProtocolV4 {
namespace {
void word(std::vector<uint8_t>& bytes, uint32_t value) {
    for (unsigned i = 0; i < 4; ++i) bytes.push_back(static_cast<uint8_t>(value >> (8 * i)));
}
void wide(std::vector<uint8_t>& bytes, uint64_t value) {
    for (unsigned i = 0; i < 8; ++i) bytes.push_back(static_cast<uint8_t>(value >> (8 * i)));
}
bool utf8(const std::string& text) {
    size_t i = 0;
    while (i < text.size()) {
        const auto first = static_cast<uint8_t>(text[i++]);
        if (first == 0) return false;
        if (first < 0x80) continue;
        unsigned count; uint32_t value, minimum;
        if (first >= 0xc2 && first <= 0xdf) { count = 1; value = first & 31; minimum = 0x80; }
        else if (first >= 0xe0 && first <= 0xef) { count = 2; value = first & 15; minimum = 0x800; }
        else if (first >= 0xf0 && first <= 0xf4) { count = 3; value = first & 7; minimum = 0x10000; }
        else return false;
        if (count > text.size() - i) return false;
        for (unsigned n = 0; n < count; ++n) {
            const auto next = static_cast<uint8_t>(text[i++]);
            if ((next & 0xc0) != 0x80) return false;
            value = (value << 6) | (next & 63);
        }
        if (value < minimum || value > 0x10ffff || (value >= 0xd800 && value <= 0xdfff)) return false;
    }
    return true;
}
bool statusCode(int32_t code) { return code >= -3 && code <= 2; }
bool meterValid(const Packet& p) {
    if (!p.meterAvailable) return !p.meterStream && !p.meterOrdinal
        && !p.meterFrames && !p.meterEnvelopeBits && !p.meterAgeUpperMs && !p.meterPitch && !p.meterPitchOrdinal;
    if (p.meterPitch) {
        if (!p.meterPitchOrdinal) return false;
        if (p.meterPitch>>34 || !(p.meterPitch&(uint64_t(1)<<33))) return false;
        const uint32_t bits=uint32_t(p.meterPitch); float note; std::memcpy(&note,&bits,4);
        if (!std::isfinite(note) || (!(p.meterPitch&(uint64_t(1)<<32)) && note<=0.f)) return false;
    }
    float value; std::memcpy(&value,&p.meterEnvelopeBits,sizeof(value));
    return p.meterStream && p.meterOrdinal && p.meterFrames && p.meterFrames<=65536 && p.meterAgeUpperMs<=100
        && std::isfinite(value) && value>=0.f;
}
bool latencyValid(const LatencyDeclaration& d) {
    return d.epoch ? d.samples && d.sourceConfig && d.sourceSetup
        : d.samples==6000 && !d.sourceConfig && !d.sourceSetup;
}
bool appendLatency(std::vector<uint8_t>& bytes,const LatencyDeclaration& d) {
    if (!latencyValid(d)) return false;
    wide(bytes,d.epoch); word(bytes,d.samples); wide(bytes,d.sourceConfig); wide(bytes,d.sourceSetup); return true;
}
bool appendRoute(std::vector<uint8_t>& bytes,const Packet& p) {
    if (!p.latencyRouteSerial) return false;
    word(bytes,p.latencyHandlerReady?1:0); wide(bytes,p.latencyRouteSerial); return true;
}
bool headerValid(const Packet& p) {
    switch (p.kind) {
    case Kind::ConnectionReady:
        return !isZero(p.processorInstance) && !isZero(p.serverEpoch) && isZero(p.clientEpoch) && p.correlation == 0;
    case Kind::RequestSnapshot:
        return !isZero(p.clientEpoch) && p.correlation != 0
            && (isZero(p.serverEpoch) ? isZero(p.processorInstance) : !isZero(p.processorInstance));
    case Kind::Recall: case Kind::LatencyChanged:
        return !isZero(p.processorInstance) && !isZero(p.serverEpoch) && !isZero(p.clientEpoch) && p.correlation == 0;
    case Kind::Snapshot: case Kind::Edit: case Kind::EditAck:
    case Kind::RequestStatus: case Kind::Status:
    case Kind::MultithreadRequest: case Kind::MultithreadStatus:
        return !isZero(p.processorInstance) && !isZero(p.serverEpoch) && !isZero(p.clientEpoch) && p.correlation != 0;
    }
    return false;
}
bool appendSnapshot(std::vector<uint8_t>& bytes, const Packet& p) {
    if (!p.stateRevision || !p.configToken) return false;
    std::vector<uint8_t> state;
    if (!encodeState(p.state, state)) return false;
    wide(bytes, p.stateRevision); wide(bytes, p.configToken);
    word(bytes, static_cast<uint32_t>(state.size()));
    bytes.insert(bytes.end(), state.begin(), state.end());
    return appendLatency(bytes,p.latency);
}
struct Reader {
    const uint8_t* data; size_t size, at = 0;
    bool word(uint32_t& value) {
        if (size - at < 4) return false;
        value = 0;
        for (unsigned i = 0; i < 4; ++i) value |= uint32_t(data[at++]) << (8 * i);
        return true;
    }
    bool wide(uint64_t& value) {
        if (size - at < 8) return false;
        value = 0;
        for (unsigned i = 0; i < 8; ++i) value |= uint64_t(data[at++]) << (8 * i);
        return true;
    }
    bool epoch(Epoch& value) {
        if (size - at < value.size()) return false;
        std::copy_n(data + at, value.size(), value.begin()); at += value.size(); return true;
    }
    bool state(PluginState& value) {
        uint32_t length;
        if (!word(length) || length > size - at || length > kMaximumStateBytes
            || !decodeState(data + at, length, value)) return false;
        at += length; return true;
    }
    bool latency(LatencyDeclaration& d) {
        return wide(d.epoch) && word(d.samples) && wide(d.sourceConfig) && wide(d.sourceSetup) && latencyValid(d);
    }
    bool route(Packet& p) {
        uint32_t ready;
        if (!word(ready) || ready>1 || !wide(p.latencyRouteSerial) || !p.latencyRouteSerial) return false;
        p.latencyHandlerReady=ready!=0; return true;
    }
    bool snapshot(Packet& p) {
        return wide(p.stateRevision) && p.stateRevision && wide(p.configToken) && p.configToken && state(p.state) && latency(p.latency);
    }
    bool detail(std::string& value) {
        uint32_t length;
        if (!word(length) || length > kMaximumDetailBytes || length > size - at) return false;
        value.assign(reinterpret_cast<const char*>(data + at), length); at += length;
        return utf8(value);
    }
};
}

bool isZero(const Epoch& epoch) {
    return std::all_of(epoch.begin(), epoch.end(), [](uint8_t byte) { return byte == 0; });
}

bool encode(const Packet& p, std::vector<uint8_t>& output) {
    if (!headerValid(p)) return false;
    try {
        std::vector<uint8_t> bytes;
        word(bytes, kMagic); word(bytes, kVersion); word(bytes, static_cast<uint32_t>(p.kind)); word(bytes, 0);
        bytes.insert(bytes.end(), p.processorInstance.begin(), p.processorInstance.end());
        bytes.insert(bytes.end(), p.serverEpoch.begin(), p.serverEpoch.end());
        bytes.insert(bytes.end(), p.clientEpoch.begin(), p.clientEpoch.end()); wide(bytes, p.correlation);
        switch (p.kind) {
        case Kind::ConnectionReady: break;
        case Kind::RequestSnapshot:
            if (!appendRoute(bytes,p)) return false;
            break;
        case Kind::LatencyChanged:
            if (!p.latency.epoch || !appendLatency(bytes,p.latency)) return false;
            break;
        case Kind::MultithreadRequest:
            word(bytes,p.multithreadRequested?1:0);
            break;
        case Kind::MultithreadStatus:
            if (!p.multithreadAccepted || !p.multithreadStatusSequence
                || (!p.multithreadPreparedAttempt && p.multithreadPrepared)) return false;
            wide(bytes,p.multithreadAccepted); wide(bytes,p.multithreadStatusSequence); word(bytes,p.multithreadRequested?1:0);
            wide(bytes,p.multithreadPreparedAttempt); word(bytes,p.multithreadPrepared?1:0);
            break;
        case Kind::Snapshot: case Kind::Recall:
            if (!appendSnapshot(bytes, p)) return false;
            break;
        case Kind::Edit: {
            if (!p.baseRevision) return false;
            std::vector<uint8_t> state;
            if (!encodeState(p.state, state)) return false;
            wide(bytes, p.baseRevision); word(bytes, static_cast<uint32_t>(state.size()));
            bytes.insert(bytes.end(), state.begin(), state.end());
            break;
        }
        case Kind::EditAck:
            if (static_cast<uint32_t>(p.editResult) > 3
                || (p.editResult == EditResult::Accepted
                    ? !p.appliedRevision || p.appliedRevision > p.stateRevision : p.appliedRevision != 0)) return false;
            word(bytes, static_cast<uint32_t>(p.editResult)); wide(bytes, p.appliedRevision);
            if (!appendSnapshot(bytes, p)) return false;
            break;
        case Kind::RequestStatus:
            if (!p.stateRevision || !p.configToken) return false;
            wide(bytes, p.stateRevision); wide(bytes, p.configToken);
            if (!appendRoute(bytes,p)) return false;
            break;
        case Kind::Status:
            if (static_cast<uint32_t>(p.statusResult) > 2) return false;
            word(bytes, static_cast<uint32_t>(p.statusResult));
            if (!appendLatency(bytes,p.latency)) return false;
            if (p.statusResult == StatusResult::Busy) break;
            if (!p.stateRevision || !p.configToken) return false;
            wide(bytes, p.stateRevision); wide(bytes, p.configToken);
            if (p.statusResult == StatusResult::StaleState) break;
            if (!p.statusSequence || static_cast<uint32_t>(p.phase) > 4 || !statusCode(p.code)
                || p.detail.size() > kMaximumDetailBytes || !utf8(p.detail)) return false;
            wide(bytes, p.attemptId); wide(bytes, p.statusSequence);
            word(bytes, static_cast<uint32_t>(p.phase));
            uint32_t code; std::memcpy(&code, &p.code, sizeof(code)); word(bytes, code);
            word(bytes, static_cast<uint32_t>(p.detail.size()));
            bytes.insert(bytes.end(), p.detail.begin(), p.detail.end());
            if (!meterValid(p)) return false;
            word(bytes,p.meterAvailable?1:0);
            if (p.meterAvailable) {
                wide(bytes,p.meterStream); wide(bytes,p.meterOrdinal);
                word(bytes,p.meterFrames); word(bytes,p.meterEnvelopeBits); word(bytes,p.meterAgeUpperMs); wide(bytes,p.meterPitch); wide(bytes,p.meterPitchOrdinal);
            }
            break;
        }
        if (bytes.size() > kMaximumPacketBytes) return false;
        output.swap(bytes); return true;
    } catch (...) { return false; }
}

bool decode(const void* data, size_t size, Packet& output) {
    if (!data || size < 72 || size > kMaximumPacketBytes) return false;
    try {
        Reader r{static_cast<const uint8_t*>(data), size};
        uint32_t magic, version, kind, reserved;
        Packet p;
        if (!r.word(magic) || magic != kMagic || !r.word(version) || version != kVersion
            || !r.word(kind) || kind < 1 || kind > 11 || !r.word(reserved) || reserved
            || !r.epoch(p.processorInstance) || !r.epoch(p.serverEpoch)
            || !r.epoch(p.clientEpoch) || !r.wide(p.correlation)) return false;
        p.kind = static_cast<Kind>(kind);
        if (!headerValid(p)) return false;
        switch (p.kind) {
        case Kind::ConnectionReady: break;
        case Kind::RequestSnapshot:
            if (!r.route(p)) return false;
            break;
        case Kind::LatencyChanged:
            if (!r.latency(p.latency) || !p.latency.epoch) return false;
            break;
        case Kind::MultithreadRequest: {
            uint32_t requested;
            if (!r.word(requested) || requested>1) return false;
            p.multithreadRequested=requested!=0; break;
        }
        case Kind::MultithreadStatus: {
            uint32_t requested,prepared;
            if (!r.wide(p.multithreadAccepted) || !p.multithreadAccepted
                || !r.wide(p.multithreadStatusSequence) || !p.multithreadStatusSequence
                || !r.word(requested) || requested>1 || !r.wide(p.multithreadPreparedAttempt)
                || !r.word(prepared) || prepared>1 || (!p.multithreadPreparedAttempt && prepared)) return false;
            p.multithreadRequested=requested!=0; p.multithreadPrepared=prepared!=0; break;
        }
        case Kind::Snapshot: case Kind::Recall:
            if (!r.snapshot(p)) return false;
            break;
        case Kind::Edit:
            if (!r.wide(p.baseRevision) || !p.baseRevision || !r.state(p.state)) return false;
            break;
        case Kind::EditAck: {
            uint32_t result;
            if (!r.word(result) || result > 3 || !r.wide(p.appliedRevision) || !r.snapshot(p)) return false;
            p.editResult = static_cast<EditResult>(result);
            if (p.editResult == EditResult::Accepted
                ? !p.appliedRevision || p.appliedRevision > p.stateRevision : p.appliedRevision != 0) return false;
            break;
        }
        case Kind::RequestStatus:
            if (!r.wide(p.stateRevision) || !p.stateRevision || !r.wide(p.configToken) || !p.configToken || !r.route(p)) return false;
            break;
        case Kind::Status: {
            uint32_t result;
            if (!r.word(result) || result > 2) return false;
            p.statusResult = static_cast<StatusResult>(result);
            if (!r.latency(p.latency)) return false;
            if (p.statusResult == StatusResult::Busy) break;
            if (!r.wide(p.stateRevision) || !p.stateRevision || !r.wide(p.configToken) || !p.configToken) return false;
            if (p.statusResult == StatusResult::StaleState) break;
            uint32_t phase, code;
            if (!r.wide(p.attemptId) || !r.wide(p.statusSequence) || !p.statusSequence
                || !r.word(phase) || phase > 4 || !r.word(code) || !r.detail(p.detail)) return false;
            p.phase = static_cast<Phase>(phase); std::memcpy(&p.code, &code, sizeof(code));
            if (!statusCode(p.code)) return false;
            uint32_t available;
            if (!r.word(available) || available>1) return false;
            p.meterAvailable=available!=0;
            if (p.meterAvailable && (!r.wide(p.meterStream) || !r.wide(p.meterOrdinal)
                || !r.word(p.meterFrames) || !r.word(p.meterEnvelopeBits) || !r.word(p.meterAgeUpperMs) || !r.wide(p.meterPitch) || !r.wide(p.meterPitchOrdinal))) return false;
            if (!meterValid(p)) return false;
            break;
        }
        }
        if (r.at != r.size) return false;
        output = std::move(p); return true;
    } catch (...) { return false; }
}
}
