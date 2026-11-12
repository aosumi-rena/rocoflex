#pragma once

#include <algorithm>
#include <array>
#include <atomic>
#include <chrono>
#include <cmath>
#include <cstddef>
#include <cstdint>
#include <cstring>
#include <limits>
#include <xmmintrin.h>

namespace Rocoflex {

class PitchObservation final {
public:
    using Clock=std::chrono::steady_clock;
    void reset() noexcept {stream_=ordinal_=word_=0;observed_={};}
    void observe(uint64_t stream,uint64_t ordinal,uint64_t word,Clock::time_point observed) noexcept {
        if (stream_!=stream) {reset();stream_=stream;}
        if (ordinal>ordinal_) {ordinal_=ordinal;word_=word;observed_=observed;}
        else if (!ordinal || word!=word_) word_=0;
    }
    uint64_t value(Clock::time_point now) const noexcept {
        return ordinal_ && now>=observed_ && now-observed_<=std::chrono::milliseconds(100)?word_:0;
    }
private:
    uint64_t stream_=0,ordinal_=0,word_=0;
    Clock::time_point observed_{};
};

class InputBusState final {
public:
    explicit InputBusState(bool enabled=false) noexcept : state_(uint64_t(2)|uint64_t(enabled)) {}
    bool load() const noexcept {return (state_.load()&1)!=0;}
    uint64_t snapshot() const noexcept {return state_.load();}
    bool exchange(bool enabled) noexcept {
        auto before=state_.load();
        for (;;) {
            const bool old=(before&1)!=0;
            if (old==enabled) return old;
            auto generation=before>>1;
            generation=generation && generation<(std::numeric_limits<uint64_t>::max()>>1)?generation+1:0;
            const auto after=(generation<<1)|uint64_t(enabled);
            if (state_.compare_exchange_weak(before,after)) return old;
        }
    }
    void store(bool enabled) noexcept {exchange(enabled);}
private:
    std::atomic<uint64_t> state_;
};

// A serialized audio callback writes; control reads the atomic mailbox.
class InputMeter final {
public:
    struct Sample {
        uint64_t revision=0, configuration=0, attempt=0, connection=0, stream=0, request=0, ordinal=0;
        uint32_t frames=0, envelopeBits=0;
        uint64_t pitch=0, pitchOrdinal=0, sourceBus=0;
    };

    bool inputBlock(const float* input, uint32_t frames, float gain) noexcept {
        if (!input || !frames || frames>65536 || !std::isfinite(gain) || gain<0.f) return false;
        struct Fp {
            unsigned saved;
            Fp() noexcept : saved(_mm_getcsr()) { _mm_setcsr(saved|0x8040); }
            ~Fp() { _mm_setcsr(saved); }
        } fp;
        float peak=0.f;
        for (uint32_t i=0;i<frames;++i) {
            if (!std::isfinite(input[i])) return false;
            peak=std::max(peak,std::abs(input[i]));
        }
        const float incoming=peak*gain;
        const float decayed=envelope_*0.99f;
        const float next=std::max(incoming,decayed);
        if (!std::isfinite(next)) return false;
        envelope_=next;
        return true;
    }

    bool publish(Sample sample) noexcept {
        if (!sample.revision || !sample.configuration || !sample.attempt
            || !sample.connection || !sample.stream || !sample.request || !sample.frames || sample.frames>65536) return false;
        if (exhausted_.load() || ordinal_==std::numeric_limits<uint64_t>::max()) {
            exhausted_.store(true); return false;
        }
        const auto before=guard_.load();
        if (before>std::numeric_limits<uint64_t>::max()-2) {
            exhausted_.store(true); return false;
        }
        sample.ordinal=++ordinal_;
        std::memcpy(&sample.envelopeBits,&envelope_,sizeof(envelope_));
        guard_.store(before+1);
        words_[Revision].store(sample.revision);
        words_[Configuration].store(sample.configuration);
        words_[Attempt].store(sample.attempt);
        words_[Connection].store(sample.connection);
        words_[Stream].store(sample.stream);
        words_[Request].store(sample.request);
        words_[Ordinal].store(sample.ordinal);
        words_[Audio].store((uint64_t(sample.frames)<<32)|sample.envelopeBits);
        words_[Pitch].store(sample.pitch);
        words_[PitchOrdinal].store(sample.pitchOrdinal);
        words_[SourceBus].store(sample.sourceBus);
        guard_.store(before+2);
        return true;
    }

    bool snapshot(Sample& output) const noexcept {
        if (exhausted_.load()) return false;
        for (unsigned attempt=0;attempt<3;++attempt) {
            const auto before=guard_.load();
            if (!before || (before&1)) continue;
            Sample value;
            value.revision=words_[Revision].load();
            value.configuration=words_[Configuration].load();
            value.attempt=words_[Attempt].load();
            value.connection=words_[Connection].load();
            value.stream=words_[Stream].load();
            value.request=words_[Request].load();
            value.ordinal=words_[Ordinal].load();
            const auto audio=words_[Audio].load();
            value.frames=uint32_t(audio>>32); value.envelopeBits=uint32_t(audio);
            value.pitch=words_[Pitch].load();
            value.pitchOrdinal=words_[PitchOrdinal].load();
            value.sourceBus=words_[SourceBus].load();
            if (guard_.load()!=before || exhausted_.load()) continue;
            output=value; return true;
        }
        return false;
    }

private:
    static_assert(std::atomic<uint64_t>::is_always_lock_free,
        "Input meter publication requires the admitted x86-64 atomics");
    static_assert(std::atomic<bool>::is_always_lock_free,
        "Input meter exhaustion flag must also be lock free");
    enum Word : std::size_t { Revision, Configuration, Attempt, Connection, Stream, Request, Ordinal, Audio, Pitch, PitchOrdinal, SourceBus, Count };
    std::array<std::atomic<uint64_t>,Count> words_{};
    std::atomic<uint64_t> guard_{0};
    std::atomic<bool> exhausted_{false};
    float envelope_=0.f;
    uint64_t ordinal_=0;
};
}
