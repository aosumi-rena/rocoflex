// Included once inside controller.cpp's Rocoflex namespace.
namespace { constexpr ProgramListID kFactoryPresetListId = 1886548852; }

int32 PLUGIN_API Controller::getUnitCount() { return 1; }
tresult PLUGIN_API Controller::getUnitInfo(int32 index, UnitInfo& info) {
    if (index != 0) {
        if (!unitBackingSeen_) info = {};
        return kResultFalse;
    }
    info.id = kRootUnitId; info.parentUnitId = kNoParentUnitId;
    info.programListId = unitBackingSeen_ ? kFactoryPresetListId : kNoProgramListId;
    text16(info.name, "Root Unit"); return kResultOk;
}
int32 PLUGIN_API Controller::getProgramListCount() { return unitBackingSeen_ ? 1 : 0; }
tresult PLUGIN_API Controller::getProgramListInfo(int32 index, ProgramListInfo& info) {
    if (!unitBackingSeen_ || index != 0) { info = {}; return kResultFalse; }
    info.id = kFactoryPresetListId; info.programCount = 1;
    text16(info.name, "Factory Presets"); return kResultOk;
}
tresult PLUGIN_API Controller::getProgramName(ProgramListID list, int32 index, String128 name) {
    if (!name) return kInvalidArgument;
    text16(name, 128, "");
    return unitBackingSeen_ && list == kFactoryPresetListId && index == 0 ? kResultOk : kResultFalse;
}
tresult PLUGIN_API Controller::getProgramInfo(ProgramListID, int32, Steinberg::Vst::CString, String128) {
    return unitBackingSeen_ ? kNotImplemented : kResultFalse;
}
tresult PLUGIN_API Controller::hasProgramPitchNames(ProgramListID, int32) {
    return unitBackingSeen_ ? kNotImplemented : kResultFalse;
}
tresult PLUGIN_API Controller::getProgramPitchName(ProgramListID, int32, int16, String128) {
    return unitBackingSeen_ ? kNotImplemented : kResultFalse;
}
UnitID PLUGIN_API Controller::getSelectedUnit() {
    return unitBackingSeen_ ? kRootUnitId : 1;
}
tresult PLUGIN_API Controller::selectUnit(UnitID) {
    return unitBackingSeen_ ? kNotImplemented : kResultFalse;
}
tresult PLUGIN_API Controller::getUnitByBus(MediaType, BusDirection, int32, int32, UnitID& unit) {
    if (!unitBackingSeen_) return kResultFalse;
    unit = kRootUnitId; return kResultOk;
}
tresult PLUGIN_API Controller::setUnitProgramData(int32, int32, IBStream*) {
    return unitBackingSeen_ ? kNotImplemented : kResultFalse;
}
