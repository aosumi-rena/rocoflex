@echo off
setlocal
if not defined VST3_SDK_DIR (
    echo Set VST3_SDK_DIR to the VST3 SDK 3.8.1 root. 1>&2
    exit /b 1
)
for %%I in ("%VST3_SDK_DIR%") do set "VST3_SDK_DIR=%%~fI"
if not exist "%VST3_SDK_DIR%\pluginterfaces\base\funknown.cpp" (
    echo VST3 SDK pluginterfaces not found. 1>&2
    exit /b 1
)
if defined VSCMD_ARG_TGT_ARCH if /i not "%VSCMD_ARG_TGT_ARCH%"=="x64" (
    echo Use an x64 Native Tools command prompt. 1>&2
    exit /b 1
)
where.exe cl.exe >nul 2>&1
if errorlevel 1 (
    echo Install Visual Studio C++ Build Tools and use an x64 Native Tools prompt. 1>&2
    exit /b 1
)
where.exe cargo.exe >nul 2>&1
if errorlevel 1 (
    echo Install Rust for x86_64-pc-windows-msvc and add Cargo to PATH. 1>&2
    exit /b 1
)
pushd "%~dp0.."
if errorlevel 1 exit /b 1
set "ROCO_ROOT=%CD%"
if not defined CARGO_TARGET_DIR set "CARGO_TARGET_DIR=%ROCO_ROOT%\target"
for %%I in ("%CARGO_TARGET_DIR%") do set "CARGO_TARGET_DIR=%%~fI"
set "ROCO_STAGE=%CARGO_TARGET_DIR%\rocoflex-vst3"
set "ROCO_BUNDLE=%ROCO_STAGE%\Rocoflex.vst3\Contents"
set "ROCOFLEX_BUILD_ID=rocoflex-current"
if not exist "%ROCO_STAGE%\objects" mkdir "%ROCO_STAGE%\objects"
if errorlevel 1 goto failed
if not exist "%ROCO_BUNDLE%\x86_64-win" mkdir "%ROCO_BUNDLE%\x86_64-win"
if errorlevel 1 goto failed
if not exist "%ROCO_BUNDLE%\Resources\licenses" mkdir "%ROCO_BUNDLE%\Resources\licenses"
if errorlevel 1 goto failed
cargo build --release --target x86_64-pc-windows-msvc --lib --locked
if errorlevel 1 goto failed
cl.exe /nologo /std:c++17 /EHsc /O2 /MD /W4 /DNOMINMAX /DWIN32_LEAN_AND_MEAN /D_CRT_SECURE_NO_WARNINGS /LD /I"%VST3_SDK_DIR%" "%ROCO_ROOT%\vst3\factory.cpp" "%ROCO_ROOT%\vst3\controller.cpp" "%ROCO_ROOT%\vst3\processor.cpp" "%ROCO_ROOT%\vst3\plugin_protocol.cpp" "%VST3_SDK_DIR%\pluginterfaces\base\funknown.cpp" /Fo"%ROCO_STAGE%\objects\\" /link /OUT:"%ROCO_BUNDLE%\x86_64-win\Rocoflex.vst3" /IMPLIB:"%ROCO_STAGE%\Rocoflex-plugin.lib" "%CARGO_TARGET_DIR%\x86_64-pc-windows-msvc\release\rocoflex.lib" kernel32.lib user32.lib gdi32.lib ole32.lib oleaut32.lib shell32.lib shlwapi.lib comdlg32.lib advapi32.lib avrt.lib imm32.lib dwmapi.lib uxtheme.lib winmm.lib version.lib ntdll.lib userenv.lib bcrypt.lib ws2_32.lib d3d12.lib dxgi.lib dxguid.lib d3dcompiler.lib dcomp.lib uiautomationcore.lib opengl32.lib windowscodecs.lib propsys.lib runtimeobject.lib shcore.lib oleacc.lib setupapi.lib hid.lib rpcrt4.lib wintrust.lib crypt32.lib secur32.lib dbghelp.lib winhttp.lib msimg32.lib synchronization.lib windowsapp.lib
if errorlevel 1 goto failed
copy /y "%ROCO_ROOT%\vst3\moduleinfo.json" "%ROCO_BUNDLE%\Resources\moduleinfo.json" >nul
if errorlevel 1 goto failed
copy /y "%ROCO_ROOT%\LICENSE" "%ROCO_BUNDLE%\Resources\LICENSE" >nul
if errorlevel 1 goto failed
copy /y "%ROCO_ROOT%\licenses\*.txt" "%ROCO_BUNDLE%\Resources\licenses\" >nul
if errorlevel 1 goto failed
echo VST3 built: "%ROCO_STAGE%\Rocoflex.vst3"
popd
exit /b 0
:failed
popd
exit /b 1
