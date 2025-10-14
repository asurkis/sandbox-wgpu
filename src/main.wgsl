struct Rect {
    pos: vec4<f32>,
    color: vec4<f32>,
}

struct VertexOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) color: vec4<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) in_vertex_index: u32) -> VertexOut {
    var out: VertexOut;
    let xy_i = vec2<i32>(i32(in_vertex_index & 1), i32(in_vertex_index >> 1));
    let xy = vec2<f32>(xy_i);
    out.pos = vec4<f32>(xy - 0.5, 0.0, 1.0);
    out.color = vec4<f32>(xy, 0.0, 1.0);
    return out;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    return in.color;
}
