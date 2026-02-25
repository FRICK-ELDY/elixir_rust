//! Path: native/game_native/src/render_thread.rs
//! Summary: 描画スレッドのエントリ（1.7.4）
//!
//! winit の EventLoop・ウィンドウ作成・wgpu 初期化の骨組みを実装。
//! 1.7.5 で GameWorld から描画データ取得、RenderSnapshot 接続を行う。

use game_core::constants::{BG_B, BG_G, BG_R, SCREEN_HEIGHT, SCREEN_WIDTH};
use std::sync::Arc;
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowId},
};

#[cfg(target_os = "windows")]
use winit::platform::windows::EventLoopBuilderExtWindows;

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
    /// 1.7.5 で read してスナップショット取得する。1.7.4 では未使用。
    #[allow(dead_code)]
    world:        ResourceArc<GameWorld>,
    window:       Option<Arc<Window>>,
    /// wgpu 初期化の骨組み（surface, device, queue, config）
    wgpu_surface: Option<wgpu::Surface<'static>>,
    wgpu_device:  Option<wgpu::Device>,
    wgpu_queue:   Option<wgpu::Queue>,
    wgpu_config:  Option<wgpu::SurfaceConfiguration>,
}

impl RenderApp {
    fn new(world: ResourceArc<GameWorld>) -> Self {
        Self {
            world,
            window:       None,
            wgpu_surface: None,
            wgpu_device:  None,
            wgpu_queue:   None,
            wgpu_config:  None,
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

        let instance = wgpu::Instance::default();
        let surface = instance
            .create_surface(window.clone())
            .expect("サーフェスの作成に失敗しました");

        let adapter = pollster::block_on(
            instance.request_adapter(&wgpu::RequestAdapterOptions {
                power_preference:   wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                ..Default::default()
            }),
        )
        .expect("アダプターの取得に失敗しました");

        let (device, queue) = pollster::block_on(
            adapter.request_device(&wgpu::DeviceDescriptor::default(), None),
        )
        .expect("デバイスとキューの取得に失敗しました");

        let size = window.inner_size();
        let config = surface
            .get_default_config(&adapter, size.width, size.height)
            .expect("サーフェス設定の取得に失敗しました");
        surface.configure(&device, &config);

        self.window       = Some(window.clone());
        self.wgpu_surface = Some(surface);
        self.wgpu_device  = Some(device);
        self.wgpu_queue   = Some(queue);
        self.wgpu_config  = Some(config);

        window.request_redraw();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }

            WindowEvent::Resized(size) => {
                if let (Some(device), Some(surface), Some(config)) = (
                    &self.wgpu_device,
                    &self.wgpu_surface,
                    &mut self.wgpu_config,
                ) {
                    if size.width > 0 && size.height > 0 {
                        config.width  = size.width;
                        config.height = size.height;
                        surface.configure(device, config);
                    }
                }
            }

            WindowEvent::RedrawRequested => {
                if let (Some(surface), Some(device), Some(queue), Some(config), Some(window)) = (
                    &self.wgpu_surface,
                    &self.wgpu_device,
                    &self.wgpu_queue,
                    &self.wgpu_config,
                    &self.window,
                ) {
                    let output = match surface.get_current_texture() {
                        Ok(t) => t,
                        Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                            surface.configure(device, config);
                            return;
                        }
                        Err(e) => {
                            eprintln!("Surface error: {:?}", e);
                            return;
                        }
                    };

                    let view = output.texture.create_view(&Default::default());
                    let mut encoder =
                        device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                            label: Some("Clear Encoder"),
                        });

                    {
                        let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                            label: Some("Clear Pass"),
                            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                view:           &view,
                                resolve_target: None,
                                ops:            wgpu::Operations {
                                    load:  wgpu::LoadOp::Clear(wgpu::Color {
                                        r: BG_R,
                                        g: BG_G,
                                        b: BG_B,
                                        a: 1.0,
                                    }),
                                    store: wgpu::StoreOp::Store,
                                },
                            })],
                            depth_stencil_attachment: None,
                            timestamp_writes:         None,
                            occlusion_query_set:      None,
                        });
                    }

                    queue.submit(std::iter::once(encoder.finish()));
                    output.present();

                    window.request_redraw();
                }
            }

            _ => {}
        }
    }
}
