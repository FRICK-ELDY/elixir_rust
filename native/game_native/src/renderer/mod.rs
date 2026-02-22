use crate::constants::{BG_B, BG_G, BG_R, SPRITE_SIZE};
use crate::item::{RENDER_KIND_GEM, RENDER_KIND_MAGNET, RENDER_KIND_POTION};
use std::sync::Arc;
use wgpu::util::DeviceExt;
use winit::window::Window;

// ─── 頂点・インデックス ────────────────────────────────────────

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 2],
}

const VERTICES: &[Vertex] = &[
    Vertex { position: [0.0, 0.0] }, // 左上
    Vertex { position: [1.0, 0.0] }, // 右上
    Vertex { position: [1.0, 1.0] }, // 右下
    Vertex { position: [0.0, 1.0] }, // 左下
];

const INDICES: &[u16] = &[0, 1, 2, 0, 2, 3];

// ─── インスタンスデータ ────────────────────────────────────────

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SpriteInstance {
    pub position:   [f32; 2], // ワールド座標（左上）
    pub size:       [f32; 2], // ピクセルサイズ
    pub uv_offset:  [f32; 2], // アトラス UV オフセット（0.0〜1.0）
    pub uv_size:    [f32; 2], // アトラス UV サイズ（0.0〜1.0）
    pub color_tint: [f32; 4], // RGBA 乗算カラー
}

// アトラス内の UV 座標（576x64 px アトラス）
// [0..63]    プレイヤー
// [64..127]  Slime（緑）
// [128..191] Bat（紫）
// [192..255] Golem（灰）
// [256..319] 弾丸
// [320..383] パーティクル
// [384..447] 経験値宝石（緑）
// [448..511] 回復ポーション（赤）
// [512..575] 磁石（黄）
const ATLAS_W: f32 = 576.0;
const ATLAS_H: f32 = 64.0;

pub fn player_uv() -> ([f32; 2], [f32; 2]) {
    ([0.0 / ATLAS_W, 0.0 / ATLAS_H], [64.0 / ATLAS_W, 64.0 / ATLAS_H])
}
pub fn slime_uv() -> ([f32; 2], [f32; 2]) {
    ([64.0 / ATLAS_W, 0.0 / ATLAS_H], [64.0 / ATLAS_W, 64.0 / ATLAS_H])
}
pub fn bat_uv() -> ([f32; 2], [f32; 2]) {
    ([128.0 / ATLAS_W, 0.0 / ATLAS_H], [64.0 / ATLAS_W, 64.0 / ATLAS_H])
}
pub fn golem_uv() -> ([f32; 2], [f32; 2]) {
    ([192.0 / ATLAS_W, 0.0 / ATLAS_H], [64.0 / ATLAS_W, 64.0 / ATLAS_H])
}
pub fn bullet_uv() -> ([f32; 2], [f32; 2]) {
    ([256.0 / ATLAS_W, 0.0 / ATLAS_H], [64.0 / ATLAS_W, 64.0 / ATLAS_H])
}
pub fn particle_uv() -> ([f32; 2], [f32; 2]) {
    ([320.0 / ATLAS_W, 0.0 / ATLAS_H], [64.0 / ATLAS_W, 64.0 / ATLAS_H])
}
pub fn gem_uv() -> ([f32; 2], [f32; 2]) {
    ([384.0 / ATLAS_W, 0.0 / ATLAS_H], [64.0 / ATLAS_W, 64.0 / ATLAS_H])
}
pub fn potion_uv() -> ([f32; 2], [f32; 2]) {
    ([448.0 / ATLAS_W, 0.0 / ATLAS_H], [64.0 / ATLAS_W, 64.0 / ATLAS_H])
}
pub fn magnet_uv() -> ([f32; 2], [f32; 2]) {
    ([512.0 / ATLAS_W, 0.0 / ATLAS_H], [64.0 / ATLAS_W, 64.0 / ATLAS_H])
}

// ─── 画面サイズ Uniform ────────────────────────────────────────

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct ScreenUniform {
    half_size: [f32; 2],
    _pad: [f32; 2],
}

impl ScreenUniform {
    fn new(width: u32, height: u32) -> Self {
        Self {
            half_size: [width as f32 / 2.0, height as f32 / 2.0],
            _pad: [0.0; 2],
        }
    }
}

// ─── インスタンスバッファの最大容量 ────────────────────────────
// Player 1 + Enemies 10000 + Bullets 2000 + Particles 2000 + Items 500 = 14501
const MAX_INSTANCES: usize = 14501;

// 敵タイプ別のスプライトサイズ（px）
// kind: 1=slime(40px), 2=bat(24px), 3=golem(64px)
fn enemy_sprite_size(kind: u8) -> f32 {
    match kind {
        2 => 24.0,  // Bat: 小さい
        3 => 64.0,  // Golem: 大きい
        _ => 40.0,  // Slime: 基本
    }
}

fn enemy_uv_for_kind(kind: u8) -> ([f32; 2], [f32; 2]) {
    match kind {
        2 => bat_uv(),
        3 => golem_uv(),
        _ => slime_uv(),
    }
}

// ─── HUD データ ────────────────────────────────────────────────

#[derive(Default, Clone)]
pub struct HudData {
    pub hp:               f32,
    pub max_hp:           f32,
    pub score:            u32,
    pub elapsed_seconds:  f32,
    pub level:            u32,
    pub exp:              u32,
    pub exp_to_next:      u32,
    pub enemy_count:      usize,
    pub bullet_count:     usize,
    // Populated from Renderer::current_fps each frame; passed to build_hud_ui
    #[allow(dead_code)]
    pub fps:              f32,
    pub level_up_pending: bool,
    pub weapon_choices:   Vec<String>,
    /// Step 17: 装備中の武器レベル [(weapon_name, level)]
    pub weapon_levels:    Vec<(String, u32)>,
    /// Step 19: 磁石エフェクト残り時間（秒）
    pub magnet_timer:     f32,
    /// Step 19: アイテム数
    pub item_count:       usize,
}

// ─── Renderer ─────────────────────────────────────────────────

pub struct Renderer {
    surface:              wgpu::Surface<'static>,
    device:               wgpu::Device,
    queue:                wgpu::Queue,
    config:               wgpu::SurfaceConfiguration,
    render_pipeline:      wgpu::RenderPipeline,
    vertex_buffer:        wgpu::Buffer,
    index_buffer:         wgpu::Buffer,
    instance_buffer:      wgpu::Buffer,
    instance_count:       u32,
    bind_group:           wgpu::BindGroup,
    screen_uniform_buf:   wgpu::Buffer,
    screen_bind_group:    wgpu::BindGroup,
    // egui
    egui_ctx:             egui::Context,
    egui_renderer:        egui_wgpu::Renderer,
    egui_winit:           egui_winit::State,
    // FPS 計測
    frame_count:          u32,
    fps_timer:            std::time::Instant,
    pub current_fps:      f32,
}

impl Renderer {
    pub async fn new(window: Arc<Window>) -> Self {
        let instance = wgpu::Instance::default();
        let surface = instance
            .create_surface(window.clone())
            .expect("サーフェスの作成に失敗しました");

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference:   wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                ..Default::default()
            })
            .await
            .expect("アダプターの取得に失敗しました");

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default(), None)
            .await
            .expect("デバイスとキューの取得に失敗しました");

        let size = window.inner_size();
        let config = surface
            .get_default_config(&adapter, size.width, size.height)
            .expect("サーフェス設定の取得に失敗しました");
        surface.configure(&device, &config);

        // ─── テクスチャアトラス ──────────────────────────────────
        let atlas_bytes = include_bytes!("../../../../assets/sprites/atlas.png");
        let atlas_image = image::load_from_memory(atlas_bytes)
            .expect("atlas.png の読み込みに失敗しました")
            .to_rgba8();
        let atlas_size = wgpu::Extent3d {
            width:                 atlas_image.width(),
            height:                atlas_image.height(),
            depth_or_array_layers: 1,
        };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label:           Some("Atlas Texture"),
            size:            atlas_size,
            mip_level_count: 1,
            sample_count:    1,
            dimension:       wgpu::TextureDimension::D2,
            format:          wgpu::TextureFormat::Rgba8UnormSrgb,
            usage:           wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats:    &[],
        });
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture:   &texture,
                mip_level: 0,
                origin:    wgpu::Origin3d::ZERO,
                aspect:    wgpu::TextureAspect::All,
            },
            &atlas_image,
            wgpu::TexelCopyBufferLayout {
                offset:         0,
                bytes_per_row:  Some(4 * atlas_image.width()),
                rows_per_image: Some(atlas_image.height()),
            },
            atlas_size,
        );
        let texture_view = texture.create_view(&Default::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label:          Some("Atlas Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter:     wgpu::FilterMode::Nearest,
            min_filter:     wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        // ─── バインドグループ group(0): テクスチャ ───────────────
        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label:   Some("Texture Bind Group Layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding:    0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty:         wgpu::BindingType::Texture {
                            multisampled:   false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type:    wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding:    1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty:         wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count:      None,
                    },
                ],
            });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label:   Some("Texture Bind Group"),
            layout:  &texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding:  0,
                    resource: wgpu::BindingResource::TextureView(&texture_view),
                },
                wgpu::BindGroupEntry {
                    binding:  1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        // ─── バインドグループ group(1): 画面サイズ Uniform ──────
        let screen_uniform = ScreenUniform::new(size.width, size.height);
        let screen_uniform_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label:    Some("Screen Uniform Buffer"),
            contents: bytemuck::bytes_of(&screen_uniform),
            usage:    wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let screen_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label:   Some("Screen Bind Group Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding:    0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty:         wgpu::BindingType::Buffer {
                        ty:                 wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size:   None,
                    },
                    count: None,
                }],
            });

        let screen_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label:   Some("Screen Bind Group"),
            layout:  &screen_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding:  0,
                resource: screen_uniform_buf.as_entire_binding(),
            }],
        });

        // ─── シェーダー・パイプライン ────────────────────────────
        let shader_source = include_str!("shaders/sprite.wgsl");
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label:  Some("Sprite Shader"),
            source: wgpu::ShaderSource::Wgsl(shader_source.into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label:                Some("Sprite Pipeline Layout"),
            bind_group_layouts:   &[&texture_bind_group_layout, &screen_bind_group_layout],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label:  Some("Sprite Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module:      &shader,
                entry_point: Some("vs_main"),
                buffers:     &[
                    wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
                        step_mode:    wgpu::VertexStepMode::Vertex,
                        attributes:   &wgpu::vertex_attr_array![0 => Float32x2],
                    },
                    wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<SpriteInstance>() as wgpu::BufferAddress,
                        step_mode:    wgpu::VertexStepMode::Instance,
                        attributes:   &wgpu::vertex_attr_array![
                            1 => Float32x2, // i_position
                            2 => Float32x2, // i_size
                            3 => Float32x2, // i_uv_offset
                            4 => Float32x2, // i_uv_size
                            5 => Float32x4, // i_color_tint
                        ],
                    },
                ],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module:      &shader,
                entry_point: Some("fs_main"),
                targets:     &[Some(wgpu::ColorTargetState {
                    format:     config.format,
                    blend:      Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology:           wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face:         wgpu::FrontFace::Ccw,
                cull_mode:          None,
                ..Default::default()
            },
            depth_stencil: None,
            multisample:   wgpu::MultisampleState::default(),
            multiview:     None,
            cache:         None,
        });

        // ─── 頂点・インデックスバッファ ──────────────────────────
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label:    Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(VERTICES),
            usage:    wgpu::BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label:    Some("Index Buffer"),
            contents: bytemuck::cast_slice(INDICES),
            usage:    wgpu::BufferUsages::INDEX,
        });

        // ─── インスタンスバッファ（動的・最大 MAX_INSTANCES 体）──
        let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label:              Some("Instance Buffer"),
            size:               (std::mem::size_of::<SpriteInstance>() * MAX_INSTANCES) as u64,
            usage:              wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // ─── egui 初期化 ─────────────────────────────────────────
        let egui_ctx = egui::Context::default();
        let egui_renderer = egui_wgpu::Renderer::new(&device, config.format, None, 1, false);
        let egui_winit = egui_winit::State::new(
            egui_ctx.clone(),
            egui::ViewportId::ROOT,
            &window,
            None,
            None,
            None,
        );

        Self {
            surface,
            device,
            queue,
            config,
            render_pipeline,
            vertex_buffer,
            index_buffer,
            instance_buffer,
            instance_count: 0,
            bind_group,
            screen_uniform_buf,
            screen_bind_group,
            egui_ctx,
            egui_renderer,
            egui_winit,
            frame_count: 0,
            fps_timer: std::time::Instant::now(),
            current_fps: 0.0,
        }
    }

    /// winit のウィンドウイベントを egui に転送する
    pub fn handle_window_event(
        &mut self,
        window: &Window,
        event: &winit::event::WindowEvent,
    ) -> bool {
        self.egui_winit.on_window_event(window, event).consumed
    }

    /// ゲーム状態からインスタンスリストを構築して GPU バッファを更新する
    /// render_data: [(x, y, kind)] kind: 0=player, 1=slime, 2=bat, 3=golem, 4=bullet
    /// particle_data: [(x, y, r, g, b, alpha, size)]
    /// item_data: [(x, y, kind)] kind: 5=gem, 6=potion, 7=magnet
    pub fn update_instances(
        &mut self,
        render_data: &[(f32, f32, u8)],
        particle_data: &[(f32, f32, f32, f32, f32, f32, f32)],
        item_data: &[(f32, f32, u8)],
    ) {
        let (player_uv_off, player_uv_sz)     = player_uv();
        let (bullet_uv_off, bullet_uv_sz)     = bullet_uv();
        let (particle_uv_off, particle_uv_sz) = particle_uv();
        let (gem_uv_off, gem_uv_sz)           = gem_uv();
        let (potion_uv_off, potion_uv_sz)     = potion_uv();
        let (magnet_uv_off, magnet_uv_sz)     = magnet_uv();

        let mut instances: Vec<SpriteInstance> =
            Vec::with_capacity(render_data.len() + particle_data.len() + item_data.len());

        for &(x, y, kind) in render_data {
            let inst = match kind {
                0 => SpriteInstance {
                    position:   [x, y],
                    size:       [SPRITE_SIZE, SPRITE_SIZE],
                    uv_offset:  player_uv_off,
                    uv_size:    player_uv_sz,
                    color_tint: [1.0, 1.0, 1.0, 1.0],
                },
                // 敵タイプ: 1=slime, 2=bat, 3=golem
                1 | 2 | 3 => {
                    let sz = enemy_sprite_size(kind);
                    let (uv_off, uv_sz) = enemy_uv_for_kind(kind);
                    SpriteInstance {
                        position:   [x, y],
                        size:       [sz, sz],
                        uv_offset:  uv_off,
                        uv_size:    uv_sz,
                        color_tint: [1.0, 1.0, 1.0, 1.0],
                    }
                }
                4 => SpriteInstance {
                    position:   [x - 8.0, y - 8.0],
                    size:       [16.0, 16.0],
                    uv_offset:  bullet_uv_off,
                    uv_size:    bullet_uv_sz,
                    color_tint: [1.0, 1.0, 1.0, 1.0],
                },
                _ => continue,
            };
            instances.push(inst);
            if instances.len() >= MAX_INSTANCES {
                break;
            }
        }

        // パーティクルを描画（スプライトサイズはパーティクルの size に合わせる）
        for &(x, y, r, g, b, alpha, size) in particle_data {
            if instances.len() >= MAX_INSTANCES { break; }
            instances.push(SpriteInstance {
                position:   [x - size / 2.0, y - size / 2.0],
                size:       [size, size],
                uv_offset:  particle_uv_off,
                uv_size:    particle_uv_sz,
                color_tint: [r, g, b, alpha],
            });
        }

        // Step 19: アイテムを描画
        for &(x, y, kind) in item_data {
            if instances.len() >= MAX_INSTANCES { break; }
            let (uv_off, uv_sz, sz) = match kind {
                RENDER_KIND_GEM    => (gem_uv_off,    gem_uv_sz,    20.0_f32),
                RENDER_KIND_POTION => (potion_uv_off, potion_uv_sz, 24.0_f32),
                RENDER_KIND_MAGNET => (magnet_uv_off, magnet_uv_sz, 28.0_f32),
                _ => continue,
            };
            instances.push(SpriteInstance {
                position:   [x - sz / 2.0, y - sz / 2.0],
                size:       [sz, sz],
                uv_offset:  uv_off,
                uv_size:    uv_sz,
                color_tint: [1.0, 1.0, 1.0, 1.0],
            });
        }

        self.instance_count = instances.len() as u32;

        if !instances.is_empty() {
            self.queue.write_buffer(
                &self.instance_buffer,
                0,
                bytemuck::cast_slice(&instances),
            );
        }
    }

    pub fn resize(&mut self, new_width: u32, new_height: u32) {
        if new_width == 0 || new_height == 0 {
            return;
        }
        self.config.width  = new_width;
        self.config.height = new_height;
        self.surface.configure(&self.device, &self.config);

        let screen_uniform = ScreenUniform::new(new_width, new_height);
        self.queue.write_buffer(
            &self.screen_uniform_buf,
            0,
            bytemuck::bytes_of(&screen_uniform),
        );
    }

    /// HUD を描画し、レベルアップ画面でボタンが押された場合は選択された武器名を返す
    pub fn render(&mut self, window: &Window, hud: &HudData) -> Option<String> {
        // ─── FPS 計測 ────────────────────────────────────────────
        self.frame_count += 1;
        let elapsed = self.fps_timer.elapsed();
        if elapsed.as_secs_f32() >= 1.0 {
            self.current_fps = self.frame_count as f32 / elapsed.as_secs_f32();
            self.frame_count = 0;
            self.fps_timer   = std::time::Instant::now();
        }

        // ─── サーフェス取得 ──────────────────────────────────────
        let output = match self.surface.get_current_texture() {
            Ok(t) => t,
            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                self.surface.configure(&self.device, &self.config);
                return None;
            }
            Err(e) => {
                eprintln!("Surface error: {e:?}");
                return None;
            }
        };

        let view = output.texture.create_view(&Default::default());
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        // ─── スプライト描画パス ──────────────────────────────────
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Sprite Pass"),
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

            if self.instance_count > 0 {
                pass.set_pipeline(&self.render_pipeline);
                pass.set_bind_group(0, &self.bind_group, &[]);
                pass.set_bind_group(1, &self.screen_bind_group, &[]);
                pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
                pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
                pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
                pass.draw_indexed(0..INDICES.len() as u32, 0, 0..self.instance_count);
            }
        }

        // ─── egui HUD パス ───────────────────────────────────────
        let raw_input = self.egui_winit.take_egui_input(window);
        let mut chosen_weapon: Option<String> = None;
        let full_output = self.egui_ctx.run(raw_input, |ctx| {
            chosen_weapon = build_hud_ui(ctx, hud, self.current_fps);
        });

        self.egui_winit.handle_platform_output(window, full_output.platform_output);

        let tris = self.egui_ctx.tessellate(full_output.shapes, full_output.pixels_per_point);

        for (id, delta) in full_output.textures_delta.set {
            self.egui_renderer.update_texture(&self.device, &self.queue, id, &delta);
        }

        let screen_desc = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [self.config.width, self.config.height],
            pixels_per_point: full_output.pixels_per_point,
        };
        self.egui_renderer.update_buffers(&self.device, &self.queue, &mut encoder, &tris, &screen_desc);

        // egui_renderer.render() は RenderPass を消費するため、別スコープで処理する
        {
            let render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("egui Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view:           &view,
                    resolve_target: None,
                    ops:            wgpu::Operations {
                        load:  wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes:         None,
                occlusion_query_set:      None,
            });
            // egui-wgpu 0.31 では render() が RenderPass を所有する形に変更された
            self.egui_renderer.render(
                &mut render_pass.forget_lifetime(),
                &tris,
                &screen_desc,
            );
        }

        for id in full_output.textures_delta.free {
            self.egui_renderer.free_texture(&id);
        }

        self.queue.submit([encoder.finish()]);
        output.present();

        chosen_weapon
    }
}

// ─── HUD UI 構築 ───────────────────────────────────────────────

/// HUD を描画し、レベルアップ画面でボタンが押された場合は選択された武器名を返す
fn build_hud_ui(ctx: &egui::Context, hud: &HudData, fps: f32) -> Option<String> {
    let mut chosen: Option<String> = None;
    // 上部 HUD バー
    egui::Area::new(egui::Id::new("hud_top"))
        .anchor(egui::Align2::LEFT_TOP, egui::vec2(8.0, 8.0))
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            egui::Frame::new()
                .fill(egui::Color32::from_rgba_unmultiplied(0, 0, 0, 180))
                .inner_margin(egui::Margin::symmetric(12, 8))
                .corner_radius(6.0)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        // HP バー
                        let hp_ratio = if hud.max_hp > 0.0 {
                            (hud.hp / hud.max_hp).clamp(0.0, 1.0)
                        } else {
                            0.0
                        };
                        ui.label(
                            egui::RichText::new("HP")
                                .color(egui::Color32::from_rgb(255, 100, 100))
                                .strong(),
                        );
                        let (rect, _) = ui.allocate_exact_size(
                            egui::vec2(160.0, 18.0),
                            egui::Sense::hover(),
                        );
                        let painter = ui.painter();
                        painter.rect_filled(rect, 4.0, egui::Color32::from_rgb(60, 20, 20));
                        let fill_w = rect.width() * hp_ratio;
                        let fill_rect = egui::Rect::from_min_size(
                            rect.min,
                            egui::vec2(fill_w, rect.height()),
                        );
                        let hp_color = if hp_ratio > 0.5 {
                            egui::Color32::from_rgb(80, 220, 80)
                        } else if hp_ratio > 0.25 {
                            egui::Color32::from_rgb(220, 180, 0)
                        } else {
                            egui::Color32::from_rgb(220, 60, 60)
                        };
                        painter.rect_filled(fill_rect, 4.0, hp_color);
                        ui.label(
                            egui::RichText::new(format!("{:.0}/{:.0}", hud.hp, hud.max_hp))
                                .color(egui::Color32::WHITE),
                        );

                        ui.separator();

                        // EXP バー
                        let exp_total = hud.exp + hud.exp_to_next;
                        let exp_ratio = if exp_total > 0 {
                            (hud.exp as f32 / exp_total as f32).clamp(0.0, 1.0)
                        } else {
                            0.0
                        };
                        ui.label(
                            egui::RichText::new(format!("Lv.{}", hud.level))
                                .color(egui::Color32::from_rgb(255, 220, 50))
                                .strong(),
                        );
                        let (exp_rect, _) = ui.allocate_exact_size(
                            egui::vec2(100.0, 18.0),
                            egui::Sense::hover(),
                        );
                        let painter = ui.painter();
                        painter.rect_filled(exp_rect, 4.0, egui::Color32::from_rgb(20, 20, 60));
                        let exp_fill = egui::Rect::from_min_size(
                            exp_rect.min,
                            egui::vec2(exp_rect.width() * exp_ratio, exp_rect.height()),
                        );
                        painter.rect_filled(exp_fill, 4.0, egui::Color32::from_rgb(80, 120, 255));
                        ui.label(
                            egui::RichText::new(format!("EXP {}", hud.exp))
                                .color(egui::Color32::from_rgb(180, 200, 255)),
                        );

                        ui.separator();

                        // スコア・タイマー
                        let total_s = hud.elapsed_seconds as u32;
                        let m = total_s / 60;
                        let s = total_s % 60;
                        ui.label(
                            egui::RichText::new(format!("Score: {}", hud.score))
                                .color(egui::Color32::from_rgb(255, 220, 100))
                                .strong(),
                        );
                        ui.label(
                            egui::RichText::new(format!("{:02}:{:02}", m, s))
                                .color(egui::Color32::WHITE),
                        );

                        // Step 17: weapon slot display
                        if !hud.weapon_levels.is_empty() {
                            ui.separator();
                            for (name, lv) in &hud.weapon_levels {
                                ui.label(
                                    egui::RichText::new(format!("[{}] Lv.{lv}", weapon_short_name(name)))
                                        .color(egui::Color32::from_rgb(180, 230, 255))
                                        .strong(),
                                );
                            }
                        }
                    });
                });
        });

    // 右上: デバッグ情報
    egui::Area::new(egui::Id::new("hud_debug"))
        .anchor(egui::Align2::RIGHT_TOP, egui::vec2(-8.0, 8.0))
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            egui::Frame::new()
                .fill(egui::Color32::from_rgba_unmultiplied(0, 0, 0, 140))
                .inner_margin(egui::Margin::symmetric(8, 6))
                .corner_radius(6.0)
                .show(ui, |ui| {
                    ui.label(
                        egui::RichText::new(format!("FPS: {fps:.0}"))
                            .color(egui::Color32::from_rgb(100, 255, 100)),
                    );
                    ui.label(
                        egui::RichText::new(format!("Enemies: {}", hud.enemy_count))
                            .color(egui::Color32::from_rgb(255, 150, 100)),
                    );
                    ui.label(
                        egui::RichText::new(format!("Bullets: {}", hud.bullet_count))
                            .color(egui::Color32::from_rgb(200, 200, 255)),
                    );
                    ui.label(
                        egui::RichText::new(format!("Items: {}", hud.item_count))
                            .color(egui::Color32::from_rgb(150, 230, 150)),
                    );
                    if hud.magnet_timer > 0.0 {
                        ui.label(
                            egui::RichText::new(format!("MAGNET {:.1}s", hud.magnet_timer))
                                .color(egui::Color32::from_rgb(255, 230, 50))
                                .strong(),
                        );
                    }
                });
        });

    // レベルアップ画面
    if hud.level_up_pending {
        egui::Area::new(egui::Id::new("level_up"))
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                egui::Frame::new()
                    .fill(egui::Color32::from_rgba_unmultiplied(10, 10, 40, 240))
                    .inner_margin(egui::Margin::symmetric(40, 30))
                    .corner_radius(12.0)
                    .stroke(egui::Stroke::new(2.0, egui::Color32::from_rgb(255, 220, 50)))
                    .show(ui, |ui| {
                        ui.vertical_centered(|ui| {
                            ui.label(
                                egui::RichText::new(format!("*** LEVEL UP!  Lv.{} ***", hud.level))
                                    .color(egui::Color32::from_rgb(255, 220, 50))
                                    .size(28.0)
                                    .strong(),
                            );
                            ui.add_space(8.0);

                            let result = if hud.weapon_choices.is_empty() {
                                build_max_level_ui(ui)
                            } else {
                                build_weapon_choice_ui(ui, hud)
                            };
                            if result.is_some() {
                                chosen = result;
                            }
                        });
                    });
            });
    }

    chosen
}

/// 全武器がMaxLvの場合のUI（「Continue [Esc]」ボタンのみ）
fn build_max_level_ui(ui: &mut egui::Ui) -> Option<String> {
    ui.label(
        egui::RichText::new("All weapons are at MAX level!")
            .color(egui::Color32::from_rgb(255, 180, 50))
            .size(16.0)
            .strong(),
    );
    ui.add_space(16.0);
    let btn = egui::Button::new(
        egui::RichText::new("Continue  [Esc]")
            .size(16.0)
            .strong(),
    )
    .fill(egui::Color32::from_rgb(80, 80, 80))
    .min_size(egui::vec2(160.0, 36.0));
    if ui.add(btn).clicked() {
        Some("__skip__".to_string())
    } else {
        None
    }
}

/// 武器選択肢がある場合のUI（武器カード × N + 「Skip [Esc]」ボタン）
fn build_weapon_choice_ui(ui: &mut egui::Ui, hud: &HudData) -> Option<String> {
    let mut chosen: Option<String> = None;

    ui.label(
        egui::RichText::new("Choose a weapon")
            .color(egui::Color32::WHITE)
            .size(16.0),
    );
    ui.add_space(16.0);

    ui.horizontal(|ui| {
        for choice in &hud.weapon_choices {
            let current_lv = hud.weapon_levels
                .iter()
                .find(|(n, _)| n == choice)
                .map(|(_, lv)| *lv)
                .unwrap_or(0);
            if build_weapon_card(ui, choice, current_lv).is_some() {
                chosen = Some(choice.clone());
            }
            ui.add_space(12.0);
        }
    });

    ui.add_space(12.0);
    let skip_btn = egui::Button::new(
        egui::RichText::new("Skip  [Esc]").size(12.0),
    )
    .fill(egui::Color32::from_rgba_unmultiplied(60, 60, 60, 200))
    .min_size(egui::vec2(90.0, 24.0));
    if ui.add(skip_btn).clicked() {
        chosen = Some("__skip__".to_string());
    }

    chosen
}

/// 武器1枚分のカードUIを描画し、選択されたら `Some(())` を返す
fn build_weapon_card(ui: &mut egui::Ui, choice: &str, current_lv: u32) -> Option<()> {
    let is_upgrade  = current_lv > 0;
    let next_lv     = current_lv + 1;

    let border_color = if is_upgrade {
        egui::Color32::from_rgb(255, 180, 50)   // 強化: 金色
    } else {
        egui::Color32::from_rgb(100, 180, 255)  // 新規: 青色
    };
    let bg_color = if is_upgrade {
        egui::Color32::from_rgb(50, 35, 10)
    } else {
        egui::Color32::from_rgb(15, 30, 60)
    };

    let frame = egui::Frame::new()
        .fill(bg_color)
        .inner_margin(egui::Margin::symmetric(16, 14))
        .corner_radius(10.0)
        .stroke(egui::Stroke::new(2.0, border_color));

    let response = frame.show(ui, |ui| {
        ui.set_min_width(140.0);
        ui.vertical_centered(|ui| {
            ui.label(
                egui::RichText::new(weapon_short_name(choice))
                    .color(egui::Color32::from_rgb(220, 230, 255))
                    .size(16.0)
                    .strong(),
            );
            ui.add_space(4.0);

            let lv_text = if is_upgrade {
                format!("Lv.{current_lv} -> Lv.{next_lv}")
            } else {
                "NEW!".to_string()
            };
            let lv_color = if is_upgrade {
                egui::Color32::from_rgb(255, 200, 80)
            } else {
                egui::Color32::from_rgb(100, 255, 150)
            };
            ui.label(
                egui::RichText::new(lv_text)
                    .color(lv_color)
                    .size(13.0)
                    .strong(),
            );
            ui.add_space(6.0);

            for line in weapon_upgrade_desc(choice, current_lv) {
                ui.label(
                    egui::RichText::new(line)
                        .color(egui::Color32::from_rgb(180, 200, 180))
                        .size(11.0),
                );
            }
            ui.add_space(8.0);

            let btn = egui::Button::new(
                egui::RichText::new("Select  [1/2/3]")
                    .size(13.0)
                    .strong(),
            )
            .fill(border_color)
            .min_size(egui::vec2(110.0, 28.0));
            ui.add(btn)
        }).inner
    });

    if response.inner.clicked() { Some(()) } else { None }
}

fn weapon_short_name(name: &str) -> &str {
    match name {
        "magic_wand" => "Magic Wand",
        "axe"        => "Axe",
        "cross"      => "Cross",
        _            => name,
    }
}

/// Returns upgrade description lines for the level-up card
fn weapon_upgrade_desc(name: &str, current_lv: u32) -> Vec<String> {
    let next = current_lv + 1;
    match name {
        "magic_wand" => {
            let mut lines = vec![
                format!("DMG: {} -> {}", magic_wand_dmg(current_lv), magic_wand_dmg(next)),
                format!("CD:  {:.1}s -> {:.1}s", magic_wand_cd(current_lv), magic_wand_cd(next)),
            ];
            let bullets_now  = magic_wand_bullets(current_lv);
            let bullets_next = magic_wand_bullets(next);
            if bullets_next > bullets_now {
                lines.push(format!("Shots: {} -> {} (+)", bullets_now, bullets_next));
            } else {
                lines.push(format!("Shots: {}", bullets_now));
            }
            lines
        }
        "axe" => vec![
            format!("DMG: {} -> {}", axe_dmg(current_lv), axe_dmg(next)),
            format!("CD:  {:.1}s -> {:.1}s", axe_cd(current_lv), axe_cd(next)),
            "Throws upward".to_string(),
        ],
        "cross" => {
            let dirs_now  = if current_lv == 0 || current_lv <= 3 { 4 } else { 8 };
            let dirs_next = if next <= 3 { 4 } else { 8 };
            let mut lines = vec![
                format!("DMG: {} -> {}", cross_dmg(current_lv), cross_dmg(next)),
                format!("CD:  {:.1}s -> {:.1}s", cross_cd(current_lv), cross_cd(next)),
            ];
            if dirs_next > dirs_now {
                lines.push(format!("Dirs: {} -> {} (+)", dirs_now, dirs_next));
            } else {
                lines.push(format!("{}-way fire", dirs_now));
            }
            lines
        }
        _ => vec!["Upgrade weapon".to_string()],
    }
}

fn magic_wand_dmg(lv: u32) -> i32 { let b = 10i32; b + (lv as i32).saturating_sub(1) * (b / 4).max(1) }
fn magic_wand_cd(lv: u32) -> f32  { let b = 0.8f32; (b * (1.0 - (lv as f32 - 1.0).max(0.0) * 0.07)).max(b * 0.5) }
fn magic_wand_bullets(lv: u32) -> u32 { match lv { 0..=2 => 1, 3..=4 => 2, 5..=6 => 3, _ => 4 } }

fn axe_dmg(lv: u32) -> i32 { let b = 25i32; b + (lv as i32).saturating_sub(1) * (b / 4).max(1) }
fn axe_cd(lv: u32) -> f32  { let b = 1.5f32; (b * (1.0 - (lv as f32 - 1.0).max(0.0) * 0.07)).max(b * 0.5) }

fn cross_dmg(lv: u32) -> i32 { let b = 15i32; b + (lv as i32).saturating_sub(1) * (b / 4).max(1) }
fn cross_cd(lv: u32) -> f32  { let b = 2.0f32; (b * (1.0 - (lv as f32 - 1.0).max(0.0) * 0.07)).max(b * 0.5) }
