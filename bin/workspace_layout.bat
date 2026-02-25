@echo off
pushd "%~dp0..\native"
cargo run -p xtask -- workspace-layout %*
popd
