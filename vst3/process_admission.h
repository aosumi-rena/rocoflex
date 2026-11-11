#pragma once
#ifndef NOMINMAX
#define NOMINMAX
#endif
#include <windows.h>
#include <atomic>
#include <chrono>
#include <cstdint>

namespace Rocoflex {
namespace ProcessAdmission {
// Never wait or prepare on a process callback stack, including cross-owner reentry.
class Scope {
public:
    explicit Scope(const void* owner) noexcept : owner_(owner), previous_(head_) { head_=this; }
    ~Scope() { head_=previous_; }
    Scope(const Scope&)=delete;
    Scope& operator=(const Scope&)=delete;
    static bool any() noexcept { return head_!=nullptr; }
    static bool contains(const void* owner) noexcept {
        for (auto* at=head_;at;at=at->previous_) if (at->owner_==owner) return true;
        return false;
    }
private:
    const void* owner_;
    Scope* previous_;
    inline static thread_local Scope* head_=nullptr;
};

class ControlLease {
public:
    explicit ControlLease(std::atomic_flag& flag) noexcept : flag_(flag), held_(!flag.test_and_set()) {}
    ~ControlLease() { if (held_) flag_.clear(); }
    explicit operator bool() const noexcept { return held_; }
    ControlLease(const ControlLease&)=delete;
    ControlLease& operator=(const ControlLease&)=delete;
private:
    std::atomic_flag& flag_;
    bool held_;
};

class Gate {
public:
    static constexpr std::uint32_t closedBit=0x80000000u;
    static constexpr std::uint32_t countMask=~closedBit;
    Gate() noexcept = default;
    Gate(const Gate&)=delete;
    Gate& operator=(const Gate&)=delete;
    bool enter() noexcept {
        auto observed=word();
        for (;;) {
            if ((observed&closedBit) || (observed&countMask)==countMask) return false;
            const auto actual=static_cast<std::uint32_t>(InterlockedCompareExchange(
                &word_,static_cast<LONG>(observed+1),static_cast<LONG>(observed)));
            if (actual==observed) return true;
            observed=actual;
        }
    }
    void leave() noexcept {
        const auto before=static_cast<std::uint32_t>(InterlockedExchangeAdd(&word_,-1));
        if (before==(closedBit|1u)) WakeByAddressAll(const_cast<LONG*>(&word_));
    }
    void close() noexcept { InterlockedOr(&word_,closedWord); }
    std::uint32_t admitted() const noexcept { return word()&countMask; }
    bool closed() const noexcept { return (word()&closedBit)!=0; }
    bool quiescent() const noexcept { return word()==closedBit; }
    // The lifecycle reservation owner publishes activation after quiescence under control_.
    void openQuiescent() noexcept { InterlockedExchange(&word_,0); }
    bool waitUntil(std::chrono::steady_clock::time_point deadline) noexcept {
        for (;;) {
            auto observed=word();
            if (!(observed&closedBit)) return false;
            if (!(observed&countMask)) return true;
            const auto now=std::chrono::steady_clock::now();
            if (now>=deadline) return quiescent();
            const auto whole=std::chrono::duration_cast<std::chrono::milliseconds>(deadline-now).count();
            if (whole<=0) return quiescent();
            const auto remaining=static_cast<DWORD>(whole<1000 ? whole : 1000);
            if (!WaitOnAddress(&word_,&observed,sizeof(observed),remaining)
                && GetLastError()!=ERROR_TIMEOUT) return quiescent();
        }
    }
private:
    std::uint32_t word() const noexcept {
        return static_cast<std::uint32_t>(InterlockedCompareExchange(
            const_cast<volatile LONG*>(&word_),0,0));
    }
    static_assert(sizeof(LONG)==4);
    static constexpr LONG closedWord=(-2147483647L-1L);
    alignas(LONG) volatile LONG word_=closedWord;
};

class Call {
public:
    explicit Call(Gate& gate) noexcept : gate_(gate), admitted_(gate.enter()) {}
    ~Call() { if (admitted_) gate_.leave(); }
    explicit operator bool() const noexcept { return admitted_; }
    Call(const Call&)=delete;
    Call& operator=(const Call&)=delete;
private:
    Gate& gate_;
    bool admitted_;
};
}
}
