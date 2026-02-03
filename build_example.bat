@echo off
echo Building C example for Windows...

REM 检查是否存在必要的文件
if not exist "hid_monitor.h" (
    echo Error: hid_monitor.h not found!
    exit /b 1
)

if not exist "example.c" (
    echo Error: example.c not found!
    exit /b 1
)

if not exist "target\release\hid_monitor.dll" (
    echo Error: hid_monitor.dll not found! Please run 'cargo build --release' first.
    exit /b 1
)

if not exist "target\release\hid_monitor.dll.lib" (
    echo Error: hid_monitor.dll.lib not found! Please run 'cargo build --release' first.
    exit /b 1
)

REM 复制必要的文件到当前目录
copy "target\release\hid_monitor.dll" .
copy "target\release\hid_monitor.dll.lib" .

REM 尝试使用MSVC编译
where cl >nul 2>nul
if %ERRORLEVEL% == 0 (
    echo Using MSVC compiler...
    cl example.c /I. hid_monitor.dll.lib /Fe:example.exe
    if %ERRORLEVEL% == 0 (
        echo Build successful! Run example.exe to test.
        goto :end
    )
)

REM 尝试使用MinGW编译
where gcc >nul 2>nul
if %ERRORLEVEL% == 0 (
    echo Using MinGW compiler...
    gcc example.c -I. -L. -lhid_monitor -o example.exe
    if %ERRORLEVEL% == 0 (
        echo Build successful! Run example.exe to test.
        goto :end
    )
)

echo Error: No suitable compiler found! Please install MSVC or MinGW.
exit /b 1

:end
echo.
echo Files in current directory:
dir *.exe *.dll 2>nul
echo.
echo You can now run: example.exe