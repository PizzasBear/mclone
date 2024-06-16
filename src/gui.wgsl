struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) tex_coords: vec2<f32>,
    @location(2) color: vec4<f32>,
};

struct InstanceInput {
    @location(3) model_mat1: vec2<f32>,
    @location(4) model_mat2: vec2<f32>,
    @location(5) offset: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) color: vec4<f32>,
};

struct GuiUniform {
    resolution: vec2<f32>,
};

@group(1) @binding(0)
var<uniform> gui_uniform: GuiUniform;

@vertex
fn vs_main(
    model: VertexInput,
    instance: InstanceInput,
) -> VertexOutput {
    let model_mat = mat2x2<f32>(
        instance.model_mat1,
        instance.model_mat2,
    );

    let adj_position = model.position / gui_uniform.resolution;

    var out: VertexOutput;
    out.clip_position = vec4(instance.offset + vec3(model_mat * adj_position, 0.0), 1.0);
    out.tex_coords = model.tex_coords;
    out.color = model.color;

    return out;
}

// Fragment shader
@group(0) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(0) @binding(1)
var s_diffuse: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color * textureSample(t_diffuse, s_diffuse, in.tex_coords);
}
