@group(0) @binding(0) var sprite_texture: texture_2d<f32>;
@group(0) @binding(1) var sprite_sampler: sampler;

struct VertexInput {
    @location(0) position: vec2<f32>,
};

// インスタンスごとのデータ（@location(1)〜(5)）
struct InstanceInput {
    @location(1) i_position:  vec2<f32>,
    @location(2) i_size:      vec2<f32>,
    @location(3) i_uv_offset: vec2<f32>,
    @location(4) i_uv_size:   vec2<f32>,
    @location(5) i_color_tint: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color_tint: vec4<f32>,
};

// 画面サイズの半分（クリップ空間変換用）
const HALF_W: f32 = 640.0;
const HALF_H: f32 = 360.0;

@vertex
fn vs_main(in: VertexInput, inst: InstanceInput) -> VertexOutput {
    var out: VertexOutput;

    // ワールド座標：インスタンスの左上 + 頂点オフセット（サイズ分）
    let world_pos = inst.i_position + in.position * inst.i_size;

    // クリップ座標（Y 軸は下が正なので反転）
    out.clip_position = vec4<f32>(
        (world_pos.x - HALF_W) / HALF_W,
        -(world_pos.y - HALF_H) / HALF_H,
        0.0,
        1.0,
    );

    // アトラス UV
    out.uv = inst.i_uv_offset + in.position * inst.i_uv_size;
    out.color_tint = inst.i_color_tint;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let tex_color = textureSample(sprite_texture, sprite_sampler, in.uv);
    return tex_color * in.color_tint;
}
