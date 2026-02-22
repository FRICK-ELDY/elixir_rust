use std::sync::Arc;
use wgpu::util::DeviceExt;
use winit::window::Window;

// 四角形 1 枚の頂点（正規化座標 0.0〜1.0）
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

// インスタンスごとの GPU データ
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SpriteInstance {
    pub position:   [f32; 2], // ワールド座標（左上）
    pub size:       [f32; 2], // ピクセルサイズ
    pub uv_offset:  [f32; 2], // アトラス UV オフセット（0.0〜1.0）
    pub uv_size:    [f32; 2], // アトラス UV サイズ（0.0〜1.0）
    pub color_tint: [f32; 4], // RGBA 乗算カラー
}

// 画面サイズ Uniform（シェーダーの group(1) に対応）
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct ScreenUniform {
    half_size: [f32; 2], // (width / 2, height / 2)
    _pad: [f32; 2],      // wgpu の 16 バイトアライメント要件
}

impl ScreenUniform {
    fn new(width: u32, height: u32) -> Self {
        Self {
            half_size: [width as f32 / 2.0, height as f32 / 2.0],
            _pad: [0.0; 2],
        }
    }
}

pub const SPRITE_SIZE: f32 = 64.0;
const GRID_DIM: usize = 10;
const INSTANCE_COUNT: usize = GRID_DIM * GRID_DIM;

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
    bind_group:           wgpu::BindGroup,           // group(0): テクスチャ
    screen_uniform_buf:   wgpu::Buffer,              // group(1): 画面サイズ
    screen_bind_group:    wgpu::BindGroup,           // group(1) バインドグループ
    // FPS 計測
    frame_count:          u32,
    fps_timer:            std::time::Instant,
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

        // テクスチャアトラスの読み込み
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

        // group(0): テクスチャ・サンプラー バインドグループレイアウト
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

        // group(1): 画面サイズ Uniform バッファ
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

        // シェーダー
        let shader_source = include_str!("shaders/sprite.wgsl");
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label:  Some("Sprite Shader"),
            source: wgpu::ShaderSource::Wgsl(shader_source.into()),
        });

        // パイプライン（group(0): テクスチャ、group(1): 画面サイズ）
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
                    // slot 0: 頂点バッファ（Vertex ごとに進む）
                    wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
                        step_mode:    wgpu::VertexStepMode::Vertex,
                        attributes:   &wgpu::vertex_attr_array![0 => Float32x2],
                    },
                    // slot 1: インスタンスバッファ（Instance ごとに進む）
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

        // 頂点・インデックスバッファ
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

        // インスタンスバッファ（100体を格子状に配置）
        let instances: Vec<SpriteInstance> = (0..INSTANCE_COUNT)
            .map(|i| SpriteInstance {
                position:   [(i % GRID_DIM) as f32 * SPRITE_SIZE, (i / GRID_DIM) as f32 * SPRITE_SIZE],
                size:       [SPRITE_SIZE, SPRITE_SIZE],
                uv_offset:  [0.0, 0.0],
                uv_size:    [1.0, 1.0],
                color_tint: [1.0, 1.0, 1.0, 1.0],
            })
            .collect();

        let instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label:    Some("Instance Buffer"),
            contents: bytemuck::cast_slice(&instances),
            usage:    wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });

        Self {
            surface,
            device,
            queue,
            config,
            render_pipeline,
            vertex_buffer,
            index_buffer,
            instance_buffer,
            instance_count: INSTANCE_COUNT as u32,
            bind_group,
            screen_uniform_buf,
            screen_bind_group,
            frame_count: 0,
            fps_timer: std::time::Instant::now(),
        }
    }

    /// プレイヤーのインスタンス（index 0）の位置を更新する
    pub fn update_player(&mut self, x: f32, y: f32) {
        let instance = SpriteInstance {
            position:   [x, y],
            size:       [SPRITE_SIZE, SPRITE_SIZE],
            uv_offset:  [0.0, 0.0],
            uv_size:    [1.0, 1.0],
            color_tint: [0.2, 0.8, 1.0, 1.0], // 水色でプレイヤーを識別
        };
        self.queue.write_buffer(
            &self.instance_buffer,
            0,
            bytemuck::bytes_of(&instance),
        );
    }

    pub fn resize(&mut self, new_width: u32, new_height: u32) {
        if new_width == 0 || new_height == 0 {
            return;
        }
        self.config.width = new_width;
        self.config.height = new_height;
        self.surface.configure(&self.device, &self.config);

        // ウィンドウサイズ変更時に Uniform バッファを更新
        let screen_uniform = ScreenUniform::new(new_width, new_height);
        self.queue.write_buffer(
            &self.screen_uniform_buf,
            0,
            bytemuck::bytes_of(&screen_uniform),
        );
    }

    pub fn render(&mut self) {
        // FPS 計測（1 秒ごとにコンソール出力）
        self.frame_count += 1;
        let elapsed = self.fps_timer.elapsed();
        if elapsed.as_secs_f32() >= 1.0 {
            let fps = self.frame_count as f32 / elapsed.as_secs_f32();
            println!("FPS: {fps:.1}");
            self.frame_count = 0;
            self.fps_timer = std::time::Instant::now();
        }

        let output = match self.surface.get_current_texture() {
            Ok(t) => t,
            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                self.surface.configure(&self.device, &self.config);
                return;
            }
            Err(e) => {
                eprintln!("Surface error: {e:?}");
                return;
            }
        };

        let view = output.texture.create_view(&Default::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Sprite Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view:           &view,
                    resolve_target: None,
                    ops:            wgpu::Operations {
                        load:  wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.05,
                            g: 0.0,
                            b: 0.1,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes:         None,
                occlusion_query_set:      None,
            });

            pass.set_pipeline(&self.render_pipeline);
            pass.set_bind_group(0, &self.bind_group, &[]);
            pass.set_bind_group(1, &self.screen_bind_group, &[]);
            pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
            pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            // 1 draw call で 100 体を描画
            pass.draw_indexed(0..INDICES.len() as u32, 0, 0..self.instance_count);
        }

        self.queue.submit([encoder.finish()]);
        output.present();
    }
}
