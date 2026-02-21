@echo off
cd /d "%~dp0.."
cargo run --bin game_window --manifest-path native\game_native\Cargo.toml
