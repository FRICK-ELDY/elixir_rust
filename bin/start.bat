@echo off
pushd "%~dp0.."
rem Step 39: ゲーム別アセットパス（GAME_ASSETS_ID 未設定時は vampire_survivor をデフォルト）
if "%GAME_ASSETS_ID%"=="" set GAME_ASSETS_ID=vampire_survivor
cargo run --bin game_window --manifest-path native\game_native\Cargo.toml
popd
