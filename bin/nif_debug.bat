@echo off
rem Step 45: NIF デバッグ用 — RUST_BACKTRACE と RUST_LOG を有効にしてアプリを起動
rem - NIF パニック時に Rust のバックトレースが表示される
rem - RUST_LOG=debug で Rust 側の debug ログ（毎フレームは出さない）
rem - 毎フレームの physics ログを見る場合は RUST_LOG=trace（かなり多くなる）
if not defined ERLANG_HOME set "ERLANG_HOME=C:\Program Files\Erlang OTP"
if not defined ELIXIR_HOME set "ELIXIR_HOME=C:\Program Files\Elixir"
set "PATH=%ERLANG_HOME%\bin;%ELIXIR_HOME%\bin;%PATH%"
set "RUST_BACKTRACE=1"
set "RUST_LOG=debug"
pushd "%~dp0.."
mix run --no-halt
popd
