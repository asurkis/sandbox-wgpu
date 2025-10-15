struct PrimitiveRect {
    @location(0) pos: vec4<f32>,
    @location(1) color: vec4<f32>,
}

struct VertexOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) color: vec4<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) in_vertex_index: u32, in_rect: PrimitiveRect) -> VertexOut {
    var out: VertexOut;
    let xy_i = vec2<i32>(i32(in_vertex_index & 1), i32(in_vertex_index >> 1));
    out.pos.x = in_rect.pos[xy_i.x << 1];
    out.pos.y = in_rect.pos[(xy_i.y << 1) | 1];
    out.pos.z = 0.0;
    out.pos.w = 1.0;
    out.color = in_rect.color;
    return out;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    return in.color;
}
