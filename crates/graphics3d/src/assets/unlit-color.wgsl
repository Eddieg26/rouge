struct FragmentInput {
    @builtin(position) position: vec4<f32>;
}

struct FragmentOutput {
    @location(0) color: vec4<f32>;
}

@group(2) @binding(0) var<uniform> color: vec4<f32>;

@fragment
fn main(input: FragmentInput) -> FragmentOutput {
    var output: FragmentOutput;
    output.color = color;
    return output;
}