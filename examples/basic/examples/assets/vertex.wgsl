struct VertexInput {
    @location(0) position: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
}

struct Globals {
    frame: u32,
    time: f32,
    delta_time: f32,
    _padding: u32,
}

@group(0) @binding(0) var<uniform> globals: Globals;
@group(1) @binding(0) var<uniform> object: mat4x4<f32>;

@vertex
fn main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    let time = globals.time;
    output.position = object * vec4<f32>(input.position, 1.0);
    return output;
}