struct VertexOutput {
    @builtin(position) position: vec3<f32>,
    @location(0) color: vec3<f32>,
    @location(1) uv: vec2<f32>,
};

struct VertexInput {
    @location(0) position: vec4<f32>,
    @location(1) color: vec4<f32>,
    @location(2) uv: vec2<f32>,
};
            
@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.position = vec4<f32>(input.position.xyz, 1.0);
    out.color = vec4<f32>(input.color.xyz, 1.0);
    out.uv = input.uv;
    return out;
}