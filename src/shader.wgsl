struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
    @location(2) normal: vec3<f32>,
    @location(3) tangent: vec3<f32>,
};

struct InstanceInput {
    @location(5) model_mat0: vec4<f32>,
    @location(6) model_mat1: vec4<f32>,
    @location(7) model_mat2: vec4<f32>,
    @location(8) model_mat3: vec4<f32>,

    @location(9) normal_mat0: vec3<f32>,
    @location(10) normal_mat1: vec3<f32>,
    @location(11) normal_mat2: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) position: vec3<f32>,
    @location(2) normal: vec3<f32>,
    @location(3) tangent: vec3<f32>,
};

struct CameraUniform {
    view_proj: mat4x4<f32>,
    view_position: vec3<f32>,
};
@group(1) @binding(0)
var<uniform> camera: CameraUniform;

struct LightUniform {
    dir: vec3<f32>,
    color: vec3<f32>,
};

@group(2) @binding(0)
var<uniform> light: LightUniform;

@vertex
fn vs_main(
    model: VertexInput,
    instance: InstanceInput,
) -> VertexOutput {
    let model_mat = mat4x4<f32>(
        instance.model_mat0,
        instance.model_mat1,
        instance.model_mat2,
        instance.model_mat3,
    );
    let normal_mat = mat3x3<f32>(
        instance.normal_mat0,
        instance.normal_mat1,
        instance.normal_mat2,
    );

    let world_position = model_mat * vec4<f32>(model.position, 1.0);

    var out: VertexOutput;
    out.tex_coords = model.tex_coords;
    out.clip_position = camera.view_proj * world_position;
    out.position = world_position.xyz / world_position.w;
    out.normal = normal_mat * model.normal;

    out.tangent = normalize(normal_mat * model.tangent);
    out.tangent = normalize(out.tangent - dot(out.tangent, out.normal) * out.normal);

    return out;
}

// Fragment shader
@group(0) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(0) @binding(1)
var s_diffuse: sampler;

@group(0) @binding(2)
var t_normal: texture_2d<f32>;
@group(0) @binding(3)
var s_normal: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let object_color: vec4<f32> = textureSample(t_diffuse, s_diffuse, in.tex_coords);

    let ambient_strength = 0.02;
    let ambient_color = ambient_strength * light.color;

    let tangent_normal = normalize(2 * textureSample(t_normal, s_normal, in.tex_coords).xyz - 1);
    let normal = normalize(in.normal);
    let tangent = normalize(in.tangent);
    let bitangent = cross(normal, tangent);
    let TBN = transpose(mat3x3<f32>(tangent, bitangent, normal));

    let light_dir = TBN * -light.dir;
    let view_dir = TBN * normalize(camera.view_position - in.position);
    let half_dir = normalize(light_dir + view_dir);

    let diffuse_stength = max(dot(tangent_normal, light_dir), 0.0);
    let diffuse_color = diffuse_stength * light.color;

    let specular_strength = pow(max(dot(tangent_normal, half_dir), 0.0), 32.0);
    let specular_color = specular_strength * light.color;

    let result = (ambient_color + diffuse_color + specular_color) * object_color.rgb;
    return vec4<f32>(result, object_color.a);
}
