
                struct Camera {
                    view : mat4x4<f32>;
                    projection : mat4x4<f32>;
                }

                struct Globals {
                    camera: Camera;
                }

                struct Object {
                    model: mat4x4<f32>;
                    normal: mat3x3<f32>;
                }

                struct VertexInput {
                    @location(0) position: vec4<f32>;
                    @location(1) normal: vec3<f32>;
                    @location(2) uv: vec2<f32>;
                }

                struct VertexOutput {
                    @builtin(position) position: vec4<f32>;
                    @location(0) normal: vec3<f32>;
                    @location(1) uv: vec2<f32>;
                }

                @group(0) @binding(0)
                var<uniform> globals : Globals;

                @group(1) @binding(0)
                var<uniform> object : Object;

                @vertex
                fn vs_main(input: VertexInput) -> VertexOutput {
                    var output: VertexOutput;

                    output.position = globals.camera.view * globals.camera.proj * object.model * input.position;
                    output.normal = input.normal;
                    output.uv = input.uv;

                    return output;
                }
            
                    struct ShaderInputs {
                         color : vec4<f32>;

                    }

                    @group(2) @binding(0)
                    var<uniform> shader_inputs : ShaderInputs;
                @location(0) var out_color : vec4<f32>;

                    fn fs_main(input: VertexOutput) {
                        var color = shader_inputs.color;
out_color = color;

                    }
                