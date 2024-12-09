struct FragmentInput {
    @builtin(position) position: vec4<f32>;
    @location(0) uv: vec2<f32>;
}

struct FragmentOutput {
    @location(0) color: vec4<f32>;
}

@group(2) @binding(0) var texture: texture_2d<f32>;
@group(2) @binding(1) var texture_sampler: sampler;

@fragment
fn main(input: FragmentInput) -> FragmentOutput {
    var output: FragmentOutput;
    output.color = textureSample(texture, texture_sampler, input.uv);
    return output;
}