
struct VertexOutput {
@builtin(position) position: vec4<f32>;
@location(0) normal: vec3<f32>;
@location(1) uv: vec2<f32>;
};


struct Standard {
 color: vec4<f32>;
 opacity: f32;
};

                

                
                @group(1) @binding(0)
                var color_texture: texture_2d<f32>;
            @group(1) @binding(1)
                var opacity_texture: texture_2d<f32>;
            @group(1) @binding(2)
                var color_sampler: sampler;
            @group(1) @binding(3)
                var opacity_sampler: sampler;

                @fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let standard: Standard;

    standard.color = textureSample(color_texture, color_sampler, in.uv.xy);
    standard.opacity = (textureSample(opacity_texture, opacity_sampler, in.uv.xy)).x;



    return vec4<f32>(standard.color.xyz, standard.opacity);
}

            