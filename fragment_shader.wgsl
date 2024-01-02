struct VertexOutput {
    @builtin(position) position: vec3<f32>,
    @location(0) color: vec3<f32>,
    @location(1) uv: vec2<f32>,
};

@group(1) @binding(0)
var<uniform> mat_color: vec4<f32>;

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(mat_color.xyz, 1.0);
}