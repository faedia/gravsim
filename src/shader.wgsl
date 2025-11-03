struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;
    let x = f32(1 - i32(vertex_index)) * 0.5;
    let y = f32(i32(vertex_index & 1) * 2 - 1) * 0.5;
    out.clip_position = vec4<f32>(x, y, 0.0, 1.0);
    if vertex_index == 0 {
        out.color = vec3<f32>(1.0, 0.0, 0.0);
    } else if vertex_index == 1 {
        out.color = vec3<f32>(0.0, 1.0, 0.0);
    } else {
        out.color = vec3<f32>(0.0, 0.0, 1.0);
    }
    return out;
}

@fragment
fn fs_main(@location(0) in_color: vec3<f32>) -> @location(0) vec4<f32> {
    return vec4<f32>(in_color, 1.0);
}