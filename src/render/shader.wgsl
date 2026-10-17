// Mesh and Line Shader for Mesh 3D viewer

struct Uniforms {
    view_proj: mat4x4<f32>,
    camera_pos: vec3<f32>,
    _pad0: f32,
    light_dir: vec3<f32>,
    _pad1: f32,
    object_color: vec4<f32>,
    light_color: vec4<f32>,
    shadow_color: vec4<f32>,
    material_mode: u32, // 0 = Material/Textured, 1 = Solid, 2 = Wireframe
    has_texture: u32,   // 0 = No, 1 = Yes
    _pad2: vec2<u32>,
};

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

@group(1) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(1) @binding(1)
var s_diffuse: sampler;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_pos: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) color: vec4<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.world_pos = in.position;
    out.clip_position = uniforms.view_proj * vec4<f32>(in.position, 1.0);
    out.normal = in.normal;
    out.uv = in.uv;
    out.color = in.color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput, @builtin(front_facing) is_front: bool) -> @location(0) vec4<f32> {
    var base_color: vec4<f32>;
    if (uniforms.material_mode == 1u) {
        base_color = uniforms.object_color;
    } else {
        let tex_color = textureSample(t_diffuse, s_diffuse, in.uv);
        if (in.color.a > 0.0) {
            base_color = tex_color * in.color;
        } else {
            base_color = tex_color * uniforms.object_color;
        }
    }

    var n = normalize(in.normal);
    if (length(n) < 0.1) {
        n = vec3<f32>(0.0, 0.0, 1.0);
    }
    if (!is_front) {
        n = -n;
    }

    let v_dir = normalize(uniforms.camera_pos - in.world_pos);

    // Studio Key Light: uniform camera-relative direction computed once per frame
    let key_dir = normalize(uniforms.light_dir);

    // 1. Directional Key Light + Headlight (powered by light_color)
    let ndotv = max(dot(n, v_dir), 0.0);
    let ndotl = max(dot(n, key_dir), 0.0);
    let key_diff = ndotl * 0.68;
    let headlight = ndotv * 0.18;
    let direct_light = uniforms.light_color.rgb * (headlight + key_diff);

    // 2. Shadow / Ambient Light (powered by shadow_color with baseline fallback)
    let shadow_factor = 1.0 - ndotl * 0.70;
    let hemi_factor = n.z * 0.5 + 0.5;
    let base_ambient = mix(vec3<f32>(0.07, 0.07, 0.09), vec3<f32>(0.13, 0.13, 0.15), hemi_factor);
    let shadow_len = clamp(length(uniforms.shadow_color.rgb), 0.0, 1.0);
    let shadow_light = mix(base_ambient, uniforms.shadow_color.rgb, shadow_len * 0.85) + uniforms.shadow_color.rgb * 0.15;
    let ambient_contrib = shadow_light * shadow_factor;

    // 3. Soft Diffuse Specular (skin-like matte sheen lit by light_color)
    let half_dir = normalize(key_dir + v_dir);
    let spec = pow(max(dot(n, half_dir), 0.0), 16.0) * 0.07;
    let spec_light = uniforms.light_color.rgb * spec;

    // 4. Subtle Rim Light (silhouette edge definition tinted by light_color)
    let rim = pow(1.0 - ndotv, 4.0) * 0.06;
    let rim_color = mix(uniforms.light_color.rgb, vec3<f32>(1.0), 0.3) * rim;

    let total_light = direct_light + ambient_contrib + rim_color;
    let final_rgb = clamp(base_color.rgb * total_light + spec_light, vec3<f32>(0.0), vec3<f32>(1.0));

    return vec4<f32>(final_rgb, base_color.a);
}

// Line rendering for Grid and Axes
struct LineVertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec4<f32>,
};

struct LineVertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vs_line(in: LineVertexInput) -> LineVertexOutput {
    var out: LineVertexOutput;
    out.clip_position = uniforms.view_proj * vec4<f32>(in.position, 1.0);
    out.color = in.color;
    return out;
}

@fragment
fn fs_line(in: LineVertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}

// Wireframe rendering
@vertex
fn vs_wireframe(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.world_pos = in.position;
    out.clip_position = uniforms.view_proj * vec4<f32>(in.position, 1.0);
    out.normal = in.normal;
    out.uv = in.uv;
    out.color = in.color;
    return out;
}

@fragment
fn fs_wireframe(in: VertexOutput) -> @location(0) vec4<f32> {
    return uniforms.object_color;
}

