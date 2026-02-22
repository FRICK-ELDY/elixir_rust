mod renderer;

use std::collections::HashSet;
use std::sync::Arc;
use std::time::Instant;

use renderer::{Renderer, SPRITE_SIZE};
use winit::{
    application::ApplicationHandler,
    event::{ElementState, KeyEvent, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowId},
};

const PLAYER_SPEED: f32 = 200.0; // ピクセル/秒

#[derive(Default)]
struct App {
    window:     Option<Arc<Window>>,
    renderer:   Option<Renderer>,
    keys_held:  HashSet<KeyCode>,
    player_x:   f32,
    player_y:   f32,
    last_update: Option<Instant>,
}

impl App {
    fn new() -> Self {
        Self {
            window:      None,
            renderer:    None,
            keys_held:   HashSet::new(),
            player_x:    640.0 - SPRITE_SIZE / 2.0,
            player_y:    360.0 - SPRITE_SIZE / 2.0,
            last_update: None,
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = Arc::new(
            event_loop
                .create_window(
                    Window::default_attributes()
                        .with_title("Elixir x Rust Survivor")
                        .with_inner_size(winit::dpi::LogicalSize::new(1280u32, 720u32)),
                )
                .expect("ウィンドウの作成に失敗しました"),
        );

        let renderer = pollster::block_on(Renderer::new(window.clone()));

        self.window   = Some(window);
        self.renderer = Some(renderer);
        self.last_update = Some(Instant::now());
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),

            WindowEvent::Resized(size) => {
                if let Some(renderer) = self.renderer.as_mut() {
                    renderer.resize(size.width, size.height);
                }
            }

            WindowEvent::KeyboardInput {
                event: KeyEvent {
                    physical_key: PhysicalKey::Code(code),
                    state,
                    ..
                },
                ..
            } => {
                match state {
                    ElementState::Pressed  => { self.keys_held.insert(code); }
                    ElementState::Released => { self.keys_held.remove(&code); }
                }
            }

            WindowEvent::RedrawRequested => {
                // 経過時間を計算してプレイヤーを移動
                let now = Instant::now();
                if let Some(last) = self.last_update {
                    let dt = now.duration_since(last).as_secs_f32();

                    let mut dx = 0.0f32;
                    let mut dy = 0.0f32;
                    if self.keys_held.contains(&KeyCode::KeyW) || self.keys_held.contains(&KeyCode::ArrowUp)    { dy -= 1.0; }
                    if self.keys_held.contains(&KeyCode::KeyS) || self.keys_held.contains(&KeyCode::ArrowDown)  { dy += 1.0; }
                    if self.keys_held.contains(&KeyCode::KeyA) || self.keys_held.contains(&KeyCode::ArrowLeft)  { dx -= 1.0; }
                    if self.keys_held.contains(&KeyCode::KeyD) || self.keys_held.contains(&KeyCode::ArrowRight) { dx += 1.0; }

                    let len = (dx * dx + dy * dy).sqrt();
                    if len > 0.001 {
                        self.player_x += (dx / len) * PLAYER_SPEED * dt;
                        self.player_y += (dy / len) * PLAYER_SPEED * dt;
                    }

                    // 画面端クランプ（1280x720 想定）
                    self.player_x = self.player_x.clamp(0.0, 1280.0 - SPRITE_SIZE);
                    self.player_y = self.player_y.clamp(0.0, 720.0  - SPRITE_SIZE);
                }
                self.last_update = Some(now);

                if let Some(renderer) = self.renderer.as_mut() {
                    renderer.update_player(self.player_x, self.player_y);
                    renderer.render();
                }
                if let Some(window) = self.window.as_ref() {
                    window.request_redraw();
                }
            }

            _ => {}
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let event_loop = EventLoop::new()?;
    let mut app = App::new();
    event_loop.run_app(&mut app)?;
    Ok(())
}
