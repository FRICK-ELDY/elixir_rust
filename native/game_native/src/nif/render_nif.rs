//! Path: native/game_native/src/nif/render_nif.rs
//! Summary: 描画スレッド起動 NIF（1.7.4）
//!
//! NIF から描画用スレッドを spawn し、そのスレッドで winit の EventLoop・
//! ウィンドウ作成・wgpu 初期化の骨組みを実行する。

use crate::render_thread::run_render_thread;
use crate::world::GameWorld;
use rustler::{Atom, NifResult, ResourceArc};
use std::panic::AssertUnwindSafe;
use std::thread;

use crate::ok;

#[rustler::nif]
pub fn start_render_thread(world: ResourceArc<GameWorld>) -> NifResult<Atom> {
    let world_clone = world.clone();

    thread::spawn(move || {
        if let Err(e) = std::panic::catch_unwind(AssertUnwindSafe(move || {
            run_render_thread(world_clone);
        })) {
            eprintln!("Render thread panicked: {:?}", e);
        }
    });

    Ok(ok())
}
