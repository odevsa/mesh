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

    if (uniforms.material_mode == 2u) {
        return vec4<f32>(base_color.rgb * 0.25, base_color.a);
    }

    var n = normalize(in.normal);
    if (length(n) < 0.1) {
        n = vec3<f32>(0.0, 0.0, 1.0);
    }
    if (!is_front) {
        n = -n;
    }

    let v_dir = normalize(uniforms.camera_pos - in.world_pos);

    // 1. Generous Hemisphere Ambient Light
    // Soft skylight from +Z and gentle ground bounce from -Z
    let hemi_factor = n.z * 0.5 + 0.5;
    let ambient_sky = vec3<f32>(0.55, 0.55, 0.58);
    let ambient_ground = vec3<f32>(0.40, 0.40, 0.42);
    var ambient_light = mix(ambient_ground, ambient_sky, hemi_factor);

    // Incorporate user shadow_color if configured
    if (length(uniforms.shadow_color.rgb) > 0.01) {
        ambient_light = mix(ambient_light, uniforms.shadow_color.rgb * 0.75, 0.35);
    }

    // 2. Camera Headlight (ensures surfaces facing the camera are always well illuminated)
    let ndotv = max(dot(n, v_dir), 0.0);
    let headlight = ndotv * 0.35;

    // 3. Key Light (slightly offset above/right of camera to give crisp 3D volume)
    let key_dir = normalize(v_dir + vec3<f32>(0.25, 0.35, 0.20));
    let key_diff = max(dot(n, key_dir), 0.0) * 0.45;

    let direct_light = uniforms.light_color.rgb * (headlight + key_diff);

    // 4. Specular Highlight
    let half_dir = normalize(key_dir + v_dir);
    let spec = pow(max(dot(n, half_dir), 0.0), 32.0) * 0.25;

    // 5. Subtle Rim Light (gives clean silhouettes from dark backgrounds)
    let rim = pow(1.0 - ndotv, 3.5) * 0.18;

    let total_light = ambient_light + direct_light + vec3<f32>(rim);
    let final_rgb = clamp(base_color.rgb * total_light + uniforms.light_color.rgb * spec, vec3<f32>(0.0), vec3<f32>(1.0));

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
