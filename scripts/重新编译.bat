@echo off
chcp 65001 >nul
cd /d "%~dp0.."

echo === [1/4] 版本号 +1 ===
for /f "tokens=*" %%v in ('python scripts\version.py --bump') do set NEWVER=%%v
echo 新版本: %NEWVER%

echo === [2/4] 构建前端 ===
cd ui
call npx vite build
if errorlevel 1 goto :err
cd ..

echo === [3/4] 编译桌面程序(2-5 分钟)===
cd rustatio-desktop
cargo build --release
if errorlevel 1 goto :err
cd ..

echo === [4/4] 复制产物 ===
copy /y target\release\rustatio-desktop.exe "..\Rustatio-中文合并版-%NEWVER%.exe" >nul
if errorlevel 1 goto :err

echo.
echo === 完成: ..\Rustatio-中文合并版-%NEWVER%.exe (标题栏显示 vN) ===
pause
exit /b 0

:err
echo === 构建失败,请查看上方错误信息 ===
pause
exit /b 1
