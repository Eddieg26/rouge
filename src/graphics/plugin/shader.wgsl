struct VertexOutput {
            @builtin(position) position: vec3<f32>,
            @location(0) color: vec3<f32>,
            @location(1) tex_coords: vec2<f32>,
        };

struct VertexInput {
            @location(0) position: vec4<f32>,
            @location(1) color: vec4<f32>,
            @location(2) tex_coords: vec2<f32>,
        }
            
        @vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.position = vec4<f32>(input.position.xyz, 1.0);
    out.color = vec4<f32>(input.color.xyz, 1.0);
    out.tex_coords = input.tex_coords;
    return out;
}

        @fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(input.color.xyz, 1.0);
}