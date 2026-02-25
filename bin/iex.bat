@echo off
rem iex -S mix のラッパー（PowerShell の iex エイリアスを回避）
if not defined ERLANG_HOME set "ERLANG_HOME=%ProgramFiles%\Erlang OTP"
if not defined ELIXIR_HOME set "ELIXIR_HOME=%ProgramFiles%\Elixir"
set "PATH=%ERLANG_HOME%\bin;%ELIXIR_HOME%\bin;%PATH%"
pushd "%~dp0.."
iex -S mix
popd
