struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) tex_coord: vec2<f32>,
    @location(2) color: u32,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coord: vec2<f32>,
    @location(1) color: vec4<f32>,
}

struct Uniforms {
    screen_size: vec2<f32>,
}

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

@vertex
fn vs_main(vertex: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    
    out.clip_position = vec4<f32>(
        vertex.position.x * 2.0 / uniforms.screen_size.x - 1.0,
        1.0 - vertex.position.y * 2.0 / uniforms.screen_size.y,
        0.0, 1.0
    );
    out.tex_coord = vertex.tex_coord;
    
    let r = f32((vertex.color >> 0u) & 0xFFu) / 255.0;
    let g = f32((vertex.color >> 8u) & 0xFFu) / 255.0;
    let b = f32((vertex.color >> 16u) & 0xFFu) / 255.0;
    let a = f32((vertex.color >> 24u) & 0xFFu) / 255.0;
    out.color = vec4<f32>(r, g, b, a);
    
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}
