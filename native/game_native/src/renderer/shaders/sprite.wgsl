@group(0) @binding(0) var sprite_texture: texture_2d<f32>;
@group(0) @binding(1) var sprite_sampler: sampler;

struct VertexInput {
    @location(0) position: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    // 画面中央に 64x64 px のスプライトを配置（1280x720 基準）
    let world_pos = in.position * vec2<f32>(64.0, 64.0) - vec2<f32>(32.0, 32.0);
    out.clip_position = vec4<f32>(world_pos / vec2<f32>(640.0, 360.0), 0.0, 1.0);
    out.uv = in.position;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(sprite_texture, sprite_sampler, in.uv);
}
