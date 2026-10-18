use crate::loader::{
    compute_normals, load_texture_from_path, resolve_relative_file,
    FileFormatInfo, FormatPlugin, MeshData, SubMesh, TextureData,
};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::Path;
use std::sync::Arc;

pub struct ColladaLoader {}

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
    let mut current_tag = Vec::new();

    // Intermediate primitive storage
    struct PrimGroup {
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
                    b"float_array" | b"geometry" => {
                        for attr in e.attributes().with_checks(false).flatten() {
                            if attr.key.into_inner() == b"id" {
                                if let Ok(val) = attr.unescape_value() {
                                    current_id = val.to_string();
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
                        } else if let Some(ref mut prim) = current_prim {
                            prim.stride = prim.stride.max(offset + 1);
                            match semantic.as_str() {
                                "VERTEX" => {
                                    prim.pos_source = source;
                                    prim.pos_offset = offset;
                                }
                                "POSITION" => {
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
                    b"init_from" => {
                        let val = text.trim().to_string();
                        if !cur_image_id.is_empty() {
                            image_files.insert(cur_image_id.clone(), val.clone());
                        }
                        if !cur_newparam_sid.is_empty() {
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
                        if let Some(ref mut prim) = current_prim {
                            let counts: Vec<usize> = text
                                .split_ascii_whitespace()
                                .filter_map(|s| s.parse::<usize>().ok())
                                .collect();
                            prim.vcounts.extend(counts);
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
                    _ => {}
                }
            }
            Ok(Event::End(ref e)) => {
                let name = e.name().into_inner();
                if name == b"triangles" || name == b"polylist" {
                    if let Some(prim) = current_prim.take() {
                        prim_groups.push(prim);
                    }
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

    let mut submeshes = Vec::new();
    let is_y_up = up_axis.starts_with('Y');

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

        let stride = prim.stride;
        let mut positions = Vec::new();
        let mut normals = Vec::new();
        let mut uvs = Vec::new();
        let mut indices = Vec::new();

        // Helper to extract one vertex tuple
        let get_vertex = |tuple_idx: usize| -> (usize, [f32; 3], [f32; 3], [f32; 2]) {
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

            let n = if let (Some(no), Some(n_arr)) = (prim.norm_offset, norm_array) {
                let ni = prim.indices.get(base + no).copied().unwrap_or(0) as usize;
                if ni * 3 + 2 < n_arr.len() {
                    let n_raw = [n_arr[ni * 3], n_arr[ni * 3 + 1], n_arr[ni * 3 + 2]];
                    if is_y_up {
                        [n_raw[0], -n_raw[2], n_raw[1]]
                    } else {
                        n_raw
                    }
                } else {
                    [0.0, 0.0, 1.0]
                }
            } else {
                [0.0, 0.0, 1.0]
            };

            let uv = if let (Some(uo), Some(uv_arr)) = (prim.uv_offset, uv_array) {
                let ui = prim.indices.get(base + uo).copied().unwrap_or(0) as usize;
                if ui * 2 + 1 < uv_arr.len() {
                    [uv_arr[ui * 2], uv_arr[ui * 2 + 1]]
                } else {
                    [0.0, 0.0]
                }
            } else {
                [0.0, 0.0]
            };

            (pi, p, n, uv)
        };

        // Triangulate
        if prim.vcounts.is_empty() {
            // Pure triangles: 3 tuples per triangle
            let num_tris = (prim.indices.len() / stride) / 3;
            for t in 0..num_tris {
                let i0 = t * 3;
                let i1 = t * 3 + 1;
                let i2 = t * 3 + 2;

                let (_, p0, n0, uv0) = get_vertex(i0);
                let (_, p1, n1, uv1) = get_vertex(i1);
                let (_, p2, n2, uv2) = get_vertex(i2);

                let base_idx = positions.len() as u32;
                positions.push(p0);
                positions.push(p1);
                positions.push(p2);

                normals.push(n0);
                normals.push(n1);
                normals.push(n2);

                uvs.push(uv0);
                uvs.push(uv1);
                uvs.push(uv2);

                indices.push([base_idx, base_idx + 1, base_idx + 2]);
            }
        } else {
            // Polylist with vcounts
            let mut tuple_offset = 0;
            for vc in prim.vcounts {
                if vc >= 3 {
                    let (_, p0, n0, uv0) = get_vertex(tuple_offset);
                    let start_idx = positions.len() as u32;
                    positions.push(p0);
                    normals.push(n0);
                    uvs.push(uv0);

                    for k in 1..vc {
                        let (_, pk, nk, uvk) = get_vertex(tuple_offset + k);
                        positions.push(pk);
                        normals.push(nk);
                        uvs.push(uvk);
                    }

                    // Fan triangulation: (0, k, k+1)
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
            }

            submeshes.push(SubMesh {
                positions,
                normals,
                uvs,
                indices,
                color: mat_color,
                texture: mat_texture,
            });
        }
    }

    if submeshes.is_empty() {
        return Err("No valid geometry decoded from COLLADA".into());
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

    Ok(MeshData {
        positions: all_positions,
        normals: all_normals,
        uvs: all_uvs,
        indices: all_indices,
        submeshes,
        source_path: None,
        has_animation: true,
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
        }];
        &fallback_submesh
    } else {
        return Err("Mesh contains no geometry to export".into());
    };

    let mut out = std::fs::File::create(path).map_err(|e| format!("create file: {}", e))?;

    writeln!(out, r#"<?xml version="1.0" encoding="utf-8"?>"#).map_err(|e| e.to_string())?;
    writeln!(out, r#"<COLLADA xmlns="http://www.collada.org/2005/11/COLLADASchema" version="1.4.1">"#).map_err(|e| e.to_string())?;
    writeln!(out, r#"  <asset>"#).map_err(|e| e.to_string())?;
    writeln!(out, r#"    <contributor><authoring_tool>Mesh 3D Viewer</authoring_tool></contributor>"#).map_err(|e| e.to_string())?;
    writeln!(out, r#"    <unit name="meter" meter="1"/>"#).map_err(|e| e.to_string())?;
    writeln!(out, r#"    <up_axis>Z_UP</up_axis>"#).map_err(|e| e.to_string())?;
    writeln!(out, r#"  </asset>"#).map_err(|e| e.to_string())?;

    // 1. Effects
    writeln!(out, r#"  <library_effects>"#).map_err(|e| e.to_string())?;
    for (i, sub) in submeshes.iter().enumerate() {
        let (r, g, b, a) = if let Some(c) = sub.color {
            (c[0], c[1], c[2], c[3])
        } else {
            (0.8, 0.8, 0.8, 1.0)
        };
        writeln!(out, r#"    <effect id="effect_{}">"#, i).map_err(|e| e.to_string())?;
        writeln!(out, r#"      <profile_COMMON>"#).map_err(|e| e.to_string())?;
        writeln!(out, r#"        <technique sid="common">"#).map_err(|e| e.to_string())?;
        writeln!(out, r#"          <lambert>"#).map_err(|e| e.to_string())?;
        writeln!(out, r#"            <diffuse><color>{:.6} {:.6} {:.6} {:.6}</color></diffuse>"#, r, g, b, a).map_err(|e| e.to_string())?;
        writeln!(out, r#"          </lambert>"#).map_err(|e| e.to_string())?;
        writeln!(out, r#"        </technique>"#).map_err(|e| e.to_string())?;
        writeln!(out, r#"      </profile_COMMON>"#).map_err(|e| e.to_string())?;
        writeln!(out, r#"    </effect>"#).map_err(|e| e.to_string())?;
    }
    writeln!(out, r#"  </library_effects>"#).map_err(|e| e.to_string())?;

    // 2. Materials
    writeln!(out, r#"  <library_materials>"#).map_err(|e| e.to_string())?;
    for i in 0..submeshes.len() {
        writeln!(out, r#"    <material id="mat_{}" name="Material_{}">"#, i, i).map_err(|e| e.to_string())?;
        writeln!(out, r##"      <instance_effect url="#effect_{}"/>"##, i).map_err(|e| e.to_string())?;
        writeln!(out, r#"    </material>"#).map_err(|e| e.to_string())?;
    }
    writeln!(out, r#"  </library_materials>"#).map_err(|e| e.to_string())?;

    // 3. Geometries
    writeln!(out, r#"  <library_geometries>"#).map_err(|e| e.to_string())?;
    for (i, sub) in submeshes.iter().enumerate() {
        let geom_id = format!("geom_{}", i);
        let pos_id = format!("{}_pos", geom_id);
        let norm_id = format!("{}_norm", geom_id);
        let uv_id = format!("{}_uv", geom_id);
        let vert_id = format!("{}_vertices", geom_id);

        writeln!(out, r#"    <geometry id="{}" name="SubMesh_{}">"#, geom_id, i).map_err(|e| e.to_string())?;
        writeln!(out, r#"      <mesh>"#).map_err(|e| e.to_string())?;

        // Positions
        let pos_count = sub.positions.len() * 3;
        writeln!(out, r#"        <source id="{}">"#, pos_id).map_err(|e| e.to_string())?;
        write!(out, r#"          <float_array id="{}_array" count="{}">"#, pos_id, pos_count).map_err(|e| e.to_string())?;
        for p in &sub.positions {
            write!(out, "{:.6} {:.6} {:.6} ", p[0], p[1], p[2]).map_err(|e| e.to_string())?;
        }
        writeln!(out, "</float_array>").map_err(|e| e.to_string())?;
        writeln!(out, r##"          <technique_common><accessor source="#{}_array" count="{}" stride="3"><param name="X" type="float"/><param name="Y" type="float"/><param name="Z" type="float"/></accessor></technique_common>"##, pos_id, sub.positions.len()).map_err(|e| e.to_string())?;
        writeln!(out, r#"        </source>"#).map_err(|e| e.to_string())?;

        // Normals
        let normals = if sub.normals.len() == sub.positions.len() {
            sub.normals.clone()
        } else {
            compute_normals(&sub.positions, &sub.indices)
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

        // UVs if available
        let has_uv = sub.uvs.len() == sub.positions.len();
        if has_uv {
            let uv_count = sub.uvs.len() * 2;
            writeln!(out, r#"        <source id="{}">"#, uv_id).map_err(|e| e.to_string())?;
            write!(out, r#"          <float_array id="{}_array" count="{}">"#, uv_id, uv_count).map_err(|e| e.to_string())?;
            for uv in &sub.uvs {
                write!(out, "{:.6} {:.6} ", uv[0], uv[1]).map_err(|e| e.to_string())?;
            }
            writeln!(out, "</float_array>").map_err(|e| e.to_string())?;
            writeln!(out, r##"          <technique_common><accessor source="#{}_array" count="{}" stride="2"><param name="S" type="float"/><param name="T" type="float"/></accessor></technique_common>"##, uv_id, sub.uvs.len()).map_err(|e| e.to_string())?;
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

    // 4. Visual Scene
    writeln!(out, r#"  <library_visual_scenes>"#).map_err(|e| e.to_string())?;
    writeln!(out, r#"    <visual_scene id="Scene" name="Scene">"#).map_err(|e| e.to_string())?;
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
    writeln!(out, r#"    </visual_scene>"#).map_err(|e| e.to_string())?;
    writeln!(out, r#"  </library_visual_scenes>"#).map_err(|e| e.to_string())?;

    // 5. Scene instance
    writeln!(out, r#"  <scene>"#).map_err(|e| e.to_string())?;
    writeln!(out, r##"    <instance_visual_scene url="#Scene"/>"##).map_err(|e| e.to_string())?;
    writeln!(out, r#"  </scene>"#).map_err(|e| e.to_string())?;

    writeln!(out, r#"</COLLADA>"#).map_err(|e| e.to_string())?;

    println!("[COLLADA Format] Successfully exported {} submeshes to '{}'", submeshes.len(), path.display());
    Ok(())
}
