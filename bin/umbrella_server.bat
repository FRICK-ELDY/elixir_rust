@echo off
rem ============================================================
rem  umbrella_server.bat
rem  Umbrella サーバー起動（headless: true・Rust 描画/音なし）
rem
rem  使い方:
rem    bin\umbrella_server.bat
rem
rem  本番デプロイ時は MIX_ENV=prod に変更してください。
rem  Phoenix Endpoint は game_network アプリが起動します。
rem  ポート: 4000（umbrella/config/prod.exs で変更可）
rem ============================================================

if not defined ERLANG_HOME set "ERLANG_HOME=%ProgramFiles%\Erlang OTP"
if not defined ELIXIR_HOME set "ELIXIR_HOME=%ProgramFiles%\Elixir"
where erl >nul 2>&1 || set "PATH=%PATH%;%ERLANG_HOME%\bin"
where iex >nul 2>&1 || set "PATH=%PATH%;%ELIXIR_HOME%\bin"

rem Umbrella ルートへ移動
pushd "%~dp0..\umbrella"

rem ヘッドレスモード強制（Rust 描画・音スレッドをロードしない）
set GAME_ENGINE_HEADLESS=true

rem 環境（dev / prod）
if "%MIX_ENV%"=="" set MIX_ENV=dev

echo [umbrella_server] Starting Umbrella (server / headless: true) ...
echo   cwd:     %CD%
echo   MIX_ENV: %MIX_ENV%
echo.

iex -S mix

popd
