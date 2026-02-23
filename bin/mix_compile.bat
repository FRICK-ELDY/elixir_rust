@echo off
set "PATH=C:\Program Files\Erlang OTP\bin;C:\Program Files\Elixir\bin;%PATH%"
pushd "%~dp0.."
mix deps.get
mix compile
popd
