// Mesh and Line Shader for Mesh 3D viewer

struct Uniforms {
    view_proj: mat4x4<f32>,
    camera_pos: vec3<f32>,
    _pad0: f32,
    light_dir: vec3<f32>,
    _pad1: f32,
    object_color: vec4<f32>,
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
        return base_color;
    }

    var n = normalize(in.normal);
    if (length(n) < 0.1) {
        n = vec3<f32>(0.0, 0.0, 1.0);
    }
    if (!is_front) {
        n = -n;
    }

    let v_dir = normalize(uniforms.camera_pos - in.world_pos);

    // Key light offset slightly above/right from camera for crisp 3D depth
    let key_dir = normalize(v_dir + vec3<f32>(0.25, 0.35, 0.15));
    let diff1 = max(dot(n, key_dir), 0.0) * 0.55;

    // Headlight fill directly from camera position
    let diff2 = max(dot(n, v_dir), 0.0) * 0.35;

    // Specular highlight
    let half_dir = normalize(key_dir + v_dir);
    let spec = pow(max(dot(n, half_dir), 0.0), 32.0) * 0.35;

    let ambient = 0.45;
    let total_light = clamp(ambient + diff1 + diff2 + spec, 0.0, 1.40);

    return vec4<f32>(base_color.rgb * total_light, base_color.a);
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
