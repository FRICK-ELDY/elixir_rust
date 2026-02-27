@echo off
setlocal
rem ============================================================
rem  umbrella_compile.bat
rem  Umbrella コンパイル（Elixir + Rust NIF ビルド）
rem
rem  使い方:
rem    bin\umbrella_compile.bat          # 通常コンパイル
rem    bin\umbrella_compile.bat clean    # クリーン後コンパイル
rem
rem  初回は Rust NIF のビルドが走るため数分かかります。
rem ============================================================

if not defined ERLANG_HOME set "ERLANG_HOME=%ProgramFiles%\Erlang OTP"
if not defined ELIXIR_HOME set "ELIXIR_HOME=%ProgramFiles%\Elixir"
where erl >nul 2>&1 || set "PATH=%PATH%;%ERLANG_HOME%\bin"
where mix >nul 2>&1 || set "PATH=%PATH%;%ELIXIR_HOME%\bin"

rem Umbrella ルートへ移動
pushd "%~dp0..\umbrella"

if "%1"=="clean" (
    echo [umbrella_compile] Cleaning build artifacts ...
    mix clean
    echo.
)

echo [umbrella_compile] Compiling Umbrella (Elixir + Rust NIF) ...
echo   cwd: %CD%
echo.

mix deps.get
mix compile

if %ERRORLEVEL% neq 0 (
    echo.
    echo [umbrella_compile] ERROR: Compilation failed. See above for details.
    popd
    exit /b 1
)

echo.
echo [umbrella_compile] Done.
popd
