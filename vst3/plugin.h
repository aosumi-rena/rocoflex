#pragma once

#include <cstddef>
#include <cstdint>
#include <string>
#include <vector>

#include "pluginterfaces/base/funknown.h"
#include "pluginterfaces/base/ibstream.h"
#include "pluginterfaces/vst/vsttypes.h"

// Bridges are component-local and are never serialized or sent to a host.
extern "C" {
struct RocoPlugin;
struct RocoCondition {
    uint32_t mode;
    float pregain_db;
    float pitch_semitones;
    float tone[3];
    float canvas[128];
};

// Caller zeroes preparation output; Rust clears failures. Output policy: 0 invalid, 1 fixed, 2 prepared delay.
enum RocoLatencyInputPolicy : uint32_t { RocoFixed6000 = 0, RocoPreparedCount = 1 };
struct RocoPrepareInfo { uint32_t delay_samples; uint32_t policy; };
RocoPlugin* roco_plugin_create(void);
void roco_plugin_destroy(RocoPlugin* plugin);
int32_t roco_plugin_invalidate_editor_session(RocoPlugin* plugin);
uint32_t roco_plugin_read_multithread_preference(void);
int32_t roco_plugin_try_read_multithread_preference(uint32_t* enabled);
int32_t roco_plugin_take_multithread_edit(const RocoPlugin* plugin, uint32_t* enabled);
int32_t roco_plugin_prepare(RocoPlugin* plugin, const char* analysis_utf8,
    const char* userdata_utf8, uint32_t sample_rate, uint32_t max_block_frames,
    uint32_t input_channel, uint32_t quality_profile, uint32_t latency_input_policy,
    uint32_t multithread, RocoPrepareInfo* prepared);
struct RocoQualityChoice { uint32_t profile; uint32_t defaulted_label; };
int32_t roco_quality_resolve(const uint8_t* document, size_t length, RocoQualityChoice* choice);
int32_t roco_plugin_set_condition(RocoPlugin* plugin, const RocoCondition* condition);
int32_t roco_plugin_get_condition(RocoPlugin* plugin, RocoCondition* condition, uint64_t* revision);
int32_t roco_plugin_process(RocoPlugin* plugin, const float* in_left, const float* in_right,
    float* out_left, float* out_right, uint32_t frames, uint32_t input_channels,
    uint32_t output_channels);
uint32_t roco_plugin_latency_samples(const RocoPlugin* plugin);
int32_t roco_plugin_input_gain(const RocoPlugin* plugin, float* gain);
size_t roco_plugin_copy_error(const RocoPlugin* plugin, char* buffer, size_t capacity);

// Editor calls run on the host GUI thread; the bridge is borrowed until close returns.
struct RocoEditor;
RocoEditor* roco_editor_open(RocoPlugin* plugin, void* parent, uint32_t width, uint32_t height);
int32_t roco_editor_idle(RocoEditor* editor);
int32_t roco_editor_set_input_meter(RocoEditor* editor, uint32_t available, float envelope);
int32_t roco_editor_resize(RocoEditor* editor, uint32_t width, uint32_t height);
// Host scale replaces native DPI; zero restores it. GUI thread only.
int32_t roco_editor_set_content_scale(RocoEditor* editor, float factor);
// GUI-thread staging: 0 unhandled, 1 handled with result, -2 invalid; C++ owns BeginPaint/EndPaint.
int32_t roco_editor_window_message(RocoEditor* editor, uint32_t message,
    uintptr_t wparam, intptr_t lparam, intptr_t* result);
// Keys use VST SDK, not Win32, codes and modifiers: 1 handled, 0 unhandled, -2 invalid.
int32_t roco_editor_host_key(RocoEditor* editor, uint16_t unicode,
    int16_t vst_keycode, int16_t vst_modifiers, uint32_t is_down);
// Non-dropping owner-thread cancellation is safe inside an editor callback.
int32_t roco_editor_cancel(RocoEditor* editor);
void roco_editor_close(RocoEditor* editor);
uint64_t roco_editor_state_revision(const RocoEditor* editor);
size_t roco_editor_get_model_path(const RocoEditor* editor, uint32_t which,
    char* buffer, size_t capacity);
size_t roco_editor_get_state(const RocoEditor* editor, char* buffer, size_t capacity);
int32_t roco_editor_set_multithread(RocoEditor* editor, uint32_t enabled);
int32_t roco_editor_set_state(RocoEditor* editor, const char* state, size_t length, const RocoCondition* expected);
int32_t roco_editor_set_model_paths(RocoEditor* editor, const char* analysis_utf8,
    const char* userdata_utf8);
size_t roco_editor_last_error(char* buffer, size_t capacity);
int32_t roco_editor_report_error(RocoEditor* editor, uint64_t event_id,
    const char* utf8, size_t length);
}

static_assert(sizeof(RocoPrepareInfo) == 8 && alignof(RocoPrepareInfo) == 4,
    "Rust/C++ prepared latency ABI mismatch");
static_assert(offsetof(RocoPrepareInfo, delay_samples) == 0 && offsetof(RocoPrepareInfo, policy) == 4);
static_assert(sizeof(RocoCondition) == 536, "Rust/C++ condition ABI size mismatch");
static_assert(alignof(RocoCondition) == 4, "Rust/C++ condition ABI alignment mismatch");

namespace Rocoflex {

Steinberg::FUnknown* createProcessor();

inline const Steinberg::FUID kProcessorUID(0x7E31B53D, 0x86B946C2, 0xBE659F74, 0x11B24A80);
inline const Steinberg::FUID kControllerUID(0x19D8A43C, 0x57E14F90, 0x9B2062C5, 0x4D8FE317);
inline constexpr const char* kVersion = "0.1.0";
inline constexpr uint32_t kStateVersion = 1;
inline constexpr size_t kMaximumStateBytes = 16 * 1024 * 1024;

enum ConditionMode : uint32_t {
    kConditionEmpty = 0,
    kConditionCanvas = 1,
    kConditionTone = 2,
    kConditionUnavailable = 3,
};
inline constexpr bool hasConditionTarget(uint32_t mode) noexcept {
    return mode == kConditionCanvas || mode == kConditionTone;
}
inline constexpr const char* kUnavailableConditionDetail =
    "The selected voice condition is unavailable. Resolve the canvas selection error before processing.";

enum Parameter : Steinberg::Vst::ParamID {
    kBypass = 0,
    kPregain = 1,
    kPitch = 2,
    kToneVertical = 3,
    kToneHorizontal = 4,
    kToneHue = 5,
    kInputChannel = 6,
    kTargetMode = 7,
};

enum BridgeResult : int32_t {
    kBridgeOk = 0,
    kBridgeNotReady = 1,
    kBridgeBusy = 2,
    kBridgeInvalid = -1,
    kBridgeFailed = -2,
    kBridgePanic = -3,
};

struct PluginState {
    RocoCondition condition {0, 0.0f, 0.0f, {0.5f, 0.5f, 0.0f}, {}};
    bool bypass = false;
    uint32_t inputChannel = 0;
    std::string analysisPath;
    std::string userdataPath;
    std::string documentJson;
};

enum class PrepareProfile : uint32_t { Highest=0, Higher=1, Lower=2, Lowest=3, Invalid=4 };
struct QualityChoice {
    PrepareProfile profile=PrepareProfile::Highest;
    bool defaultedLabel=false;
};
inline QualityChoice qualityForState(const PluginState& next, const PluginState& previous,
    QualityChoice cached) {
    if (next.documentJson==previous.documentJson) return cached;
    RocoQualityChoice raw{4,0};
    const auto result=roco_quality_resolve(reinterpret_cast<const uint8_t*>(next.documentJson.data()),
        next.documentJson.size(),&raw);
    if (result!=0 || raw.profile>3 || raw.defaulted_label>1) return {PrepareProfile::Invalid,false};
    return {static_cast<PrepareProfile>(raw.profile),raw.defaulted_label!=0};
}
inline const char* qualityIssue(QualityChoice choice) {
    if (choice.profile==PrepareProfile::Invalid)
        return "Quality preferences are malformed. Use a boolean realtime preference and a text quality label.";
    return nullptr;
}
inline bool graphReprepareRequired(const PluginState& before,const PluginState& after,
    QualityChoice previous,QualityChoice next) {
    return before.analysisPath!=after.analysisPath || before.userdataPath!=after.userdataPath
        || previous.profile!=next.profile
        || (!hasConditionTarget(before.condition.mode) && hasConditionTarget(after.condition.mode));
}
static_assert(sizeof(RocoQualityChoice)==8 && alignof(RocoQualityChoice)==4);

// Live packets carry this saved-state body; failed decode leaves state untouched.
bool encodeState(const PluginState& state, std::vector<uint8_t>& bytes);
bool decodeState(const void* bytes, size_t size, PluginState& state);
bool writeState(Steinberg::IBStream* stream, const PluginState& state);
bool readState(Steinberg::IBStream* stream, PluginState& state);
bool validState(const PluginState& state);
bool isHostParameter(Steinberg::Vst::ParamID id);
double parameterValue(const PluginState& state, Steinberg::Vst::ParamID id);
bool setParameterValue(PluginState& state, Steinberg::Vst::ParamID id, double normalized);

}
