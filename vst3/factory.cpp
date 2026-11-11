// Instantiate SDK interface IDs in exactly one translation unit.
#define INIT_CLASS_IID
#include "plugin.h"
#include "controller.h"
#include "pluginterfaces/base/ipluginbase.h"
#include "pluginterfaces/gui/iplugview.h"
#include "pluginterfaces/vst/ivstaudioprocessor.h"
#include "pluginterfaces/vst/ivstcomponent.h"
#include "pluginterfaces/vst/ivsthostapplication.h"
#include "pluginterfaces/vst/ivstparameterchanges.h"
#include <atomic>
#include <cstring>
#include <new>

namespace Rocoflex {
namespace {
using namespace Steinberg;
using namespace Steinberg::Vst;
bool matches(const TUID iid, const FUID& expected) {
    return iid && std::memcmp(iid, expected.toTUID(), 16) == 0;
}
class Factory final : public IPluginFactory3 {
public:
    ~Factory() { if (host_) host_->release(); }
    tresult PLUGIN_API queryInterface(const TUID iid, void** object) override {
        if (!object) return kInvalidArgument; *object = nullptr;
        if (!matches(iid,FUnknown::iid) && !matches(iid,IPluginFactory::iid)
            && !matches(iid,IPluginFactory2::iid) && !matches(iid,IPluginFactory3::iid)) return kNoInterface;
        *object = static_cast<IPluginFactory3*>(this); addRef(); return kResultOk;
    }
    uint32 PLUGIN_API addRef() override { return ++references_; }
    uint32 PLUGIN_API release() override { const auto left = --references_; if (!left) delete this; return left; }
    tresult PLUGIN_API getFactoryInfo(PFactoryInfo* info) override {
        if (!info) return kInvalidArgument;
        // Keep the module resident while editor, OLE/UIA or GPU callbacks may use it.
        *info = PFactoryInfo("Rocoflex", "", "",
            PFactoryInfo::kUnicode | PFactoryInfo::kComponentNonDiscardable); return kResultOk;
    }
    int32 PLUGIN_API countClasses() override { return 2; }
    tresult PLUGIN_API getClassInfo(int32 index, PClassInfo* info) override {
        if (!info || index < 0 || index >= 2) return kInvalidArgument;
        *info = PClassInfo(index == 0 ? kProcessorUID.toTUID() : kControllerUID.toTUID(),
            PClassInfo::kManyInstances, index == 0 ? kVstAudioEffectClass : kVstComponentControllerClass,
            "Rocoflex"); return kResultOk;
    }
    tresult PLUGIN_API getClassInfo2(int32 index, PClassInfo2* info) override {
        if (!info || index < 0 || index >= 2) return kInvalidArgument;
        *info = PClassInfo2(index == 0 ? kProcessorUID.toTUID() : kControllerUID.toTUID(),
            PClassInfo::kManyInstances, index == 0 ? kVstAudioEffectClass : kVstComponentControllerClass,
            "Rocoflex", 0, "Fx",
            "Rocoflex", kVersion, kVstVersionString); return kResultOk;
    }
    tresult PLUGIN_API getClassInfoUnicode(int32 index, PClassInfoW* info) override {
        if (!info) return kInvalidArgument;
        PClassInfo2 ascii; const auto result = getClassInfo2(index,&ascii);
        if (result == kResultOk) info->fromAscii(ascii);
        return result;
    }
    tresult PLUGIN_API createInstance(FIDString cid, FIDString iid, void** object) override {
        if (!cid || !iid || !object) return kInvalidArgument; *object = nullptr;
        try {
            FUnknown* instance = nullptr;
            if (matches(cid,kProcessorUID)) instance = createProcessor();
            else if (matches(cid,kControllerUID)) instance = static_cast<IEditController*>(new Controller());
            else return kNoInterface;
            if (!instance) return kOutOfMemory;
            const auto result = instance->queryInterface(iid,object);
            instance->release(); return result;
        } catch (const std::bad_alloc&) { return kOutOfMemory; }
          catch (...) { return kInternalError; }
    }
    tresult PLUGIN_API setHostContext(FUnknown* context) override {
        if (context) context->addRef();
        if (host_) host_->release(); host_ = context; return kResultOk;
    }
private:
    std::atomic<uint32> references_{1};
    FUnknown* host_ = nullptr;
};
}
}

extern "C" SMTG_EXPORT_SYMBOL Steinberg::IPluginFactory* PLUGIN_API GetPluginFactory() {
    return new (std::nothrow) Rocoflex::Factory();
}
extern "C" SMTG_EXPORT_SYMBOL bool PLUGIN_API InitDll() { return true; }
extern "C" SMTG_EXPORT_SYMBOL bool PLUGIN_API ExitDll() { return true; }
