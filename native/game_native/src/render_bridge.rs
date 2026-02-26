//! Path: native/game_native/src/render_bridge.rs
//! Summary: game_window の RenderBridge 実装（1.8.4）

use crate::asset::AssetLoader;
use crate::render_snapshot::build_render_frame;
use crate::world::GameWorld;
use game_core::constants::{SCREEN_HEIGHT, SCREEN_WIDTH};
use game_render::RenderFrame;
use game_window::{run_render_loop, RenderBridge, RendererInit, WindowConfig};
use rustler::ResourceArc;

pub fn run_render_thread(world: ResourceArc<GameWorld>) {
    let bridge = NativeRenderBridge { world };
    let loader = AssetLoader::new();

    let config = WindowConfig {
        title: "Elixir x Rust Survivor".to_string(),
        width: SCREEN_WIDTH as u32,
        height: SCREEN_HEIGHT as u32,
        renderer_init: RendererInit {
            atlas_png: loader.load_sprite_atlas(),
        },
    };

    if let Err(e) = run_render_loop(bridge, config) {
        eprintln!("Render thread: {e}");
    }
}

struct NativeRenderBridge {
    world: ResourceArc<GameWorld>,
}

impl RenderBridge for NativeRenderBridge {
    fn next_frame(&self) -> RenderFrame {
        match self.world.0.read() {
            Ok(guard) => build_render_frame(&guard),
            Err(e) => {
                log::error!("Render bridge: read lock poisoned in next_frame: {e:?}");
                let guard = e.into_inner();
                build_render_frame(&guard)
            }
        }
    }

    fn on_move_input(&self, dx: f32, dy: f32) {
        match self.world.0.write() {
            Ok(mut guard) => {
                guard.player.input_dx = dx;
                guard.player.input_dy = dy;
            }
            Err(e) => {
                log::error!("Render bridge: failed to acquire write lock for input: {e:?}");
            }
        }
    }

    fn on_ui_action(&self, action: String) {
        match self.world.0.read() {
            Ok(guard) => {
                if let Ok(mut pending) = guard.pending_ui_action.lock() {
                    *pending = Some(action);
                }
            }
            Err(e) => {
                log::error!("Render bridge: failed to acquire read lock for ui action: {e:?}");
            }
        }
    }
}
