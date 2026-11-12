#pragma once
#include "plugin_protocol.h"
#include <atomic>
#include <limits>

namespace Rocoflex {

// Control mutates attempts after admission drains; audio observes without allocation or locks.
class ProcessorStatus final {
public:
    struct Snapshot {
        uint64_t attemptId = 0;
        uint64_t attemptConfig = 0;
        uint64_t sequence = 0;
        ProtocolV4::Phase phase = ProtocolV4::Phase::NotPrepared;
        int32_t code = kBridgeNotReady;
        bool bridgeDetailReady = false;
    };

    bool beginAttempt(uint64_t configuration) noexcept {
        if (nextAttempt_ == std::numeric_limits<uint64_t>::max()) return false;
        contextSequence_.fetch_add(1, std::memory_order_acq_rel);
        attempt_.store(++nextAttempt_, std::memory_order_relaxed);
        attemptConfig_.store(configuration, std::memory_order_relaxed);
        bridgeDetailReady_.store(false, std::memory_order_relaxed);
        publish(ProtocolV4::Phase::Preparing, kBridgeBusy);
        contextSequence_.fetch_add(1, std::memory_order_release);
        return true;
    }

    void finishAttempt(int32_t code, bool cameFromBridge) noexcept {
        contextSequence_.fetch_add(1, std::memory_order_acq_rel);
        bridgeDetailReady_.store(code < 0 && cameFromBridge, std::memory_order_relaxed);
        publish(code == kBridgeOk ? ProtocolV4::Phase::Ready :
            code < 0 ? ProtocolV4::Phase::Faulted : ProtocolV4::Phase::NotPrepared, code);
        contextSequence_.fetch_add(1, std::memory_order_release);
    }

    // Call after the fatal-result latch stops bridge process/set-condition calls.
    void observeProcess(int32_t code, bool cameFromBridge) noexcept {
        if (phase(word_.load(std::memory_order_acquire)) == ProtocolV4::Phase::Faulted) return;
        if (code < 0) {
            bridgeDetailReady_.store(cameFromBridge, std::memory_order_relaxed);
            publish(ProtocolV4::Phase::Faulted, code);
        } else {
            publish(code == kBridgeOk ? ProtocolV4::Phase::Processing :
                ProtocolV4::Phase::NotPrepared, code);
        }
    }

    void stopped(bool prepared) noexcept {
        const auto before = word_.load(std::memory_order_acquire);
        if (phase(before) != ProtocolV4::Phase::Faulted)
            publish(prepared ? ProtocolV4::Phase::Ready : ProtocolV4::Phase::NotPrepared,
                prepared ? kBridgeOk : kBridgeNotReady);
    }

    bool snapshot(Snapshot& out) const noexcept {
        for (unsigned tryIndex = 0; tryIndex != 3; ++tryIndex) {
            const auto before = contextSequence_.load(std::memory_order_acquire);
            if (before & 1) continue;
            Snapshot next;
            next.attemptId = attempt_.load(std::memory_order_relaxed);
            next.attemptConfig = attemptConfig_.load(std::memory_order_relaxed);
            const auto value = word_.load(std::memory_order_acquire);
            next.sequence = value >> 16;
            next.phase = phase(value);
            const auto byte = static_cast<int32_t>(value & 0xff);
            next.code = byte < 128 ? byte : byte - 256;
            next.bridgeDetailReady = bridgeDetailReady_.load(std::memory_order_acquire);
            if (before != contextSequence_.load(std::memory_order_acquire)) continue;
            next.bridgeDetailReady &= next.phase == ProtocolV4::Phase::Faulted;
            out = next;
            return true;
        }
        return false;
    }

private:
    static_assert(std::atomic<uint64_t>::is_always_lock_free,
        "The selected x86-64 callback requires lock-free status publication");
    static ProtocolV4::Phase phase(uint64_t word) noexcept {
        return static_cast<ProtocolV4::Phase>((word >> 8) & 0xff);
    }
    void publish(ProtocolV4::Phase nextPhase, int32_t code) noexcept {
        const auto payload = (uint64_t(nextPhase) << 8) | static_cast<uint8_t>(code);
        auto before = word_.load(std::memory_order_relaxed);
        for (;;) {
            if ((before & 0xffff) == payload) return;
            const auto sequence = before >> 16;
            const auto advanced = sequence < ((uint64_t(1) << 48) - 1) ? sequence + 1 : sequence;
            if (word_.compare_exchange_weak(before, (advanced << 16) | payload,
                    std::memory_order_release, std::memory_order_relaxed)) return;
        }
    }
    uint64_t nextAttempt_ = 0;
    std::atomic<uint64_t> contextSequence_{0};
    std::atomic<uint64_t> attempt_{0}, attemptConfig_{0};
    std::atomic<uint64_t> word_{(uint64_t(1) << 16) | uint64_t(kBridgeNotReady)};
    std::atomic<bool> bridgeDetailReady_{false};
};


Steinberg::FUnknown* createProcessor();
}
