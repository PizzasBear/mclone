struct VertexInput {
    [[location(0)]] position: vec3<f32>;
    [[location(1)]] tex_coords: vec2<f32>;
    [[location(2)]] color: vec3<f32>;
};

struct InstanceInput {
    [[location(5)]] model_mat0: vec4<f32>;
    [[location(6)]] model_mat1: vec4<f32>;
    [[location(7)]] model_mat2: vec4<f32>;
    [[location(8)]] model_mat3: vec4<f32>;
};

struct VertexOutput {
    [[builtin(position)]] clip_position: vec4<f32>;
    [[location(0)]] tex_coords: vec2<f32>;
    [[location(1)]] color: vec3<f32>;
};

[[block]]
struct CameraUniform {
    model: mat4x4<f32>;
};

[[group(1), binding(0)]]
var<uniform> camera_uniform: CameraUniform;

[[stage(vertex)]]
fn vs_main(
    vertex_in: VertexInput,
    instance_in: InstanceInput,
) -> VertexOutput {
    let instance_mode_mat = mat4x4<f32>(
        instance_in.model_mat0,
        instance_in.model_mat1,
        instance_in.model_mat2,
        instance_in.model_mat3,
    );
    var out: VertexOutput;
    out.tex_coords = vertex_in.tex_coords;
    out.clip_position = camera_uniform.model * instance_mode_mat * vec4<f32>(vertex_in.position, 1.0);
    out.color = vertex_in.color;
    return out;
}

[[group(0), binding(0)]]
var t_diffuse: texture_2d<f32>;
[[group(0), binding(1)]]
var s_diffuse: sampler;

[[stage(fragment)]]
fn fs_main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
    // return vec4<f32>(in.color, 1.0);
    return vec4<f32>(in.color, 1.0) * textureSample(t_diffuse, s_diffuse, in.tex_coords);
}
