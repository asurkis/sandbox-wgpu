struct VertexIn {
    @builtin(vertex_index) idx: u32,
    @location(0) pos: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) tex_coord: vec2<f32>,
}

struct VertexOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) tex_coord: vec2<f32>,
}

@group(0) @binding(0) var prim_texture: texture_2d<f32>;
@group(0) @binding(1) var prim_sampler: sampler;

@vertex
fn vs_main(in: VertexIn) -> VertexOut {
    var out: VertexOut;
    out.pos = vec4(in.pos, 0.0, 1.0);
    out.color = in.color;
    out.tex_coord = in.tex_coord;
    return out;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    let tint = in.color;
    let sampled = textureSample(prim_texture, prim_sampler, in.tex_coord);
    return tint * sampled;
}
