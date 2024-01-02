struct VertexInput {
    @location(0) position: vec4<f32>,
    @location(1) color: vec4<f32>,
    @location(2) uv: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec3<f32>,
    @location(0) color: vec3<f32>,
    @location(1) uv: vec2<f32>,
};

struct Globals {
    view: mat4x4<f32>,
    projection: mat4x4<f32>,
};

struct Object {
    model: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> globals: Globals;

@group(2) @binding(0)
var<uniform> object: Object;
            
@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.position = globals.projection * globals.view * object.model * vec4<f32>(input.position.xyz, 1.0);
    out.color = vec4<f32>(input.color.xyz, 1.0);
    out.uv = input.uv;
    return out;
}