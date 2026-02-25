//! Path: native/game_native/src/render_thread.rs
//! Summary: 描画スレッドのエントリ（1.7.4 / 1.7.5）
//!
//! winit の EventLoop・ウィンドウ作成・wgpu 初期化。
//! 1.7.5: GameWorld を read して RenderSnapshot を構築し、renderer に渡す。

use game_core::constants::{SCREEN_HEIGHT, SCREEN_WIDTH};
use std::sync::Arc;
use winit::{
    application::ApplicationHandler,
    event::{ElementState, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowId},
};

#[cfg(target_os = "windows")]
use winit::platform::windows::EventLoopBuilderExtWindows;

use crate::asset::{AssetId, AssetLoader};
use crate::render_snapshot::build_render_snapshot;
use crate::renderer::{Renderer, GameUiState};
use crate::world::GameWorld;
use rustler::ResourceArc;

/// 描画スレッドのメインエントリ。winit EventLoop がブロックする。
pub fn run_render_thread(world: ResourceArc<GameWorld>) {
    let mut builder = EventLoop::builder();
    #[cfg(target_os = "windows")]
    builder.with_any_thread(true);

    let event_loop = match builder.build() {
        Ok(el) => el,
        Err(e) => {
            eprintln!("Render thread: Failed to create event loop: {}", e);
            return;
        }
    };

    let mut app = RenderApp::new(world);

    if let Err(e) = event_loop.run_app(&mut app) {
        eprintln!("Render thread: Event loop error: {}", e);
    }
}

/// 描画スレッド用の ApplicationHandler
struct RenderApp {
    /// 1.7.5: read してスナップショット取得
    world:           ResourceArc<GameWorld>,
    window:          Option<Arc<Window>>,
    /// 1.7.5: Renderer（wgpu + egui HUD）
    renderer:        Option<Renderer>,
    /// egui 用 UI 状態（セーブ/ロード等）
    ui_state:        GameUiState,
    /// キー状態（WASD / 矢印）
    move_up:         bool,
    move_down:       bool,
    move_left:       bool,
    move_right:      bool,
}

impl RenderApp {
    fn new(world: ResourceArc<GameWorld>) -> Self {
        Self {
            world,
            window:       None,
            renderer:     None,
            ui_state:     GameUiState::default(),
            move_up:      false,
            move_down:    false,
            move_left:    false,
            move_right:   false,
        }
    }

    fn set_move_key(&mut self, key: KeyCode, pressed: bool) -> bool {
        let target = match key {
            KeyCode::KeyW | KeyCode::ArrowUp => &mut self.move_up,
            KeyCode::KeyS | KeyCode::ArrowDown => &mut self.move_down,
            KeyCode::KeyA | KeyCode::ArrowLeft => &mut self.move_left,
            KeyCode::KeyD | KeyCode::ArrowRight => &mut self.move_right,
            _ => return false,
        };

        if *target == pressed {
            return false;
        }

        *target = pressed;
        true
    }

    fn clear_move_keys(&mut self) -> bool {
        let had_pressed = self.move_up || self.move_down || self.move_left || self.move_right;
        self.move_up = false;
        self.move_down = false;
        self.move_left = false;
        self.move_right = false;
        had_pressed
    }

    fn sync_player_input(&self) {
        let dx = (self.move_right as i8 - self.move_left as i8) as f32;
        let dy = (self.move_down as i8 - self.move_up as i8) as f32;

        match self.world.0.write() {
            Ok(mut guard) => {
                guard.player.input_dx = dx;
                guard.player.input_dy = dy;
            }
            Err(e) => {
                log::error!("Render thread: Failed to acquire write lock for input: {e:?}");
            }
        }
    }
}

impl ApplicationHandler for RenderApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let window = Arc::new(
            event_loop
                .create_window(
                    Window::default_attributes()
                        .with_title("Elixir x Rust Survivor")
                        .with_inner_size(winit::dpi::LogicalSize::new(
                            SCREEN_WIDTH as u32,
                            SCREEN_HEIGHT as u32,
                        )),
                )
                .expect("ウィンドウの作成に失敗しました"),
        );

        // 1.7.5: アトラスをロードして Renderer を初期化
        let loader = AssetLoader::new();
        let atlas_bytes = loader.load_bytes(AssetId::SpriteAtlas);

        let renderer = pollster::block_on(Renderer::new(window.clone(), &atlas_bytes));

        self.window   = Some(window.clone());
        self.renderer = Some(renderer);

        window.request_redraw();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        // egui にイベントを転送
        if let (Some(renderer), Some(window)) = (&mut self.renderer, &self.window) {
            if renderer.handle_window_event(window, &event) {
                window.request_redraw();
            }
        }

        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }

            WindowEvent::Focused(false) => {
                if self.clear_move_keys() {
                    self.sync_player_input();
                }
            }

            WindowEvent::KeyboardInput { event, .. } => {
                if event.repeat {
                    return;
                }
                if let PhysicalKey::Code(code) = event.physical_key {
                    let pressed = event.state == ElementState::Pressed;
                    if self.set_move_key(code, pressed) {
                        self.sync_player_input();
                    }
                }
            }

            WindowEvent::Resized(size) => {
                if let (Some(renderer), size) = (&mut self.renderer, (size.width, size.height)) {
                    if size.0 > 0 && size.1 > 0 {
                        renderer.resize(size.0, size.1);
                    }
                }
            }

            WindowEvent::RedrawRequested => {
                if let (Some(renderer), Some(window)) = (&mut self.renderer, &self.window) {
                    // 1.7.5: read でロック取得 → スナップショット構築 → ロック解放
                    let snapshot = {
                        let guard = match self.world.0.read() {
                            Ok(g) => g,
                            Err(e) => {
                                log::error!("Render thread: Failed to acquire read lock for snapshot: {e:?}");
                                return;
                            }
                        };
                        build_render_snapshot(&guard)
                    };
                    // ロック解放済み。ここから描画（ロック外で wgpu 描画）
                    renderer.update_instances(
                        &snapshot.render_data,
                        &snapshot.particle_data,
                        &snapshot.item_data,
                        &snapshot.obstacle_data,
                        snapshot.camera_offset,
                    );
                    if let Some(action) = renderer.render(window, &snapshot.hud, &mut self.ui_state) {
                        let guard = match self.world.0.read() {
                            Ok(g) => g,
                            Err(e) => {
                                log::error!("Render thread: Failed to acquire read lock on world: {e:?}");
                                return;
                            }
                        };
                        if let Ok(mut pending) = guard.pending_ui_action.lock() {
                            *pending = Some(action);
                        };
                    }

                    window.request_redraw();
                }
            }

            _ => {}
        }
    }
}
