@echo off
pushd "%~dp0..\native\game_native"
cargo test %*
popd
