@echo off
rem NIF を完全に再ビルドする（mix clean + mix compile）
if not defined ERLANG_HOME set "ERLANG_HOME=%ProgramFiles%\Erlang OTP"
if not defined ELIXIR_HOME set "ELIXIR_HOME=%ProgramFiles%\Elixir"
set "PATH=%ERLANG_HOME%\bin;%ELIXIR_HOME%\bin;%PATH%"
pushd "%~dp0.."
mix clean
mix compile
popd
