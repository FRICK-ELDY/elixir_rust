@echo off
rem NIF を完全に再ビルドする（mix clean + mix compile）
if not defined ERLANG_HOME set "ERLANG_HOME=%ProgramFiles%\Erlang OTP"
if not defined ELIXIR_HOME set "ELIXIR_HOME=%ProgramFiles%\Elixir"
where erl >nul 2>&1 || set "PATH=%PATH%;%ERLANG_HOME%\bin"
where mix >nul 2>&1 || set "PATH=%PATH%;%ELIXIR_HOME%\bin"
pushd "%~dp0.."
mix clean
mix compile
popd
