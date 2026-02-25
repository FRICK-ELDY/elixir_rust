@echo off
rem iex -S mix のラッパー（PowerShell の iex エイリアスを回避）
if not defined ERLANG_HOME set "ERLANG_HOME=%ProgramFiles%\Erlang OTP"
if not defined ELIXIR_HOME set "ELIXIR_HOME=%ProgramFiles%\Elixir"
where erl >nul 2>&1 || set "PATH=%PATH%;%ERLANG_HOME%\bin"
where iex >nul 2>&1 || set "PATH=%PATH%;%ELIXIR_HOME%\bin"
pushd "%~dp0.."
iex -S mix
popd
