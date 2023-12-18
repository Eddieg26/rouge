
struct VertexOutput {
@builtin(position) position: vec4<f32>;
@location(0) normal: vec3<f32>;
@location(1) uv: vec2<f32>;
};


struct Standard {
 color: vec4<f32>;
 opacity: f32;
};

struct Material {
 color: vec4<f32>;
};


@group(1) @binding(0)
var material: Material;

@group(1) @binding(0)
var tex: texture_2d<f32>;
            
                

                @fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let standard: Standard;

    standard.color = material.color;


    return vec4<f32>(standard.color.xyz, 1.0);
}

            