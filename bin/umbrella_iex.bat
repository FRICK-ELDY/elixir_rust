@echo off
setlocal
rem ============================================================
rem  umbrella_iex.bat
rem  Umbrella ローカル起動（ゲーム画面あり・headless: false）
rem
rem  使い方:
rem    bin\umbrella_iex.bat
rem
rem  起動後に iex で操作できるコマンド例:
rem    GameEngine.list_rooms()
rem    GameEngine.start_room("room_2")
rem    GameEngine.stop_room("room_2")
rem    GameEngine.save_session(...)
rem ============================================================

if not defined ERLANG_HOME set "ERLANG_HOME=%ProgramFiles%\Erlang OTP"
if not defined ELIXIR_HOME set "ELIXIR_HOME=%ProgramFiles%\Elixir"
where erl >nul 2>&1 || set "PATH=%PATH%;%ERLANG_HOME%\bin"
where iex >nul 2>&1 || set "PATH=%PATH%;%ELIXIR_HOME%\bin"

rem Umbrella ルートへ移動
pushd "%~dp0..\umbrella"

rem ゲームアセット ID（未設定時は vampire_survivor）
if "%GAME_ASSETS_ID%"=="" set GAME_ASSETS_ID=vampire_survivor

echo [umbrella_iex] Starting Umbrella (local / headless: false) ...
echo   cwd: %CD%
echo   GAME_ASSETS_ID: %GAME_ASSETS_ID%
echo.

iex -S mix

popd
