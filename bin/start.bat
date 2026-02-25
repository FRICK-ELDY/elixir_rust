@echo off
if not defined ERLANG_HOME set "ERLANG_HOME=%ProgramFiles%\Erlang OTP"
if not defined ELIXIR_HOME set "ELIXIR_HOME=%ProgramFiles%\Elixir"
where erl >nul 2>&1 || set "PATH=%PATH%;%ERLANG_HOME%\bin"
where iex >nul 2>&1 || set "PATH=%PATH%;%ELIXIR_HOME%\bin"
pushd "%~dp0.."
rem 1.7.7: game_window 廃止後は iex -S mix で統合起動
rem ゲーム別アセットパス（GAME_ASSETS_ID 未設定時は vampire_survivor をデフォルト）
if "%GAME_ASSETS_ID%"=="" set GAME_ASSETS_ID=vampire_survivor
iex -S mix
popd
