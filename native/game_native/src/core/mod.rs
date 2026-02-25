//! Path: native/game_native/src/core/mod.rs
//! Summary: ゲームコア共通モジュール（main/lib 共有ロジックの集約）
//!
//! main.rs（スタンドアロン）と lib.rs（NIF）の両方で共有するロジックを集約。
//! 重複管理コストを解消し、一箇所の修正で両方に反映されるようにする。

pub mod boss;
pub mod constants;
pub mod enemy;
pub mod entity_params;
pub mod item;
pub mod physics;
pub mod util;
pub mod weapon;
