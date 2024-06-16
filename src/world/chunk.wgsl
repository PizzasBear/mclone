struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
    @location(2) color: vec4<f32>,
};

struct InstanceInput {
    @location(5) offset: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
    @location(2) color: vec4<f32>,
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
    let world_position = model.position + instance.offset;

    var out: VertexOutput;
    out.clip_position = camera.view_proj * vec4(world_position, 1.0);
    out.position = world_position;
    out.tex_coords = model.tex_coords;
    out.color = model.color;

    return out;
}

// Fragment shader
@group(0) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(0) @binding(1)
var s_diffuse: sampler;

// @group(0) @binding(2)
// var t_normal: texture_2d<f32>;
// @group(0) @binding(3)
// var s_normal: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let object_color = color_blend(textureSample(t_diffuse, s_diffuse, in.tex_coords), in.color);

    // let tangent_normal = normalize(2 * textureSample(t_normal, s_normal, in.tex_coords).xyz - 1);
    let tangent_normal = vec3(0.0, 0.0, 1.0);

    let dposdx = dpdx(in.position);
    let dposdy = dpdy(in.position);

    let tangent = normalize(dposdx);
    let normal = normalize(cross(dposdy, dposdx));
    let bitangent = cross(normal, tangent);

    let TBN = transpose(mat3x3(tangent, bitangent, normal));

    let light_dir = TBN * -light.dir;
    let view_dir = TBN * normalize(camera.view_position - in.position);
    let half_dir = normalize(light_dir + view_dir);

    let ambient_strength = 0.1;
    let diffuse_stength = max(dot(tangent_normal, light_dir), 0.0);
    let specular_strength = pow(max(dot(tangent_normal, half_dir), 0.0), 32.0);

    let result = min(ambient_strength + diffuse_stength + specular_strength, 1.0) * light.color * object_color.rgb;
    return vec4(result, object_color.a);
}

fn color_blend(base: vec4<f32>, blend: vec4<f32>) -> vec4<f32> {
    let base1 = rgb2hsv(base.rgb);
    let blend1 = rgb2hsv(blend.rgb);

    return vec4(hsv2rgb(vec3(
        mix(base1.r, blend1.r, blend.a),
        mix(base1.g, blend1.g, blend.a),
        base1.b,
        // mix(base1.b, blend1.b, blend.a),
    )), base.a);
}

// All components are in the range [0…1], including hue.
fn rgb2hsv(c: vec3<f32>) -> vec3<f32> {
    let K = vec4(0.0, -1.0 / 3.0, 2.0 / 3.0, -1.0);
    let p = mix(vec4(c.bg, K.wz), vec4(c.gb, K.xy), step(c.b, c.g));
    let q = mix(vec4(p.xyw, c.r), vec4(c.r, p.yzx), step(p.x, c.r));

    let d = q.x - min(q.w, q.y);
    let e = 1.0e-10;
    return vec3(abs(q.z + (q.w - q.y) / (6.0 * d + e)), d / (q.x + e), q.x);
}

fn hsv2rgb(c: vec3<f32>) -> vec3<f32> {
    let K = vec4(1.0, 2.0 / 3.0, 1.0 / 3.0, 3.0);
    let p = abs(fract(c.xxx + K.xyz) * 6.0 - K.www);
    return c.z * mix(K.xxx, clamp(p - K.xxx, vec3(0.0), vec3(1.0)), c.y);
}
