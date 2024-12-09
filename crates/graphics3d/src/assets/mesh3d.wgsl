struct VertexInput {
    @location(0) position: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
}

struct Camera {
    position: vec4<f32>,
    world: mat4x4<f32>,
    view: mat4x4<f32>,
    projection: mat4x4<f32>,
    frustum: array<vec4<f32>, 6>,
}

@group(0) @binding(0) var<uniform> camera: Camera;
@group(1) @binding(0) var<uniform> model: mat4x4<f32>;


@vertex
fn main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.position = camera.projection * camera.view * model * vec4<f32>(input.position, 1.0);
    return output;
}