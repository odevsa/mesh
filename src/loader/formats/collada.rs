use crate::loader::{
    compute_normals, load_texture_from_path, resolve_relative_file, AnimationChannel,
    AnimationClip, FileFormatInfo, FormatPlugin, Interpolation, Joint, MeshData, SceneNode,
    Skin, SubMesh, TargetProperty, TextureData,
};
use glam::Mat4;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::Path;
use std::sync::Arc;

pub struct ColladaLoader {}

fn quat_to_axis_angle(q: [f32; 4]) -> [f32; 4] {
    let [mut x, mut y, mut z, mut w] = q;
    if w < 0.0 {
        x = -x;
        y = -y;
        z = -z;
        w = -w;
    }
    let angle_rad = 2.0 * w.clamp(-1.0, 1.0).acos();
    let angle_deg = angle_rad.to_degrees();
    let s = (1.0 - w * w).max(0.0).sqrt();
    if s < 1e-4 {
        [0.0, 0.0, 1.0, 0.0]
    } else {
        [x / s, y / s, z / s, angle_deg]
    }
}

fn axis_angle_to_quat(aa: [f32; 4]) -> [f32; 4] {
    let [x, y, z, angle_deg] = aa;
    let len = (x * x + y * y + z * z).sqrt();
    if len < 1e-6 {
        return [0.0, 0.0, 0.0, 1.0];
    }
    let (nx, ny, nz) = (x / len, y / len, z / len);
    let angle_rad = angle_deg.to_radians();
    let half = angle_rad * 0.5;
    let sin = half.sin();
    let cos = half.cos();
    [nx * sin, ny * sin, nz * sin, cos]
}

#[derive(Default, Debug)]
struct ColladaMaterial {
    diffuse_color: Option<[f32; 4]>,
    ambient_color: Option<[f32; 4]>,
    emission_color: Option<[f32; 4]>,
    texture_path: Option<String>,
}

impl FormatPlugin for ColladaLoader {
    fn name(&self) -> &str {
        "collada"
    }

    fn formats(&self) -> Vec<FileFormatInfo> {
        vec![FileFormatInfo {
            extension: "dae",
            label: "COLLADA (*.dae)",
            default_filename: "model.dae",
            can_load: true,
            can_export: true,
        }]
    }

    fn can_load(&self, path: &Path, header: &[u8]) -> bool {
        if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
            if ext.eq_ignore_ascii_case("dae") {
                return true;
            }
        }
        let header_str = String::from_utf8_lossy(header);
        header_str.contains("<COLLADA") || header_str.contains("COLLADASchema")
    }

    fn load(&self, path: &Path, mut reader: Box<dyn Read>) -> Result<MeshData, String> {
        let mut xml_content = String::new();
        reader
            .read_to_string(&mut xml_content)
            .map_err(|e| format!("failed to read COLLADA file: {}", e))?;

        let base_dir = path.parent().unwrap_or_else(|| Path::new("."));

        parse_collada(&xml_content, base_dir)
    }

    fn can_export(&self, path: &Path) -> bool {
        let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
        ext.eq_ignore_ascii_case("dae")
    }

    fn export(&self, path: &Path, mesh: &MeshData) -> Result<(), String> {
        export_collada(path, mesh)
    }

    fn supports_animation(&self) -> bool {
        println!("[COLLADA Format] Format natively supports animation tracks (skeletal/morph)");
        true
    }
}

fn parse_collada(xml: &str, base_dir: &Path) -> Result<MeshData, String> {
    use quick_xml::events::Event;
    use quick_xml::Reader;

    let mut reader = Reader::from_str(xml);
    reader.trim_text(true);

    let mut up_axis = String::from("Z_UP");

    // id -> float numbers
    let mut float_arrays: HashMap<String, Vec<f32>> = HashMap::new();
    // id -> string tokens
    let mut name_arrays: HashMap<String, Vec<String>> = HashMap::new();
    // vertices_id -> position_source_id
    let mut vertices_map: HashMap<String, String> = HashMap::new();
    // image_id -> file path
    let mut image_files: HashMap<String, String> = HashMap::new();
    // effect_id -> ColladaMaterial
    let mut effect_materials: HashMap<String, ColladaMaterial> = HashMap::new();
    // material_id -> effect_id
    let mut material_to_effect: HashMap<String, String> = HashMap::new();
    // symbol -> material_id
    let mut symbol_to_material: HashMap<String, String> = HashMap::new();

    // Context tracking during parse
    let mut current_id = String::new();
    let mut current_source_id = String::new();
    let mut current_geom_id = String::new();
    let mut current_tag = Vec::new();

    // Intermediate primitive storage
    struct PrimGroup {
        geometry_id: String,
        pos_source: String,
        norm_source: Option<String>,
        uv_source: Option<String>,
        pos_offset: usize,
        norm_offset: Option<usize>,
        uv_offset: Option<usize>,
        stride: usize,
        indices: Vec<u32>,
        vcounts: Vec<usize>,
        material_symbol: Option<String>,
    }

    let mut prim_groups: Vec<PrimGroup> = Vec::new();
    let mut current_prim: Option<PrimGroup> = None;

    let mut cur_material_id = String::new();
    let mut cur_effect_id = String::new();
    let mut cur_image_id = String::new();
    let mut cur_newparam_sid = String::new();
    let mut param_to_source: HashMap<String, String> = HashMap::new();

    // Raw skin / controller structures
    struct RawColladaSkin {
        controller_id: String,
        geom_id: String,
        joint_source: String,
        inv_bind_source: String,
        weight_source: String,
        joint_offset: usize,
        weight_offset: usize,
        stride: usize,
        vcount: Vec<usize>,
        v: Vec<u32>,
    }

    let mut raw_skins: Vec<RawColladaSkin> = Vec::new();
    let mut cur_controller_id = String::new();
    let mut cur_skin_geom = String::new();
    let mut cur_skin_joint_src = String::new();
    let mut cur_skin_inv_bind_src = String::new();
    let mut cur_skin_weight_src = String::new();
    let mut cur_skin_joint_offset = 0usize;
    let mut cur_skin_weight_offset = 1usize;
    let mut cur_skin_stride = 2usize;
    let mut cur_skin_vcount = Vec::new();
    let mut cur_skin_v = Vec::new();

    // Raw node hierarchy
    struct RawNode {
        id: String,
        name: String,
        sid: String,
        parent: Option<usize>,
        children: Vec<usize>,
        translation: [f32; 3],
        rotation: [f32; 4], // Quat [x, y, z, w]
        scale: [f32; 3],
        geom_instances: Vec<String>,
        controller_instances: Vec<String>,
    }

    let mut raw_nodes: Vec<RawNode> = Vec::new();
    let mut node_stack: Vec<usize> = Vec::new();

    // Raw animations
    #[derive(Default, Clone)]
    struct RawSampler {
        input_src: String,
        output_src: String,
    }
    struct RawChannel {
        sampler_id: String,
        target: String,
    }
    struct RawClip {
        name: String,
        samplers: HashMap<String, RawSampler>,
        channels: Vec<RawChannel>,
    }

    let mut raw_clips: Vec<RawClip> = Vec::new();
    let mut cur_clip_idx: Option<usize> = None;
    let mut cur_sampler_id = String::new();

    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(event @ (Event::Start(_) | Event::Empty(_))) => {
                let (e, is_start) = match event {
                    Event::Start(ref e) => (e, true),
                    Event::Empty(ref e) => (e, false),
                    _ => unreachable!(),
                };
                let name = e.name().into_inner();
                if is_start {
                    current_tag.push(name.to_vec());
                }

                match name {
                    b"source" => {
                        for attr in e.attributes().with_checks(false).flatten() {
                            if attr.key.into_inner() == b"id" {
                                if let Ok(val) = attr.unescape_value() {
                                    current_source_id = val.to_string();
                                }
                            }
                        }
                    }
                    b"float_array" | b"Name_array" | b"IDREF_array" => {
                        for attr in e.attributes().with_checks(false).flatten() {
                            if attr.key.into_inner() == b"id" {
                                if let Ok(val) = attr.unescape_value() {
                                    current_id = val.to_string();
                                }
                            }
                        }
                    }
                    b"geometry" => {
                        for attr in e.attributes().with_checks(false).flatten() {
                            if attr.key.into_inner() == b"id" {
                                if let Ok(val) = attr.unescape_value() {
                                    current_id = val.to_string();
                                    current_geom_id = val.to_string();
                                }
                            }
                        }
                    }
                    b"material" => {
                        for attr in e.attributes().with_checks(false).flatten() {
                            if attr.key.into_inner() == b"id" {
                                if let Ok(val) = attr.unescape_value() {
                                    cur_material_id = val.to_string();
                                }
                            }
                        }
                    }
                    b"effect" => {
                        for attr in e.attributes().with_checks(false).flatten() {
                            if attr.key.into_inner() == b"id" {
                                if let Ok(val) = attr.unescape_value() {
                                    cur_effect_id = val.to_string();
                                }
                            }
                        }
                    }
                    b"image" => {
                        for attr in e.attributes().with_checks(false).flatten() {
                            if attr.key.into_inner() == b"id" {
                                if let Ok(val) = attr.unescape_value() {
                                    cur_image_id = val.to_string();
                                }
                            }
                        }
                    }
                    b"newparam" => {
                        for attr in e.attributes().with_checks(false).flatten() {
                            if attr.key.into_inner() == b"sid" {
                                if let Ok(val) = attr.unescape_value() {
                                    cur_newparam_sid = val.to_string();
                                }
                            }
                        }
                    }
                    b"texture" => {
                        let mut tex_name = String::new();
                        for attr in e.attributes().with_checks(false).flatten() {
                            if attr.key.into_inner() == b"texture" {
                                if let Ok(val) = attr.unescape_value() {
                                    tex_name = val.to_string();
                                }
                            }
                        }
                        if !tex_name.is_empty() {
                            let mat = effect_materials.entry(cur_effect_id.clone()).or_default();
                            mat.texture_path = Some(tex_name);
                        }
                    }
                    b"instance_effect" => {
                        for attr in e.attributes().with_checks(false).flatten() {
                            if attr.key.into_inner() == b"url" {
                                if let Ok(val) = attr.unescape_value() {
                                    let eff_id = val.trim_start_matches('#').to_string();
                                    material_to_effect.insert(cur_material_id.clone(), eff_id);
                                }
                            }
                        }
                    }
                    b"instance_material" => {
                        let mut sym = String::new();
                        let mut target = String::new();
                        for attr in e.attributes().with_checks(false).flatten() {
                            match attr.key.into_inner() {
                                b"symbol" => {
                                    if let Ok(val) = attr.unescape_value() {
                                        sym = val.to_string();
                                    }
                                }
                                b"target" => {
                                    if let Ok(val) = attr.unescape_value() {
                                        target = val.trim_start_matches('#').to_string();
                                    }
                                }
                                _ => {}
                            }
                        }
                        if !sym.is_empty() && !target.is_empty() {
                            symbol_to_material.insert(sym, target);
                        }
                    }
                    b"vertices" => {
                        for attr in e.attributes().with_checks(false).flatten() {
                            if attr.key.into_inner() == b"id" {
                                if let Ok(val) = attr.unescape_value() {
                                    current_id = val.to_string();
                                }
                            }
                        }
                    }
                    b"triangles" | b"polylist" => {
                        let mut mat_sym = None;
                        for attr in e.attributes().with_checks(false).flatten() {
                            if attr.key.into_inner() == b"material" {
                                if let Ok(val) = attr.unescape_value() {
                                    mat_sym = Some(val.to_string());
                                }
                            }
                        }
                        current_prim = Some(PrimGroup {
                            geometry_id: current_geom_id.clone(),
                            pos_source: String::new(),
                            norm_source: None,
                            uv_source: None,
                            pos_offset: 0,
                            norm_offset: None,
                            uv_offset: None,
                            stride: 1,
                            indices: Vec::new(),
                            vcounts: Vec::new(),
                            material_symbol: mat_sym,
                        });
                    }
                    b"controller" => {
                        for attr in e.attributes().with_checks(false).flatten() {
                            if attr.key.into_inner() == b"id" {
                                if let Ok(val) = attr.unescape_value() {
                                    cur_controller_id = val.to_string();
                                }
                            }
                        }
                    }
                    b"skin" => {
                        for attr in e.attributes().with_checks(false).flatten() {
                            if attr.key.into_inner() == b"source" {
                                if let Ok(val) = attr.unescape_value() {
                                    cur_skin_geom = val.trim_start_matches('#').to_string();
                                }
                            }
                        }
                        cur_skin_joint_src.clear();
                        cur_skin_inv_bind_src.clear();
                        cur_skin_weight_src.clear();
                        cur_skin_joint_offset = 0;
                        cur_skin_weight_offset = 1;
                        cur_skin_stride = 2;
                        cur_skin_vcount.clear();
                        cur_skin_v.clear();
                    }
                    b"node" => {
                        let mut nid = String::new();
                        let mut nname = String::new();
                        let mut nsid = String::new();
                        for attr in e.attributes().with_checks(false).flatten() {
                            match attr.key.into_inner() {
                                b"id" => {
                                    if let Ok(val) = attr.unescape_value() {
                                        nid = val.to_string();
                                    }
                                }
                                b"name" => {
                                    if let Ok(val) = attr.unescape_value() {
                                        nname = val.to_string();
                                    }
                                }
                                b"sid" => {
                                    if let Ok(val) = attr.unescape_value() {
                                        nsid = val.to_string();
                                    }
                                }
                                _ => {}
                            }
                        }
                        let node_idx = raw_nodes.len();
                        let parent = node_stack.last().copied();
                        if let Some(p) = parent {
                            raw_nodes[p].children.push(node_idx);
                        }
                        raw_nodes.push(RawNode {
                            id: nid,
                            name: nname,
                            sid: nsid,
                            parent,
                            children: Vec::new(),
                            translation: [0.0, 0.0, 0.0],
                            rotation: [0.0, 0.0, 0.0, 1.0],
                            scale: [1.0, 1.0, 1.0],
                            geom_instances: Vec::new(),
                            controller_instances: Vec::new(),
                        });
                        node_stack.push(node_idx);
                    }
                    b"instance_geometry" => {
                        for attr in e.attributes().with_checks(false).flatten() {
                            if attr.key.into_inner() == b"url" {
                                if let Ok(val) = attr.unescape_value() {
                                    if let Some(&top) = node_stack.last() {
                                        raw_nodes[top].geom_instances.push(val.trim_start_matches('#').to_string());
                                    }
                                }
                            }
                        }
                    }
                    b"instance_controller" => {
                        for attr in e.attributes().with_checks(false).flatten() {
                            if attr.key.into_inner() == b"url" {
                                if let Ok(val) = attr.unescape_value() {
                                    if let Some(&top) = node_stack.last() {
                                        raw_nodes[top].controller_instances.push(val.trim_start_matches('#').to_string());
                                    }
                                }
                            }
                        }
                    }
                    b"animation" => {
                        let mut anim_name = String::new();
                        for attr in e.attributes().with_checks(false).flatten() {
                            if attr.key.into_inner() == b"name" || attr.key.into_inner() == b"id" {
                                if let Ok(val) = attr.unescape_value() {
                                    if anim_name.is_empty() {
                                        anim_name = val.to_string();
                                    }
                                }
                            }
                        }
                        if cur_clip_idx.is_none() {
                            let idx = raw_clips.len();
                            raw_clips.push(RawClip {
                                name: if anim_name.is_empty() { format!("Animation_{}", idx) } else { anim_name },
                                samplers: HashMap::new(),
                                channels: Vec::new(),
                            });
                            cur_clip_idx = Some(idx);
                        }
                    }
                    b"sampler" => {
                        for attr in e.attributes().with_checks(false).flatten() {
                            if attr.key.into_inner() == b"id" {
                                if let Ok(val) = attr.unescape_value() {
                                    cur_sampler_id = val.to_string();
                                }
                            }
                        }
                    }
                    b"channel" => {
                        let mut src = String::new();
                        let mut tgt = String::new();
                        for attr in e.attributes().with_checks(false).flatten() {
                            match attr.key.into_inner() {
                                b"source" => {
                                    if let Ok(val) = attr.unescape_value() {
                                        src = val.trim_start_matches('#').to_string();
                                    }
                                }
                                b"target" => {
                                    if let Ok(val) = attr.unescape_value() {
                                        tgt = val.to_string();
                                    }
                                }
                                _ => {}
                            }
                        }
                        if !src.is_empty() && !tgt.is_empty() {
                            if let Some(cidx) = cur_clip_idx {
                                raw_clips[cidx].channels.push(RawChannel {
                                    sampler_id: src,
                                    target: tgt,
                                });
                            }
                        }
                    }
                    b"input" => {
                        let mut semantic = String::new();
                        let mut source = String::new();
                        let mut offset = 0usize;

                        for attr in e.attributes().with_checks(false).flatten() {
                            match attr.key.into_inner() {
                                b"semantic" => {
                                    if let Ok(val) = attr.unescape_value() {
                                        semantic = val.to_string();
                                    }
                                }
                                b"source" => {
                                    if let Ok(val) = attr.unescape_value() {
                                        source = val.trim_start_matches('#').to_string();
                                    }
                                }
                                b"offset" => {
                                    if let Ok(val) = attr.unescape_value() {
                                        offset = val.parse().unwrap_or(0);
                                    }
                                }
                                _ => {}
                            }
                        }

                        if current_tag.iter().any(|t| t == b"vertices") {
                            if semantic == "POSITION" {
                                vertices_map.insert(current_id.clone(), source);
                            }
                        } else if current_tag.iter().any(|t| t == b"joints") {
                            if semantic == "JOINT" {
                                cur_skin_joint_src = source;
                            } else if semantic == "INV_BIND_MATRIX" {
                                cur_skin_inv_bind_src = source;
                            }
                        } else if current_tag.iter().any(|t| t == b"vertex_weights") {
                            if semantic == "JOINT" {
                                cur_skin_joint_offset = offset;
                                cur_skin_stride = cur_skin_stride.max(offset + 1);
                            } else if semantic == "WEIGHT" {
                                cur_skin_weight_offset = offset;
                                cur_skin_weight_src = source;
                                cur_skin_stride = cur_skin_stride.max(offset + 1);
                            }
                        } else if current_tag.iter().any(|t| t == b"sampler") {
                            if let Some(cidx) = cur_clip_idx {
                                let samp = raw_clips[cidx].samplers.entry(cur_sampler_id.clone()).or_default();
                                if semantic == "INPUT" {
                                    samp.input_src = source;
                                } else if semantic == "OUTPUT" {
                                    samp.output_src = source;
                                }
                            }
                        } else if let Some(ref mut prim) = current_prim {
                            prim.stride = prim.stride.max(offset + 1);
                            match semantic.as_str() {
                                "VERTEX" | "POSITION" => {
                                    prim.pos_source = source;
                                    prim.pos_offset = offset;
                                }
                                "NORMAL" => {
                                    prim.norm_source = Some(source);
                                    prim.norm_offset = Some(offset);
                                }
                                "TEXCOORD" => {
                                    if prim.uv_source.is_none() {
                                        prim.uv_source = Some(source);
                                        prim.uv_offset = Some(offset);
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::Text(ref e)) => {
                let text = match e.unescape() {
                    Ok(t) => t,
                    Err(_) => continue,
                };
                let last = current_tag.last().map(|v| v.as_slice()).unwrap_or(b"");

                match last {
                    b"up_axis" => {
                        up_axis = text.trim().to_uppercase();
                    }
                    b"float_array" => {
                        let floats: Vec<f32> = text
                            .split_ascii_whitespace()
                            .filter_map(|s| s.parse::<f32>().ok())
                            .collect();
                        if !floats.is_empty() {
                            float_arrays.insert(current_id.clone(), floats.clone());
                            if !current_source_id.is_empty() {
                                float_arrays.insert(current_source_id.clone(), floats);
                            }
                        }
                    }
                    b"Name_array" | b"IDREF_array" => {
                        let names: Vec<String> = text
                            .split_ascii_whitespace()
                            .map(|s| s.to_string())
                            .collect();
                        if !names.is_empty() {
                            name_arrays.insert(current_id.clone(), names.clone());
                            if !current_source_id.is_empty() {
                                name_arrays.insert(current_source_id.clone(), names);
                            }
                        }
                    }
                    b"init_from" => {
                        let val = text.trim().to_string();
                        if current_tag.iter().any(|t| t == b"image") && !cur_image_id.is_empty() {
                            image_files.insert(cur_image_id.clone(), val.clone());
                        }
                        if current_tag.iter().any(|t| t == b"newparam") && !cur_newparam_sid.is_empty() {
                            param_to_source.insert(cur_newparam_sid.clone(), val);
                        }
                    }
                    b"source" => {
                        let val = text.trim().to_string();
                        if !cur_newparam_sid.is_empty() {
                            param_to_source.insert(cur_newparam_sid.clone(), val);
                        }
                    }
                    b"color" => {
                        let cols: Vec<f32> = text
                            .split_ascii_whitespace()
                            .filter_map(|s| s.parse::<f32>().ok())
                            .collect();
                        if cols.len() >= 3 {
                            let a = if cols.len() >= 4 { cols[3] } else { 1.0 };
                            let is_diffuse = current_tag.iter().any(|t| t == b"diffuse");
                            let is_ambient = current_tag.iter().any(|t| t == b"ambient");
                            let is_emission = current_tag.iter().any(|t| t == b"emission");

                            let mat = effect_materials.entry(cur_effect_id.clone()).or_default();
                            if is_diffuse {
                                mat.diffuse_color = Some([cols[0], cols[1], cols[2], a]);
                            } else if is_ambient {
                                mat.ambient_color = Some([cols[0], cols[1], cols[2], a]);
                            } else if is_emission {
                                mat.emission_color = Some([cols[0], cols[1], cols[2], a]);
                            }
                        }
                    }
                    b"texture" => {
                        let val = text.trim().to_string();
                        if !val.is_empty() {
                            let mat = effect_materials.entry(cur_effect_id.clone()).or_default();
                            if mat.texture_path.is_none() {
                                mat.texture_path = Some(val);
                            }
                        }
                    }
                    b"vcount" => {
                        let counts: Vec<usize> = text
                            .split_ascii_whitespace()
                            .filter_map(|s| s.parse::<usize>().ok())
                            .collect();
                        if current_tag.iter().any(|t| t == b"vertex_weights") {
                            cur_skin_vcount.extend(counts);
                        } else if let Some(ref mut prim) = current_prim {
                            prim.vcounts.extend(counts);
                        }
                    }
                    b"v" => {
                        if current_tag.iter().any(|t| t == b"vertex_weights") {
                            let vals: Vec<u32> = text
                                .split_ascii_whitespace()
                                .filter_map(|s| s.parse::<u32>().ok())
                                .collect();
                            cur_skin_v.extend(vals);
                        }
                    }
                    b"p" => {
                        if let Some(ref mut prim) = current_prim {
                            let idxs: Vec<u32> = text
                                .split_ascii_whitespace()
                                .filter_map(|s| s.parse::<u32>().ok())
                                .collect();
                            prim.indices.extend(idxs);
                        }
                    }
                    b"translate" => {
                        if let Some(&top) = node_stack.last() {
                            let vals: Vec<f32> = text
                                .split_ascii_whitespace()
                                .filter_map(|s| s.parse::<f32>().ok())
                                .collect();
                            if vals.len() >= 3 {
                                raw_nodes[top].translation = [vals[0], vals[1], vals[2]];
                            }
                        }
                    }
                    b"rotate" => {
                        if let Some(&top) = node_stack.last() {
                            let vals: Vec<f32> = text
                                .split_ascii_whitespace()
                                .filter_map(|s| s.parse::<f32>().ok())
                                .collect();
                            if vals.len() >= 4 {
                                let q = axis_angle_to_quat([vals[0], vals[1], vals[2], vals[3]]);
                                let q_glam = glam::Quat::from_xyzw(q[0], q[1], q[2], q[3]);
                                let cur_q = glam::Quat::from_xyzw(
                                    raw_nodes[top].rotation[0],
                                    raw_nodes[top].rotation[1],
                                    raw_nodes[top].rotation[2],
                                    raw_nodes[top].rotation[3],
                                );
                                let new_q = (cur_q * q_glam).normalize();
                                raw_nodes[top].rotation = [new_q.x, new_q.y, new_q.z, new_q.w];
                            }
                        }
                    }
                    b"scale" => {
                        if let Some(&top) = node_stack.last() {
                            let vals: Vec<f32> = text
                                .split_ascii_whitespace()
                                .filter_map(|s| s.parse::<f32>().ok())
                                .collect();
                            if vals.len() >= 3 {
                                raw_nodes[top].scale = [vals[0], vals[1], vals[2]];
                            }
                        }
                    }
                    b"matrix" => {
                        if let Some(&top) = node_stack.last() {
                            let vals: Vec<f32> = text
                                .split_ascii_whitespace()
                                .filter_map(|s| s.parse::<f32>().ok())
                                .collect();
                            if vals.len() >= 16 {
                                let mat = Mat4::from_cols_array(&[
                                    vals[0], vals[4], vals[8], vals[12],
                                    vals[1], vals[5], vals[9], vals[13],
                                    vals[2], vals[6], vals[10], vals[14],
                                    vals[3], vals[7], vals[11], vals[15],
                                ]);
                                let (sc, rot, tr) = mat.to_scale_rotation_translation();
                                raw_nodes[top].translation = tr.to_array();
                                raw_nodes[top].rotation = [rot.x, rot.y, rot.z, rot.w];
                                raw_nodes[top].scale = sc.to_array();
                            }
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::End(ref e)) => {
                let name = e.name().into_inner();
                match name {
                    b"triangles" | b"polylist" => {
                        if let Some(prim) = current_prim.take() {
                            prim_groups.push(prim);
                        }
                    }
                    b"skin" => {
                        if !cur_skin_geom.is_empty() {
                            raw_skins.push(RawColladaSkin {
                                controller_id: cur_controller_id.clone(),
                                geom_id: cur_skin_geom.clone(),
                                joint_source: cur_skin_joint_src.clone(),
                                inv_bind_source: cur_skin_inv_bind_src.clone(),
                                weight_source: cur_skin_weight_src.clone(),
                                joint_offset: cur_skin_joint_offset,
                                weight_offset: cur_skin_weight_offset,
                                stride: cur_skin_stride,
                                vcount: std::mem::take(&mut cur_skin_vcount),
                                v: std::mem::take(&mut cur_skin_v),
                            });
                        }
                    }
                    b"node" => {
                        node_stack.pop();
                    }
                    b"animation" => {
                        if current_tag.len() <= 2 {
                            cur_clip_idx = None;
                        }
                    }
                    b"image" => cur_image_id.clear(),
                    b"newparam" => cur_newparam_sid.clear(),
                    b"effect" => cur_effect_id.clear(),
                    b"material" => cur_material_id.clear(),
                    _ => {}
                }
                current_tag.pop();
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(format!("XML error at position {}: {:?}", reader.buffer_position(), e)),
            _ => {}
        }
        buf.clear();
    }

    if prim_groups.is_empty() {
        return Err("COLLADA file contains no geometry primitives".into());
    }

    // Material texture cache
    let mut loaded_textures: HashMap<String, Option<Arc<TextureData>>> = HashMap::new();

    let is_y_up = up_axis.starts_with('Y');

    // Build node hierarchy & lookup map
    let mut nodes: Vec<SceneNode> = Vec::with_capacity(raw_nodes.len());
    let mut node_lookup: HashMap<String, usize> = HashMap::new();

    for (idx, rn) in raw_nodes.iter().enumerate() {
        if !rn.id.is_empty() {
            node_lookup.insert(rn.id.clone(), idx);
            if let Some(suf) = rn.id.strip_prefix("Node_") {
                node_lookup.insert(suf.to_string(), idx);
            }
        }
        if !rn.sid.is_empty() {
            node_lookup.insert(rn.sid.clone(), idx);
        }
        if !rn.name.is_empty() {
            node_lookup.insert(rn.name.clone(), idx);
        }

        nodes.push(SceneNode {
            name: if rn.name.is_empty() { rn.id.clone() } else { rn.name.clone() },
            parent: rn.parent,
            children: rn.children.clone(),
            initial_translation: rn.translation,
            initial_rotation: rn.rotation,
            initial_scale: rn.scale,
            mesh_index: None,
        });
    }

    // Process skins & vertex weights
    let mut skins = Vec::new();
    // geom_id -> (skin_idx, HashMap<vert_idx, (joint_indices, joint_weights)>)
    let mut skin_by_geom: HashMap<String, (usize, HashMap<usize, ([u16; 4], [f32; 4])>)> = HashMap::new();

    for raw_skin in &raw_skins {
        let mut joints = Vec::new();
        let joint_names = name_arrays.get(&raw_skin.joint_source)
            .or_else(|| name_arrays.get(raw_skin.joint_source.trim_start_matches('#')));
        let inv_bind_floats = float_arrays.get(&raw_skin.inv_bind_source)
            .or_else(|| float_arrays.get(raw_skin.inv_bind_source.trim_start_matches('#')));

        if let Some(names) = joint_names {
            for (j_idx, name) in names.iter().enumerate() {
                let clean = name.trim_start_matches('#');
                let n_idx = node_lookup.get(clean)
                    .or_else(|| node_lookup.get(name.as_str()))
                    .copied()
                    .unwrap_or(j_idx.min(nodes.len().saturating_sub(1)));

                let mut inv = [[0.0f32; 4]; 4];
                if let Some(floats) = inv_bind_floats {
                    for r in 0..4 {
                        for c in 0..4 {
                            inv[c][r] = floats.get(j_idx * 16 + r * 4 + c).copied().unwrap_or(if r == c { 1.0 } else { 0.0 });
                        }
                    }
                } else {
                    inv = Mat4::IDENTITY.to_cols_array_2d();
                }

                joints.push(Joint {
                    node_index: n_idx,
                    inverse_bind_matrix: inv,
                });
            }
        }

        let weights_floats = float_arrays.get(&raw_skin.weight_source)
            .or_else(|| float_arrays.get(raw_skin.weight_source.trim_start_matches('#')));

        let mut vert_weights: HashMap<usize, ([u16; 4], [f32; 4])> = HashMap::new();
        if let (Some(weights_arr), false) = (weights_floats, raw_skin.vcount.is_empty()) {
            let mut v_ptr = 0;
            for (v_idx, &cnt) in raw_skin.vcount.iter().enumerate() {
                let mut pairs: Vec<(u16, f32)> = Vec::with_capacity(cnt);
                for _ in 0..cnt {
                    if v_ptr + raw_skin.stride <= raw_skin.v.len() {
                        let j_i = raw_skin.v[v_ptr + raw_skin.joint_offset] as u16;
                        let w_i = raw_skin.v[v_ptr + raw_skin.weight_offset] as usize;
                        let w_val = weights_arr.get(w_i).copied().unwrap_or(0.0);
                        if w_val > 0.0001 {
                            pairs.push((j_i, w_val));
                        }
                        v_ptr += raw_skin.stride;
                    }
                }
                pairs.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
                let mut ji = [0u16; 4];
                let mut jw = [0.0f32; 4];
                let mut sum = 0.0;
                for k in 0..4.min(pairs.len()) {
                    ji[k] = pairs[k].0;
                    jw[k] = pairs[k].1;
                    sum += pairs[k].1;
                }
                if sum > 0.0001 {
                    for k in 0..4 {
                        jw[k] /= sum;
                    }
                } else {
                    jw[0] = 1.0;
                }
                vert_weights.insert(v_idx, (ji, jw));
            }
        }

        let skin_idx = skins.len();
        skins.push(Skin {
            name: raw_skin.controller_id.clone(),
            joints,
        });
        skin_by_geom.insert(raw_skin.geom_id.clone(), (skin_idx, vert_weights));
    }

    let mut submeshes = Vec::new();

    for prim in prim_groups {
        let pos_src_id = vertices_map.get(&prim.pos_source).unwrap_or(&prim.pos_source);
        let pos_array = match float_arrays.get(pos_src_id) {
            Some(a) => a,
            None => continue,
        };
        let norm_array = prim.norm_source.as_ref().and_then(|s| float_arrays.get(s));
        let uv_array = prim.uv_source.as_ref().and_then(|s| float_arrays.get(s));

        // Resolve material color / texture
        let mut mat_color = None;
        let mut mat_texture = None;

        if let Some(ref sym) = prim.material_symbol {
            let mat_id = symbol_to_material.get(sym).cloned()
                .or_else(|| symbol_to_material.iter().find(|(k, _)| k.eq_ignore_ascii_case(sym)).map(|(_, v)| v.clone()))
                .unwrap_or_else(|| sym.clone());
            let effect_id = material_to_effect.get(&mat_id).cloned()
                .or_else(|| material_to_effect.iter().find(|(k, _)| k.eq_ignore_ascii_case(&mat_id)).map(|(_, v)| v.clone()))
                .unwrap_or(mat_id);
            let effect = effect_materials.get(&effect_id)
                .or_else(|| effect_materials.iter().find(|(k, _)| k.eq_ignore_ascii_case(&effect_id)).map(|(_, v)| v));

            if let Some(effect) = effect {
                mat_color = effect.diffuse_color
                    .or(effect.ambient_color)
                    .or(effect.emission_color);
                if let Some(ref tex_key) = effect.texture_path {
                    let mut resolved = tex_key.as_str();
                    if let Some(s) = param_to_source.get(resolved) {
                        resolved = s.as_str();
                        if let Some(s2) = param_to_source.get(resolved) {
                            resolved = s2.as_str();
                        }
                    }
                    let file_ref = image_files.get(resolved).map(|s| s.as_str()).unwrap_or(resolved);
                    let clean_ref = file_ref.trim_start_matches("file://");
                    if let Some(tex) = loaded_textures.entry(clean_ref.to_string()).or_insert_with(|| {
                        if let Some(path) = resolve_relative_file(base_dir, clean_ref) {
                            load_texture_from_path(&path)
                        } else {
                            None
                        }
                    }) {
                        mat_texture = Some(tex.clone());
                    }
                }
            }
        }

        let stride = prim.stride.max(1);
        let mut positions = Vec::new();
        let mut rest_positions = Vec::new();
        let mut normals = Vec::new();
        let mut rest_normals = Vec::new();
        let mut uvs = Vec::new();
        let mut indices = Vec::new();

        let skin_info = skin_by_geom.get(&prim.geometry_id)
            .or_else(|| skin_by_geom.iter().find(|(k, _)| prim.pos_source.contains(k.as_str())).map(|(_, v)| v));
        let mut joint_indices_vec: Vec<[u16; 4]> = Vec::new();
        let mut joint_weights_vec: Vec<[f32; 4]> = Vec::new();
        let has_skin = skin_info.is_some();

        // Helper to extract one vertex tuple
        let get_vertex = |tuple_idx: usize| -> (usize, [f32; 3], [f32; 3], [f32; 3], [f32; 3], [f32; 2]) {
            let base = tuple_idx * stride;
            let pi = prim.indices.get(base + prim.pos_offset).copied().unwrap_or(0) as usize;
            let p_raw = if pi * 3 + 2 < pos_array.len() {
                [pos_array[pi * 3], pos_array[pi * 3 + 1], pos_array[pi * 3 + 2]]
            } else {
                [0.0, 0.0, 0.0]
            };

            let p = if is_y_up {
                [p_raw[0], -p_raw[2], p_raw[1]]
            } else {
                p_raw
            };

            let n_raw = if let (Some(no), Some(n_arr)) = (prim.norm_offset, norm_array) {
                let ni = prim.indices.get(base + no).copied().unwrap_or(0) as usize;
                if ni * 3 + 2 < n_arr.len() {
                    [n_arr[ni * 3], n_arr[ni * 3 + 1], n_arr[ni * 3 + 2]]
                } else {
                    [0.0, 0.0, 1.0]
                }
            } else {
                [0.0, 0.0, 1.0]
            };

            let n = if is_y_up {
                [n_raw[0], -n_raw[2], n_raw[1]]
            } else {
                n_raw
            };

            let uv = if let (Some(uo), Some(uv_arr)) = (prim.uv_offset, uv_array) {
                let ui = prim.indices.get(base + uo).copied().unwrap_or(0) as usize;
                if ui * 2 + 1 < uv_arr.len() {
                    // Collada T coordinate has origin at bottom
                    [uv_arr[ui * 2], 1.0 - uv_arr[ui * 2 + 1]]
                } else {
                    [0.0, 0.0]
                }
            } else {
                [0.0, 0.0]
            };

            (pi, p, p_raw, n, n_raw, uv)
        };

        let add_vert = |pi: usize,
                        p: [f32; 3],
                        p_raw: [f32; 3],
                        n: [f32; 3],
                        n_raw: [f32; 3],
                        uv: [f32; 2],
                        pos: &mut Vec<[f32; 3]>,
                        rpos: &mut Vec<[f32; 3]>,
                        norms: &mut Vec<[f32; 3]>,
                        rnorms: &mut Vec<[f32; 3]>,
                        tex_uvs: &mut Vec<[f32; 2]>,
                        jis: &mut Vec<[u16; 4]>,
                        jws: &mut Vec<[f32; 4]>| {
            pos.push(p);
            rpos.push(p_raw);
            norms.push(n);
            rnorms.push(n_raw);
            tex_uvs.push(uv);
            if has_skin {
                if let Some((_, weights_map)) = skin_info {
                    let (ji, jw) = weights_map.get(&pi).copied().unwrap_or(([0, 0, 0, 0], [1.0, 0.0, 0.0, 0.0]));
                    jis.push(ji);
                    jws.push(jw);
                }
            }
        };

        // Triangulate
        if prim.vcounts.is_empty() {
            let num_tris = (prim.indices.len() / stride) / 3;
            for t in 0..num_tris {
                let i0 = t * 3;
                let i1 = t * 3 + 1;
                let i2 = t * 3 + 2;

                let (pi0, p0, pr0, n0, nr0, uv0) = get_vertex(i0);
                let (pi1, p1, pr1, n1, nr1, uv1) = get_vertex(i1);
                let (pi2, p2, pr2, n2, nr2, uv2) = get_vertex(i2);

                let base_idx = positions.len() as u32;
                add_vert(pi0, p0, pr0, n0, nr0, uv0, &mut positions, &mut rest_positions, &mut normals, &mut rest_normals, &mut uvs, &mut joint_indices_vec, &mut joint_weights_vec);
                add_vert(pi1, p1, pr1, n1, nr1, uv1, &mut positions, &mut rest_positions, &mut normals, &mut rest_normals, &mut uvs, &mut joint_indices_vec, &mut joint_weights_vec);
                add_vert(pi2, p2, pr2, n2, nr2, uv2, &mut positions, &mut rest_positions, &mut normals, &mut rest_normals, &mut uvs, &mut joint_indices_vec, &mut joint_weights_vec);

                indices.push([base_idx, base_idx + 1, base_idx + 2]);
            }
        } else {
            let mut tuple_offset = 0;
            for vc in prim.vcounts {
                if vc >= 3 {
                    let start_idx = positions.len() as u32;
                    for k in 0..vc {
                        let (pik, pk, prk, nk, nrk, uvk) = get_vertex(tuple_offset + k);
                        add_vert(pik, pk, prk, nk, nrk, uvk, &mut positions, &mut rest_positions, &mut normals, &mut rest_normals, &mut uvs, &mut joint_indices_vec, &mut joint_weights_vec);
                    }
                    for k in 1..(vc - 1) {
                        indices.push([
                            start_idx,
                            start_idx + k as u32,
                            start_idx + k as u32 + 1,
                        ]);
                    }
                }
                tuple_offset += vc;
            }
        }

        if !positions.is_empty() && !indices.is_empty() {
            if prim.norm_source.is_none() {
                normals = compute_normals(&positions, &indices);
                rest_normals = compute_normals(&rest_positions, &indices);
            }

            submeshes.push(SubMesh {
                positions,
                rest_positions,
                normals,
                rest_normals,
                uvs,
                indices,
                color: mat_color,
                texture: mat_texture,
                joint_indices: if has_skin { Some(joint_indices_vec) } else { None },
                joint_weights: if has_skin { Some(joint_weights_vec) } else { None },
                node_index: None,
            });
        }
    }

    if submeshes.is_empty() {
        return Err("No valid geometry decoded from COLLADA".into());
    }

    // Process Animation Clips
    let mut animations: Vec<AnimationClip> = Vec::new();

    for clip in raw_clips {
        let mut channels = Vec::new();
        for ch in clip.channels {
            let target_parts: Vec<&str> = ch.target.split('/').collect();
            if target_parts.is_empty() {
                continue;
            }
            let node_id = target_parts[0];
            let attr = if target_parts.len() > 1 { target_parts[1].to_lowercase() } else { "transform".to_string() };

            let node_idx = match node_lookup.get(node_id) {
                Some(&n) => n,
                None => continue,
            };

            let sampler = match clip.samplers.get(&ch.sampler_id) {
                Some(s) => s,
                None => continue,
            };

            let times = match float_arrays.get(&sampler.input_src).or_else(|| float_arrays.get(sampler.input_src.trim_start_matches('#'))) {
                Some(t) => t.clone(),
                None => continue,
            };

            let values = match float_arrays.get(&sampler.output_src).or_else(|| float_arrays.get(sampler.output_src.trim_start_matches('#'))) {
                Some(v) => v.clone(),
                None => continue,
            };

            if times.is_empty() || values.is_empty() {
                continue;
            }

            if attr.contains("translation") || attr.contains("translate") {
                channels.push(AnimationChannel {
                    target_node: node_idx,
                    property: TargetProperty::Translation,
                    interpolation: Interpolation::Linear,
                    timestamps: times,
                    values,
                });
            } else if attr.contains("rotat") {
                // COLLADA rotation is [axis_x, axis_y, axis_z, angle_deg] per key
                let mut quat_values = Vec::with_capacity(times.len() * 4);
                for k in 0..times.len() {
                    let base = k * 4;
                    let aa = [
                        values.get(base).copied().unwrap_or(0.0),
                        values.get(base + 1).copied().unwrap_or(0.0),
                        values.get(base + 2).copied().unwrap_or(1.0),
                        values.get(base + 3).copied().unwrap_or(0.0),
                    ];
                    let q = axis_angle_to_quat(aa);
                    quat_values.extend_from_slice(&q);
                }
                channels.push(AnimationChannel {
                    target_node: node_idx,
                    property: TargetProperty::Rotation,
                    interpolation: Interpolation::Linear,
                    timestamps: times,
                    values: quat_values,
                });
            } else if attr.contains("scale") {
                channels.push(AnimationChannel {
                    target_node: node_idx,
                    property: TargetProperty::Scale,
                    interpolation: Interpolation::Linear,
                    timestamps: times,
                    values,
                });
            } else if (attr.contains("matrix") || attr.contains("transform")) && values.len() == times.len() * 16 {
                // Decompose 16-float matrix keys into Translation, Rotation, Scale channels
                let mut trans_vals = Vec::with_capacity(times.len() * 3);
                let mut rot_vals = Vec::with_capacity(times.len() * 4);
                let mut scale_vals = Vec::with_capacity(times.len() * 3);
                for k in 0..times.len() {
                    let base = k * 16;
                    let m = &values[base..base + 16];
                    let mat = Mat4::from_cols_array(&[
                        m[0], m[4], m[8], m[12],
                        m[1], m[5], m[9], m[13],
                        m[2], m[6], m[10], m[14],
                        m[3], m[7], m[11], m[15],
                    ]);
                    let (sc, rot, tr) = mat.to_scale_rotation_translation();
                    trans_vals.extend_from_slice(&tr.to_array());
                    rot_vals.extend_from_slice(&[rot.x, rot.y, rot.z, rot.w]);
                    scale_vals.extend_from_slice(&sc.to_array());
                }
                channels.push(AnimationChannel {
                    target_node: node_idx,
                    property: TargetProperty::Translation,
                    interpolation: Interpolation::Linear,
                    timestamps: times.clone(),
                    values: trans_vals,
                });
                channels.push(AnimationChannel {
                    target_node: node_idx,
                    property: TargetProperty::Rotation,
                    interpolation: Interpolation::Linear,
                    timestamps: times.clone(),
                    values: rot_vals,
                });
                channels.push(AnimationChannel {
                    target_node: node_idx,
                    property: TargetProperty::Scale,
                    interpolation: Interpolation::Linear,
                    timestamps: times,
                    values: scale_vals,
                });
            }
        }

        if !channels.is_empty() {
            let duration = channels
                .iter()
                .flat_map(|c| c.timestamps.iter())
                .copied()
                .fold(0.0f32, f32::max);
            animations.push(AnimationClip {
                name: clip.name,
                duration,
                channels,
            });
        }
    }

    // Flatten for legacy views
    let mut all_positions = Vec::new();
    let mut all_normals = Vec::new();
    let mut all_uvs = Vec::new();
    let mut all_indices = Vec::new();

    for sub in &submeshes {
        let base = all_positions.len() as u32;
        all_positions.extend_from_slice(&sub.positions);
        all_normals.extend_from_slice(&sub.normals);
        all_uvs.extend_from_slice(&sub.uvs);
        for tri in &sub.indices {
            all_indices.push([tri[0] + base, tri[1] + base, tri[2] + base]);
        }
    }

    let has_animation = !animations.is_empty();

    Ok(MeshData {
        positions: all_positions,
        normals: all_normals,
        uvs: all_uvs,
        indices: all_indices,
        submeshes,
        source_path: None,
        has_animation,
        is_y_up,
        nodes,
        skins,
        animations,
    })
}

fn export_collada(path: &Path, mesh: &MeshData) -> Result<(), String> {
    let fallback_submesh;
    let submeshes: &[SubMesh] = if !mesh.submeshes.is_empty() {
        &mesh.submeshes
    } else if !mesh.positions.is_empty() && !mesh.indices.is_empty() {
        fallback_submesh = vec![SubMesh {
            positions: mesh.positions.clone(),
            normals: mesh.normals.clone(),
            uvs: mesh.uvs.clone(),
            indices: mesh.indices.clone(),
            color: None,
            texture: None,
            ..Default::default()
        }];
        &fallback_submesh
    } else {
        return Err("Mesh contains no geometry to export".into());
    };

    let parent_dir = path.parent().unwrap_or_else(|| Path::new("."));
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("model");
    let mut tex_files: Vec<Option<String>> = Vec::with_capacity(submeshes.len());
    let mut has_any_tex = false;

    for (i, sub) in submeshes.iter().enumerate() {
        if let Some(ref tex) = sub.texture {
            let tex_filename = format!("{}_tex_{}.png", stem, i + 1);
            let tex_path = parent_dir.join(&tex_filename);
            if let Some(img_buf) = image::RgbaImage::from_raw(tex.width, tex.height, tex.rgba.clone()) {
                let _ = img_buf.save(&tex_path);
            }
            tex_files.push(Some(tex_filename));
            has_any_tex = true;
        } else {
            tex_files.push(None);
        }
    }

    let mut out = std::fs::File::create(path).map_err(|e| format!("create file: {}", e))?;

    writeln!(out, r#"<?xml version="1.0" encoding="utf-8"?>"#).map_err(|e| e.to_string())?;
    writeln!(out, r#"<COLLADA xmlns="http://www.collada.org/2005/11/COLLADASchema" version="1.4.1">"#).map_err(|e| e.to_string())?;
    writeln!(out, r#"  <asset>"#).map_err(|e| e.to_string())?;
    writeln!(out, r#"    <contributor><authoring_tool>Mesh 3D Viewer</authoring_tool></contributor>"#).map_err(|e| e.to_string())?;
    writeln!(out, r#"    <unit name="meter" meter="1"/>"#).map_err(|e| e.to_string())?;
    if mesh.is_y_up {
        writeln!(out, r#"    <up_axis>Y_UP</up_axis>"#).map_err(|e| e.to_string())?;
    } else {
        writeln!(out, r#"    <up_axis>Z_UP</up_axis>"#).map_err(|e| e.to_string())?;
    }
    writeln!(out, r#"  </asset>"#).map_err(|e| e.to_string())?;

    // 1. Images
    if has_any_tex {
        writeln!(out, r#"  <library_images>"#).map_err(|e| e.to_string())?;
        for (i, tex_opt) in tex_files.iter().enumerate() {
            if let Some(fname) = tex_opt {
                writeln!(out, r#"    <image id="image_{}" name="image_{}">"#, i, i).map_err(|e| e.to_string())?;
                writeln!(out, r#"      <init_from>{}</init_from>"#, fname).map_err(|e| e.to_string())?;
                writeln!(out, r#"    </image>"#).map_err(|e| e.to_string())?;
            }
        }
        writeln!(out, r#"  </library_images>"#).map_err(|e| e.to_string())?;
    }

    // 2. Effects
    writeln!(out, r#"  <library_effects>"#).map_err(|e| e.to_string())?;
    for (i, sub) in submeshes.iter().enumerate() {
        writeln!(out, r#"    <effect id="effect_{}">"#, i).map_err(|e| e.to_string())?;
        writeln!(out, r#"      <profile_COMMON>"#).map_err(|e| e.to_string())?;
        if tex_files[i].is_some() {
            writeln!(out, r#"        <newparam sid="surface_{}">"#, i).map_err(|e| e.to_string())?;
            writeln!(out, r#"          <surface type="2D">"#).map_err(|e| e.to_string())?;
            writeln!(out, r#"            <init_from>image_{}</init_from>"#, i).map_err(|e| e.to_string())?;
            writeln!(out, r#"          </surface>"#).map_err(|e| e.to_string())?;
            writeln!(out, r#"        </newparam>"#).map_err(|e| e.to_string())?;
            writeln!(out, r#"        <newparam sid="sampler_{}">"#, i).map_err(|e| e.to_string())?;
            writeln!(out, r#"          <sampler2D>"#).map_err(|e| e.to_string())?;
            writeln!(out, r#"            <source>surface_{}</source>"#, i).map_err(|e| e.to_string())?;
            writeln!(out, r#"          </sampler2D>"#).map_err(|e| e.to_string())?;
            writeln!(out, r#"        </newparam>"#).map_err(|e| e.to_string())?;
            writeln!(out, r#"        <technique sid="common">"#).map_err(|e| e.to_string())?;
            writeln!(out, r#"          <lambert>"#).map_err(|e| e.to_string())?;
            writeln!(out, r#"            <diffuse>"#).map_err(|e| e.to_string())?;
            writeln!(out, r#"              <texture texture="sampler_{}" texcoord="geom_{}_uv"/>"#, i, i).map_err(|e| e.to_string())?;
            writeln!(out, r#"            </diffuse>"#).map_err(|e| e.to_string())?;
            writeln!(out, r#"          </lambert>"#).map_err(|e| e.to_string())?;
            writeln!(out, r#"        </technique>"#).map_err(|e| e.to_string())?;
        } else {
            let (r, g, b, a) = if let Some(c) = sub.color {
                (c[0], c[1], c[2], c[3])
            } else {
                (0.8, 0.8, 0.8, 1.0)
            };
            writeln!(out, r#"        <technique sid="common">"#).map_err(|e| e.to_string())?;
            writeln!(out, r#"          <lambert>"#).map_err(|e| e.to_string())?;
            writeln!(out, r#"            <diffuse><color>{:.6} {:.6} {:.6} {:.6}</color></diffuse>"#, r, g, b, a).map_err(|e| e.to_string())?;
            writeln!(out, r#"          </lambert>"#).map_err(|e| e.to_string())?;
            writeln!(out, r#"        </technique>"#).map_err(|e| e.to_string())?;
        }
        writeln!(out, r#"      </profile_COMMON>"#).map_err(|e| e.to_string())?;
        writeln!(out, r#"    </effect>"#).map_err(|e| e.to_string())?;
    }
    writeln!(out, r#"  </library_effects>"#).map_err(|e| e.to_string())?;

    // 3. Materials
    writeln!(out, r#"  <library_materials>"#).map_err(|e| e.to_string())?;
    for i in 0..submeshes.len() {
        writeln!(out, r#"    <material id="mat_{}" name="Material_{}">"#, i, i).map_err(|e| e.to_string())?;
        writeln!(out, r##"      <instance_effect url="#effect_{}"/>"##, i).map_err(|e| e.to_string())?;
        writeln!(out, r#"    </material>"#).map_err(|e| e.to_string())?;
    }
    writeln!(out, r#"  </library_materials>"#).map_err(|e| e.to_string())?;

    // 4. Geometries (use rest geometry if skinned to preserve bind pose)
    writeln!(out, r#"  <library_geometries>"#).map_err(|e| e.to_string())?;
    for (i, sub) in submeshes.iter().enumerate() {
        let geom_id = format!("geom_{}", i);
        let pos_id = format!("{}_pos", geom_id);
        let norm_id = format!("{}_norm", geom_id);
        let uv_id = format!("{}_uv", geom_id);
        let vert_id = format!("{}_vertices", geom_id);

        let pos_slice = if !sub.rest_positions.is_empty() {
            &sub.rest_positions
        } else {
            &sub.positions
        };

        writeln!(out, r#"    <geometry id="{}" name="SubMesh_{}">"#, geom_id, i).map_err(|e| e.to_string())?;
        writeln!(out, r#"      <mesh>"#).map_err(|e| e.to_string())?;

        // Positions
        let pos_count = pos_slice.len() * 3;
        writeln!(out, r#"        <source id="{}">"#, pos_id).map_err(|e| e.to_string())?;
        write!(out, r#"          <float_array id="{}_array" count="{}">"#, pos_id, pos_count).map_err(|e| e.to_string())?;
        for p in pos_slice {
            write!(out, "{:.6} {:.6} {:.6} ", p[0], p[1], p[2]).map_err(|e| e.to_string())?;
        }
        writeln!(out, "</float_array>").map_err(|e| e.to_string())?;
        writeln!(out, r##"          <technique_common><accessor source="#{}_array" count="{}" stride="3"><param name="X" type="float"/><param name="Y" type="float"/><param name="Z" type="float"/></accessor></technique_common>"##, pos_id, pos_slice.len()).map_err(|e| e.to_string())?;
        writeln!(out, r#"        </source>"#).map_err(|e| e.to_string())?;

        // Normals
        let normals = if !sub.rest_normals.is_empty() {
            sub.rest_normals.clone()
        } else if sub.normals.len() == pos_slice.len() {
            sub.normals.clone()
        } else {
            compute_normals(pos_slice, &sub.indices)
        };
        let norm_count = normals.len() * 3;
        writeln!(out, r#"        <source id="{}">"#, norm_id).map_err(|e| e.to_string())?;
        write!(out, r#"          <float_array id="{}_array" count="{}">"#, norm_id, norm_count).map_err(|e| e.to_string())?;
        for n in &normals {
            write!(out, "{:.6} {:.6} {:.6} ", n[0], n[1], n[2]).map_err(|e| e.to_string())?;
        }
        writeln!(out, "</float_array>").map_err(|e| e.to_string())?;
        writeln!(out, r##"          <technique_common><accessor source="#{}_array" count="{}" stride="3"><param name="X" type="float"/><param name="Y" type="float"/><param name="Z" type="float"/></accessor></technique_common>"##, norm_id, normals.len()).map_err(|e| e.to_string())?;
        writeln!(out, r#"        </source>"#).map_err(|e| e.to_string())?;

        // UVs
        let has_uv = sub.uvs.len() == pos_slice.len() || tex_files[i].is_some();
        if has_uv {
            let uv_count = pos_slice.len() * 2;
            writeln!(out, r#"        <source id="{}">"#, uv_id).map_err(|e| e.to_string())?;
            write!(out, r#"          <float_array id="{}_array" count="{}">"#, uv_id, uv_count).map_err(|e| e.to_string())?;
            for idx in 0..pos_slice.len() {
                let uv = sub.uvs.get(idx).copied().unwrap_or([0.0, 0.0]);
                // Collada T coordinate has origin at bottom
                write!(out, "{:.6} {:.6} ", uv[0], 1.0 - uv[1]).map_err(|e| e.to_string())?;
            }
            writeln!(out, "</float_array>").map_err(|e| e.to_string())?;
            writeln!(out, r##"          <technique_common><accessor source="#{}_array" count="{}" stride="2"><param name="S" type="float"/><param name="T" type="float"/></accessor></technique_common>"##, uv_id, pos_slice.len()).map_err(|e| e.to_string())?;
            writeln!(out, r#"        </source>"#).map_err(|e| e.to_string())?;
        }

        // Vertices
        writeln!(out, r#"        <vertices id="{}">"#, vert_id).map_err(|e| e.to_string())?;
        writeln!(out, r##"          <input semantic="POSITION" source="#{}"/>"##, pos_id).map_err(|e| e.to_string())?;
        writeln!(out, r#"        </vertices>"#).map_err(|e| e.to_string())?;

        // Triangles
        let mat_sym = format!("Mat_{}", i);
        writeln!(out, r#"        <triangles material="{}" count="{}">"#, mat_sym, sub.indices.len()).map_err(|e| e.to_string())?;
        writeln!(out, r##"          <input semantic="VERTEX" source="#{}" offset="0"/>"##, vert_id).map_err(|e| e.to_string())?;
        writeln!(out, r##"          <input semantic="NORMAL" source="#{}" offset="1"/>"##, norm_id).map_err(|e| e.to_string())?;
        if has_uv {
            writeln!(out, r##"          <input semantic="TEXCOORD" source="#{}" offset="2" set="0"/>"##, uv_id).map_err(|e| e.to_string())?;
            write!(out, r#"          <p>"#).map_err(|e| e.to_string())?;
            for tri in &sub.indices {
                write!(out, "{} {} {}  {} {} {}  {} {} {}  ",
                    tri[0], tri[0], tri[0],
                    tri[1], tri[1], tri[1],
                    tri[2], tri[2], tri[2]
                ).map_err(|e| e.to_string())?;
            }
            writeln!(out, "</p>").map_err(|e| e.to_string())?;
        } else {
            write!(out, r#"          <p>"#).map_err(|e| e.to_string())?;
            for tri in &sub.indices {
                write!(out, "{} {}  {} {}  {} {}  ",
                    tri[0], tri[0],
                    tri[1], tri[1],
                    tri[2], tri[2]
                ).map_err(|e| e.to_string())?;
            }
            writeln!(out, "</p>").map_err(|e| e.to_string())?;
        }

        writeln!(out, r#"        </triangles>"#).map_err(|e| e.to_string())?;
        writeln!(out, r#"      </mesh>"#).map_err(|e| e.to_string())?;
        writeln!(out, r#"    </geometry>"#).map_err(|e| e.to_string())?;
    }
    writeln!(out, r#"  </library_geometries>"#).map_err(|e| e.to_string())?;

    // 5. Controllers
    let has_skinning = !mesh.skins.is_empty() && submeshes.iter().any(|s| s.joint_indices.is_some() && s.joint_weights.is_some());
    if has_skinning {
        let skin = &mesh.skins[0];
        writeln!(out, r#"  <library_controllers>"#).map_err(|e| e.to_string())?;
        for (i, sub) in submeshes.iter().enumerate() {
            if let (Some(joint_indices), Some(joint_weights)) = (&sub.joint_indices, &sub.joint_weights) {
                let ctrl_id = format!("skin_{}", i);
                let geom_id = format!("geom_{}", i);
                let joints_id = format!("{}_joints", ctrl_id);
                let poses_id = format!("{}_bind_poses", ctrl_id);
                let weights_id = format!("{}_weights", ctrl_id);

                writeln!(out, r#"    <controller id="{}" name="Skin_{}">"#, ctrl_id, i).map_err(|e| e.to_string())?;
                writeln!(out, r##"      <skin source="#{}">"##, geom_id).map_err(|e| e.to_string())?;
                writeln!(out, r#"        <bind_shape_matrix>1 0 0 0 0 1 0 0 0 0 1 0 0 0 0 1</bind_shape_matrix>"#).map_err(|e| e.to_string())?;

                // Joints source
                writeln!(out, r#"        <source id="{}">"#, joints_id).map_err(|e| e.to_string())?;
                write!(out, r#"          <Name_array id="{}_array" count="{}">"#, joints_id, skin.joints.len()).map_err(|e| e.to_string())?;
                for j in &skin.joints {
                    write!(out, "Node_{} ", j.node_index).map_err(|e| e.to_string())?;
                }
                writeln!(out, "</Name_array>").map_err(|e| e.to_string())?;
                writeln!(out, r##"          <technique_common><accessor source="#{}_array" count="{}" stride="1"><param name="JOINT" type="Name"/></accessor></technique_common>"##, joints_id, skin.joints.len()).map_err(|e| e.to_string())?;
                writeln!(out, r#"        </source>"#).map_err(|e| e.to_string())?;

                // Bind poses source
                writeln!(out, r#"        <source id="{}">"#, poses_id).map_err(|e| e.to_string())?;
                write!(out, r#"          <float_array id="{}_array" count="{}">"#, poses_id, skin.joints.len() * 16).map_err(|e| e.to_string())?;
                for j in &skin.joints {
                    for row in 0..4 {
                        for col in 0..4 {
                            write!(out, "{:.6} ", j.inverse_bind_matrix[col][row]).map_err(|e| e.to_string())?;
                        }
                    }
                }
                writeln!(out, "</float_array>").map_err(|e| e.to_string())?;
                writeln!(out, r##"          <technique_common><accessor source="#{}_array" count="{}" stride="16"><param name="TRANSFORM" type="float4x4"/></accessor></technique_common>"##, poses_id, skin.joints.len()).map_err(|e| e.to_string())?;
                writeln!(out, r#"        </source>"#).map_err(|e| e.to_string())?;

                // Weights source
                let mut weights_list: Vec<f32> = Vec::new();
                let mut vcounts: Vec<usize> = Vec::with_capacity(joint_indices.len());
                let mut v_indices: Vec<(usize, usize)> = Vec::new();

                for (&jis, &jws) in joint_indices.iter().zip(joint_weights.iter()) {
                    let mut count = 0;
                    for k in 0..4 {
                        let w = jws[k];
                        if w > 0.0001 {
                            let j_idx = jis[k] as usize;
                            if j_idx < skin.joints.len() {
                                let w_idx = weights_list.len();
                                weights_list.push(w);
                                v_indices.push((j_idx, w_idx));
                                count += 1;
                            }
                        }
                    }
                    if count == 0 {
                        let w_idx = weights_list.len();
                        weights_list.push(0.0);
                        v_indices.push((0, w_idx));
                        count = 1;
                    }
                    vcounts.push(count);
                }

                writeln!(out, r#"        <source id="{}">"#, weights_id).map_err(|e| e.to_string())?;
                write!(out, r#"          <float_array id="{}_array" count="{}">"#, weights_id, weights_list.len()).map_err(|e| e.to_string())?;
                for &w in &weights_list {
                    write!(out, "{:.6} ", w).map_err(|e| e.to_string())?;
                }
                writeln!(out, "</float_array>").map_err(|e| e.to_string())?;
                writeln!(out, r##"          <technique_common><accessor source="#{}_array" count="{}" stride="1"><param name="WEIGHT" type="float"/></accessor></technique_common>"##, weights_id, weights_list.len()).map_err(|e| e.to_string())?;
                writeln!(out, r#"        </source>"#).map_err(|e| e.to_string())?;

                // Joints element
                writeln!(out, r#"        <joints>"#).map_err(|e| e.to_string())?;
                writeln!(out, r##"          <input semantic="JOINT" source="#{}"/>"##, joints_id).map_err(|e| e.to_string())?;
                writeln!(out, r##"          <input semantic="INV_BIND_MATRIX" source="#{}"/>"##, poses_id).map_err(|e| e.to_string())?;
                writeln!(out, r#"        </joints>"#).map_err(|e| e.to_string())?;

                // Vertex weights
                writeln!(out, r#"        <vertex_weights count="{}">"#, vcounts.len()).map_err(|e| e.to_string())?;
                writeln!(out, r##"          <input semantic="JOINT" source="#{}" offset="0"/>"##, joints_id).map_err(|e| e.to_string())?;
                writeln!(out, r##"          <input semantic="WEIGHT" source="#{}" offset="1"/>"##, weights_id).map_err(|e| e.to_string())?;

                write!(out, r#"          <vcount>"#).map_err(|e| e.to_string())?;
                for &vc in &vcounts {
                    write!(out, "{} ", vc).map_err(|e| e.to_string())?;
                }
                writeln!(out, "</vcount>").map_err(|e| e.to_string())?;

                write!(out, r#"          <v>"#).map_err(|e| e.to_string())?;
                for &(j_idx, w_idx) in &v_indices {
                    write!(out, "{} {} ", j_idx, w_idx).map_err(|e| e.to_string())?;
                }
                writeln!(out, "</v>").map_err(|e| e.to_string())?;

                writeln!(out, r#"        </vertex_weights>"#).map_err(|e| e.to_string())?;
                writeln!(out, r#"      </skin>"#).map_err(|e| e.to_string())?;
                writeln!(out, r#"    </controller>"#).map_err(|e| e.to_string())?;
            }
        }
        writeln!(out, r#"  </library_controllers>"#).map_err(|e| e.to_string())?;
    }

    // 6. Animations
    if !mesh.animations.is_empty() {
        writeln!(out, r#"  <library_animations>"#).map_err(|e| e.to_string())?;
        for (clip_idx, clip) in mesh.animations.iter().enumerate() {
            let clip_name = if clip.name.is_empty() { format!("Animation_{}", clip_idx) } else { clip.name.clone() };
            writeln!(out, r#"    <animation id="anim_clip_{}" name="{}">"#, clip_idx, clip_name).map_err(|e| e.to_string())?;
            for (ch_idx, ch) in clip.channels.iter().enumerate() {
                if ch.timestamps.is_empty() || ch.values.is_empty() {
                    continue;
                }
                let base_id = format!("anim_{}_{}", clip_idx, ch_idx);
                let input_id = format!("{}_input", base_id);
                let output_id = format!("{}_output", base_id);
                let interp_id = format!("{}_interp", base_id);
                let sampler_id = format!("{}_sampler", base_id);

                writeln!(out, r#"      <animation id="{}">"#, base_id).map_err(|e| e.to_string())?;

                // TIME input
                writeln!(out, r#"        <source id="{}">"#, input_id).map_err(|e| e.to_string())?;
                write!(out, r#"          <float_array id="{}_array" count="{}">"#, input_id, ch.timestamps.len()).map_err(|e| e.to_string())?;
                for &t in &ch.timestamps {
                    write!(out, "{:.6} ", t).map_err(|e| e.to_string())?;
                }
                writeln!(out, "</float_array>").map_err(|e| e.to_string())?;
                writeln!(out, r##"          <technique_common><accessor source="#{}_array" count="{}" stride="1"><param name="TIME" type="float"/></accessor></technique_common>"##, input_id, ch.timestamps.len()).map_err(|e| e.to_string())?;
                writeln!(out, r#"        </source>"#).map_err(|e| e.to_string())?;

                // Output values
                let (stride, target_attr, params) = match ch.property {
                    TargetProperty::Translation => (3, "translation", vec!["X", "Y", "Z"]),
                    TargetProperty::Rotation => (4, "rotation", vec!["X", "Y", "Z", "ANGLE"]),
                    TargetProperty::Scale => (3, "scale", vec!["X", "Y", "Z"]),
                    TargetProperty::Weights => continue,
                };

                writeln!(out, r#"        <source id="{}">"#, output_id).map_err(|e| e.to_string())?;
                let num_keys = ch.timestamps.len();
                let val_count = num_keys * stride;
                write!(out, r#"          <float_array id="{}_array" count="{}">"#, output_id, val_count).map_err(|e| e.to_string())?;
                if ch.property == TargetProperty::Rotation {
                    for k in 0..num_keys {
                        let qx = ch.values.get(k * 4).copied().unwrap_or(0.0);
                        let qy = ch.values.get(k * 4 + 1).copied().unwrap_or(0.0);
                        let qz = ch.values.get(k * 4 + 2).copied().unwrap_or(0.0);
                        let qw = ch.values.get(k * 4 + 3).copied().unwrap_or(1.0);
                        let [ax, ay, az, deg] = quat_to_axis_angle([qx, qy, qz, qw]);
                        write!(out, "{:.6} {:.6} {:.6} {:.6} ", ax, ay, az, deg).map_err(|e| e.to_string())?;
                    }
                } else {
                    for &v in &ch.values {
                        write!(out, "{:.6} ", v).map_err(|e| e.to_string())?;
                    }
                }
                writeln!(out, "</float_array>").map_err(|e| e.to_string())?;
                write!(out, r##"          <technique_common><accessor source="#{}_array" count="{}" stride="{}">"##, output_id, num_keys, stride).map_err(|e| e.to_string())?;
                for p in params {
                    write!(out, r#"<param name="{}" type="float"/>"#, p).map_err(|e| e.to_string())?;
                }
                writeln!(out, "</accessor></technique_common>").map_err(|e| e.to_string())?;
                writeln!(out, r#"        </source>"#).map_err(|e| e.to_string())?;

                // Interpolation
                writeln!(out, r#"        <source id="{}">"#, interp_id).map_err(|e| e.to_string())?;
                write!(out, r#"          <Name_array id="{}_array" count="{}">"#, interp_id, num_keys).map_err(|e| e.to_string())?;
                for _ in 0..num_keys {
                    write!(out, "LINEAR ").map_err(|e| e.to_string())?;
                }
                writeln!(out, "</Name_array>").map_err(|e| e.to_string())?;
                writeln!(out, r##"          <technique_common><accessor source="#{}_array" count="{}" stride="1"><param name="INTERPOLATION" type="name"/></accessor></technique_common>"##, interp_id, num_keys).map_err(|e| e.to_string())?;
                writeln!(out, r#"        </source>"#).map_err(|e| e.to_string())?;

                // Sampler
                writeln!(out, r#"        <sampler id="{}">"#, sampler_id).map_err(|e| e.to_string())?;
                writeln!(out, r##"          <input semantic="INPUT" source="#{}"/>"##, input_id).map_err(|e| e.to_string())?;
                writeln!(out, r##"          <input semantic="OUTPUT" source="#{}"/>"##, output_id).map_err(|e| e.to_string())?;
                writeln!(out, r##"          <input semantic="INTERPOLATION" source="#{}"/>"##, interp_id).map_err(|e| e.to_string())?;
                writeln!(out, r#"        </sampler>"#).map_err(|e| e.to_string())?;

                // Channel
                let target_node_id = format!("Node_{}", ch.target_node);
                writeln!(out, r##"        <channel source="#{}" target="{}/{}"/>"##, sampler_id, target_node_id, target_attr).map_err(|e| e.to_string())?;

                writeln!(out, r#"      </animation>"#).map_err(|e| e.to_string())?;
            }
            writeln!(out, r#"    </animation>"#).map_err(|e| e.to_string())?;
        }
        writeln!(out, r#"  </library_animations>"#).map_err(|e| e.to_string())?;
    }

    // 7. Visual Scene
    writeln!(out, r#"  <library_visual_scenes>"#).map_err(|e| e.to_string())?;
    writeln!(out, r#"    <visual_scene id="Scene" name="Scene">"#).map_err(|e| e.to_string())?;

    if !mesh.nodes.is_empty() {
        let mesh_node_idx = mesh.nodes.iter().position(|n| n.mesh_index.is_some()).unwrap_or(0);

        fn write_node<W: Write>(
            out: &mut W,
            node_idx: usize,
            root_idx: usize,
            nodes: &[SceneNode],
            mesh_node_idx: usize,
            submeshes: &[SubMesh],
            has_skinning: bool,
            indent: usize,
        ) -> Result<(), String> {
            let node = &nodes[node_idx];
            let pad = "  ".repeat(indent);
            let name = if node.name.is_empty() { format!("Node_{}", node_idx) } else { node.name.clone() };

            writeln!(out, r#"{}<node id="Node_{}" name="{}" sid="Node_{}" type="NODE">"#, pad, node_idx, name, node_idx).map_err(|e| e.to_string())?;
            let t = node.initial_translation;
            writeln!(out, r#"{}  <translate sid="translation">{:.6} {:.6} {:.6}</translate>"#, pad, t[0], t[1], t[2]).map_err(|e| e.to_string())?;
            let [rx, ry, rz, deg] = quat_to_axis_angle(node.initial_rotation);
            writeln!(out, r#"{}  <rotate sid="rotation">{:.6} {:.6} {:.6} {:.6}</rotate>"#, pad, rx, ry, rz, deg).map_err(|e| e.to_string())?;
            let s = node.initial_scale;
            writeln!(out, r#"{}  <scale sid="scale">{:.6} {:.6} {:.6}</scale>"#, pad, s[0], s[1], s[2]).map_err(|e| e.to_string())?;

            if node_idx == mesh_node_idx {
                for (i, sub) in submeshes.iter().enumerate() {
                    let mat_sym = format!("Mat_{}", i);
                    let mat_id = format!("mat_{}", i);
                    let is_sub_skinned = has_skinning && sub.joint_indices.is_some() && sub.joint_weights.is_some();

                    if is_sub_skinned {
                        let ctrl_id = format!("skin_{}", i);
                        let root_id = format!("Node_{}", root_idx);
                        writeln!(out, r##"{}  <instance_controller url="#{}">"##, pad, ctrl_id).map_err(|e| e.to_string())?;
                        writeln!(out, r##"{}    <skeleton>#{}</skeleton>"##, pad, root_id).map_err(|e| e.to_string())?;
                        writeln!(out, r#"{}    <bind_material>"#, pad).map_err(|e| e.to_string())?;
                        writeln!(out, r#"{}      <technique_common>"#, pad).map_err(|e| e.to_string())?;
                        writeln!(out, r##"{}        <instance_material symbol="{}" target="#{}"/>"##, pad, mat_sym, mat_id).map_err(|e| e.to_string())?;
                        writeln!(out, r#"{}      </technique_common>"#, pad).map_err(|e| e.to_string())?;
                        writeln!(out, r#"{}    </bind_material>"#, pad).map_err(|e| e.to_string())?;
                        writeln!(out, r#"{}  </instance_controller>"#, pad).map_err(|e| e.to_string())?;
                    } else {
                        let geom_id = format!("geom_{}", i);
                        writeln!(out, r##"{}  <instance_geometry url="#{}">"##, pad, geom_id).map_err(|e| e.to_string())?;
                        writeln!(out, r#"{}    <bind_material>"#, pad).map_err(|e| e.to_string())?;
                        writeln!(out, r#"{}      <technique_common>"#, pad).map_err(|e| e.to_string())?;
                        writeln!(out, r##"{}        <instance_material symbol="{}" target="#{}"/>"##, pad, mat_sym, mat_id).map_err(|e| e.to_string())?;
                        writeln!(out, r#"{}      </technique_common>"#, pad).map_err(|e| e.to_string())?;
                        writeln!(out, r#"{}    </bind_material>"#, pad).map_err(|e| e.to_string())?;
                        writeln!(out, r#"{}  </instance_geometry>"#, pad).map_err(|e| e.to_string())?;
                    }
                }
            }

            for &child_idx in &node.children {
                if child_idx < nodes.len() {
                    write_node(out, child_idx, root_idx, nodes, mesh_node_idx, submeshes, has_skinning, indent + 1)?;
                }
            }

            writeln!(out, r#"{}</node>"#, pad).map_err(|e| e.to_string())?;
            Ok(())
        }

        let roots: Vec<usize> = mesh.nodes.iter().enumerate()
            .filter_map(|(i, n)| if n.parent.is_none() { Some(i) } else { None })
            .collect();

        let root_list = if roots.is_empty() { vec![0] } else { roots };
        for root_idx in root_list {
            write_node(&mut out, root_idx, root_idx, &mesh.nodes, mesh_node_idx, submeshes, has_skinning, 3)?;
        }
    } else {
        for i in 0..submeshes.len() {
            let geom_id = format!("geom_{}", i);
            let mat_sym = format!("Mat_{}", i);
            let mat_id = format!("mat_{}", i);

            writeln!(out, r#"      <node id="Node_{}" name="SubMesh_{}" type="NODE">"#, i, i).map_err(|e| e.to_string())?;
            writeln!(out, r##"        <instance_geometry url="#{}">"##, geom_id).map_err(|e| e.to_string())?;
            writeln!(out, r#"          <bind_material>"#).map_err(|e| e.to_string())?;
            writeln!(out, r#"            <technique_common>"#).map_err(|e| e.to_string())?;
            writeln!(out, r##"              <instance_material symbol="{}" target="#{}"/>"##, mat_sym, mat_id).map_err(|e| e.to_string())?;
            writeln!(out, r#"            </technique_common>"#).map_err(|e| e.to_string())?;
            writeln!(out, r#"          </bind_material>"#).map_err(|e| e.to_string())?;
            writeln!(out, r#"        </instance_geometry>"#).map_err(|e| e.to_string())?;
            writeln!(out, r#"      </node>"#).map_err(|e| e.to_string())?;
        }
    }

    writeln!(out, r#"    </visual_scene>"#).map_err(|e| e.to_string())?;
    writeln!(out, r#"  </library_visual_scenes>"#).map_err(|e| e.to_string())?;

    // 8. Scene instance
    writeln!(out, r#"  <scene>"#).map_err(|e| e.to_string())?;
    writeln!(out, r##"    <instance_visual_scene url="#Scene"/>"##).map_err(|e| e.to_string())?;
    writeln!(out, r#"  </scene>"#).map_err(|e| e.to_string())?;

    writeln!(out, r#"</COLLADA>"#).map_err(|e| e.to_string())?;

    println!("[COLLADA Format] Successfully exported {} submeshes to '{}'", submeshes.len(), path.display());
    Ok(())
}
