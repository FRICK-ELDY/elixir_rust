@echo off
rem Step 45: NIF デバッグ用 — iex を起動（debug_dump_world 等を対話的に呼ぶ）
rem RUST_LOG=debug では毎フレームログは出さない。毎フレーム見る場合は RUST_LOG=trace
rem 例: world = Engine.create_world(); App.NifBridge.debug_dump_world(world)
if not defined ERLANG_HOME set "ERLANG_HOME=C:\Program Files\Erlang OTP"
if not defined ELIXIR_HOME set "ELIXIR_HOME=C:\Program Files\Elixir"
set "PATH=%ERLANG_HOME%\bin;%ELIXIR_HOME%\bin;%PATH%"
set "RUST_BACKTRACE=1"
set "RUST_LOG=debug"
pushd "%~dp0.."
iex -S mix
popd
