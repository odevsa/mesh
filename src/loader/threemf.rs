use super::{Loader, MeshData, SubMesh};
use std::path::Path;
use std::io::Read;
use std::collections::HashMap;

pub struct ThreemfLoader {}

fn parse_hex_color(s: &str) -> Option<[f32; 4]> {
    let s = s.trim().trim_start_matches('#');
    let (r, g, b, a) = if s.len() == 6 {
        let r = crate::render::renderer::srgb_to_linear(u8::from_str_radix(&s[0..2], 16).ok()? as f32 / 255.0);
        let g = crate::render::renderer::srgb_to_linear(u8::from_str_radix(&s[2..4], 16).ok()? as f32 / 255.0);
        let b = crate::render::renderer::srgb_to_linear(u8::from_str_radix(&s[4..6], 16).ok()? as f32 / 255.0);
        (r, g, b, 1.0)
    } else if s.len() == 8 {
        let r = crate::render::renderer::srgb_to_linear(u8::from_str_radix(&s[0..2], 16).ok()? as f32 / 255.0);
        let g = crate::render::renderer::srgb_to_linear(u8::from_str_radix(&s[2..4], 16).ok()? as f32 / 255.0);
        let b = crate::render::renderer::srgb_to_linear(u8::from_str_radix(&s[4..6], 16).ok()? as f32 / 255.0);
        let a = u8::from_str_radix(&s[6..8], 16).ok()? as f32 / 255.0;
        (r, g, b, a)
    } else {
        return None;
    };

    Some([r, g, b, a])
}

fn to_hex_color(c: [f32; 4]) -> String {
    let to_srgb = |v: f32| -> u8 {
        let v = v.clamp(0.0, 1.0);
        let s = if v <= 0.0031308 {
            v * 12.92
        } else {
            1.055 * v.powf(1.0 / 2.4) - 0.055
        };
        (s.clamp(0.0, 1.0) * 255.0).round() as u8
    };
    let r = to_srgb(c[0]);
    let g = to_srgb(c[1]);
    let b = to_srgb(c[2]);
    let a = (c[3].clamp(0.0, 1.0) * 255.0).round() as u8;
    format!("#{:02X}{:02X}{:02X}{:02X}", r, g, b, a)
}

fn to_hex_color_rgb(c: [f32; 4]) -> String {
    let to_srgb = |v: f32| -> u8 {
        let v = v.clamp(0.0, 1.0);
        let s = if v <= 0.0031308 {
            v * 12.92
        } else {
            1.055 * v.powf(1.0 / 2.4) - 0.055
        };
        (s.clamp(0.0, 1.0) * 255.0).round() as u8
    };
    let r = to_srgb(c[0]);
    let g = to_srgb(c[1]);
    let b = to_srgb(c[2]);
    format!("#{:02X}{:02X}{:02X}", r, g, b)
}

fn orca_filament_code(idx: usize) -> &'static str {
    match idx {
        0 => "4",
        1 => "8",
        2 => "0C",
        3 => "1C",
        4 => "2C",
        5 => "3C",
        6 => "4C",
        7 => "5C",
        8 => "6C",
        9 => "7C",
        10 => "8C",
        11 => "9C",
        12 => "AC",
        13 => "BC",
        14 => "CC",
        15 => "DC",
        16 => "EC",
        _ => "4",
    }
}

fn resolve_paint_color(s: &str, filament_colors: &[[f32; 4]]) -> Option<[f32; 4]> {
    let s = s.trim();
    if s.is_empty() { return None; }

    if let Some(col) = parse_hex_color(s) {
        return Some(col);
    }

    if filament_colors.is_empty() {
        return None;
    }

    let idx = match s {
        "4" => Some(0),        // Pupil (black, filament index 0)
        "8" => Some(1),        // Body / Skin (brown, filament index 1)
        "0C" | "0" => Some(2), // Eye / Sclera (white, filament index 2)
        "1C" => Some(3),       // Ear (pink, filament index 3)
        _ => {
            let s_clean = s.trim_end_matches('C').trim_end_matches('c');
            if let Ok(val) = u32::from_str_radix(s_clean, 16) {
                if (val as usize) < filament_colors.len() {
                    Some(val as usize)
                } else if ((val >> 2) as usize) < filament_colors.len() {
                    Some((val >> 2) as usize)
                } else {
                    Some((val as usize) % filament_colors.len())
                }
            } else {
                None
            }
        }
    };

    if let Some(i) = idx {
        if let Some(&col) = filament_colors.get(i) {
            return Some(col);
        }
    }

    None
}

impl Loader for ThreemfLoader {
    fn name(&self) -> &str { "3mf" }

    fn can_load(&self, path: &Path, header: &[u8]) -> bool {
        if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
            if ext.eq_ignore_ascii_case("3mf") { return true }
        }
        if header.starts_with(b"PK") { return true }
        false
    }

    fn load(&self, _path: &Path, reader: Box<dyn Read>) -> Result<MeshData, String> {
        let mut file_buf = Vec::new();
        let mut r = std::io::BufReader::new(reader);
        r.read_to_end(&mut file_buf).map_err(|e| format!("read: {}", e))?;
        let cursor = std::io::Cursor::new(file_buf);

        let mut archive = zip::ZipArchive::new(cursor).map_err(|e| format!("zip open: {}", e))?;

        let mut model_files: Vec<Vec<u8>> = Vec::new();
        let mut config_files: Vec<Vec<u8>> = Vec::new();

        for i in 0..archive.len() {
            if let Ok(mut file) = archive.by_index(i) {
                let name = file.name().to_string();
                let lname = name.to_lowercase();
                if lname.ends_with(".model") {
                    let mut v = Vec::new();
                    if file.read_to_end(&mut v).is_ok() {
                        if lname.ends_with("3dmodel.model") {
                            model_files.insert(0, v);
                        } else {
                            model_files.push(v);
                        }
                    }
                } else if lname.contains("project_settings.config") || lname.contains("model_settings.config") {
                    let mut v = Vec::new();
                    if file.read_to_end(&mut v).is_ok() {
                        config_files.push(v);
                    }
                }
            }
        }

        if model_files.is_empty() {
            return Err("3mf: no .model file found in archive".into());
        }

        let mut filament_colors: Vec<[f32; 4]> = Vec::new();
        for cfg_bytes in &config_files {
            if let Ok(txt) = std::str::from_utf8(cfg_bytes) {
                if let Some(idx) = txt.find("\"filament_colour\":") {
                    let slice = &txt[idx..];
                    if let Some(arr_start) = slice.find('[') {
                        if let Some(arr_end) = slice[arr_start..].find(']') {
                            let array_str = &slice[arr_start..arr_start + arr_end + 1];
                            for line in array_str.lines() {
                                let line_clean = line.trim().trim_matches(',').trim_matches('"');
                                if let Some(col) = parse_hex_color(line_clean) {
                                    filament_colors.push(col);
                                }
                            }
                        }
                    }
                }
            }
        }

        let mut resource_colors: HashMap<u32, Vec<[f32; 4]>> = HashMap::new();

        use quick_xml::events::Event;

        for model_bytes in &model_files {
            let mut reader = quick_xml::Reader::from_reader(model_bytes.as_slice());
            reader.trim_text(true);
            let mut current_resource_id: Option<u32> = None;

            loop {
                match reader.read_event() {
                    Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                        let local_name = e.name();
                        let name = local_name.into_inner();

                        if name.ends_with(b"basematerials") || name.ends_with(b"colorgroup") {
                            for attr in e.attributes().with_checks(false).flatten() {
                                let key = attr.key.into_inner();
                                if key == b"id" || key.ends_with(b":id") {
                                    if let Ok(val) = attr.unescape_value() {
                                        current_resource_id = val.parse::<u32>().ok();
                                    }
                                }
                            }
                        } else if name.ends_with(b"base") || name.ends_with(b"color") {
                            let mut color_str: Option<String> = None;
                            for attr in e.attributes().with_checks(false).flatten() {
                                let k = attr.key.into_inner();
                                if k.ends_with(b"displaycolor") || k.ends_with(b"color") {
                                    if let Ok(val) = attr.unescape_value() {
                                        color_str = Some(val.to_string());
                                    }
                                }
                            }
                            if let (Some(res_id), Some(cstr)) = (current_resource_id, color_str) {
                                if let Some(col) = parse_hex_color(&cstr) {
                                    resource_colors.entry(res_id).or_default().push(col);
                                }
                            }
                        }
                    }
                    Ok(Event::End(ref e)) => {
                        let local_name = e.name();
                        let name = local_name.into_inner();
                        if name.ends_with(b"basematerials") || name.ends_with(b"colorgroup") {
                            current_resource_id = None;
                        }
                    }
                    Ok(Event::Eof) => break,
                    _ => {}
                }
            }
        }

        let mut global_positions: Vec<[f32; 3]> = Vec::new();
        let mut global_indices: Vec<[u32; 3]> = Vec::new();
        let mut submeshes: Vec<SubMesh> = Vec::new();

        for model_bytes in &model_files {
            let mut reader = quick_xml::Reader::from_reader(model_bytes.as_slice());
            reader.trim_text(true);

            let mut current_object_pid: Option<u32> = None;
            let mut current_object_pindex: Option<u32> = None;
            let mut current_obj_positions: Vec<[f32; 3]> = Vec::new();
            let mut current_obj_tris: HashMap<Option<[u8; 4]>, Vec<[u32; 3]>> = HashMap::new();

            loop {
                match reader.read_event() {
                    Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                        let local_name = e.name();
                        let name = local_name.into_inner();

                        if name.ends_with(b"object") {
                            current_object_pid = None;
                            current_object_pindex = None;
                            current_obj_positions.clear();
                            current_obj_tris.clear();

                            for attr in e.attributes().with_checks(false).flatten() {
                                let k = attr.key.into_inner();
                                if let Ok(val) = attr.unescape_value() {
                                    if k.ends_with(b"pid") {
                                        current_object_pid = val.parse::<u32>().ok();
                                    } else if k.ends_with(b"pindex") || k.ends_with(b"p1") {
                                        current_object_pindex = val.parse::<u32>().ok();
                                    }
                                }
                            }
                        } else if name.ends_with(b"vertex") {
                            let mut x = None::<f32>;
                            let mut y = None::<f32>;
                            let mut z = None::<f32>;
                            for attr in e.attributes().with_checks(false).flatten() {
                                let k = attr.key.into_inner();
                                if let Ok(val) = attr.unescape_value() {
                                    if k.ends_with(b"x") {
                                        x = val.parse::<f32>().ok();
                                    } else if k.ends_with(b"y") {
                                        y = val.parse::<f32>().ok();
                                    } else if k.ends_with(b"z") {
                                        z = val.parse::<f32>().ok();
                                    }
                                }
                            }
                            if let (Some(x), Some(y), Some(z)) = (x, y, z) {
                                current_obj_positions.push([x, y, z]);
                            }
                        } else if name.ends_with(b"triangle") {
                            let mut v1 = None::<u32>;
                            let mut v2 = None::<u32>;
                            let mut v3 = None::<u32>;
                            let mut tri_pid = current_object_pid;
                            let mut tri_pindex = current_object_pindex;
                            let mut paint_color_str: Option<String> = None;

                            for attr in e.attributes().with_checks(false).flatten() {
                                let k = attr.key.into_inner();
                                if let Ok(val) = attr.unescape_value() {
                                    if k.ends_with(b"v1") {
                                        v1 = val.parse::<u32>().ok();
                                    } else if k.ends_with(b"v2") {
                                        v2 = val.parse::<u32>().ok();
                                    } else if k.ends_with(b"v3") {
                                        v3 = val.parse::<u32>().ok();
                                    } else if k.ends_with(b"pid") {
                                        tri_pid = val.parse::<u32>().ok();
                                    } else if k.ends_with(b"p1") || k.ends_with(b"pindex") {
                                        tri_pindex = val.parse::<u32>().ok();
                                    } else if k.ends_with(b"paint_color") || k.ends_with(b"segment") {
                                        paint_color_str = Some(val.to_string());
                                    }
                                }
                            }

                            if let (Some(a), Some(b), Some(c)) = (v1, v2, v3) {
                                let mut color_f32 = None;

                                if let Some(ref pc_str) = paint_color_str {
                                    color_f32 = resolve_paint_color(pc_str, &filament_colors);
                                }

                                if color_f32.is_none() {
                                    if let Some(pid) = tri_pid {
                                        let pidx = tri_pindex.unwrap_or(0) as usize;
                                        color_f32 = resource_colors.get(&pid).and_then(|list| list.get(pidx).copied());
                                    }
                                }

                                let color_key = color_f32.map(|c| [
                                    (c[0].clamp(0.0, 1.0) * 255.0) as u8,
                                    (c[1].clamp(0.0, 1.0) * 255.0) as u8,
                                    (c[2].clamp(0.0, 1.0) * 255.0) as u8,
                                    (c[3].clamp(0.0, 1.0) * 255.0) as u8,
                                ]);
                                current_obj_tris.entry(color_key).or_default().push([a, b, c]);
                            }
                        }
                    }
                    Ok(Event::End(ref e)) => {
                        let local_name = e.name();
                        let name = local_name.into_inner();
                        if name.ends_with(b"object") {
                            if !current_obj_positions.is_empty() && !current_obj_tris.is_empty() {
                                let base_vertex = global_positions.len() as u32;

                                for p in &current_obj_positions {
                                    global_positions.push(*p);
                                }

                                let all_obj_tris: Vec<[u32; 3]> = current_obj_tris.values().flatten().copied().collect();
                                let obj_normals = compute_normals(&current_obj_positions, &all_obj_tris);

                                for (color_key, sub_indices) in current_obj_tris.drain() {
                                    for tri in &sub_indices {
                                        global_indices.push([tri[0] + base_vertex, tri[1] + base_vertex, tri[2] + base_vertex]);
                                    }

                                    let color = color_key.map(|c| [
                                        c[0] as f32 / 255.0,
                                        c[1] as f32 / 255.0,
                                        c[2] as f32 / 255.0,
                                        c[3] as f32 / 255.0,
                                    ]);

                                    let mut local_positions = Vec::with_capacity(sub_indices.len() * 3);
                                    let mut local_normals = Vec::with_capacity(sub_indices.len() * 3);
                                    let mut local_indices = Vec::with_capacity(sub_indices.len());
                                    let mut v_map = HashMap::new();

                                    for tri in &sub_indices {
                                        let mut new_tri = [0u32; 3];
                                        for (k, &v_idx) in tri.iter().enumerate() {
                                            let new_idx = *v_map.entry(v_idx).or_insert_with(|| {
                                                let idx = local_positions.len() as u32;
                                                local_positions.push(current_obj_positions[v_idx as usize]);
                                                local_normals.push(obj_normals[v_idx as usize]);
                                                idx
                                            });
                                            new_tri[k] = new_idx;
                                        }
                                        local_indices.push(new_tri);
                                    }

                                    submeshes.push(SubMesh {
                                        positions: local_positions,
                                        normals: local_normals,
                                        indices: local_indices,
                                        color,
                                        ..Default::default()
                                    });
                                }
                            }
                        }
                    }
                    Ok(Event::Eof) => break,
                    _ => {}
                }
            }
        }

        if global_positions.is_empty() || global_indices.is_empty() {
            return Err("3mf: no geometry (vertices/triangles) found".into());
        }

        let global_normals = compute_normals(&global_positions, &global_indices);

        Ok(MeshData {
            positions: global_positions,
            normals: global_normals,
            indices: global_indices,
            submeshes,
            ..Default::default()
        })
    }

    fn can_export(&self, path: &Path) -> bool {
        if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
            if ext.eq_ignore_ascii_case("3mf") {
                return true;
            }
        }
        false
    }

    fn export(&self, path: &Path, mesh: &MeshData) -> Result<(), String> {
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

        let file = std::fs::File::create(path).map_err(|e| format!("create 3mf file failed: {}", e))?;
        let mut zip = zip::ZipWriter::new(file);

        let options = zip::write::FileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);

        use std::io::Write;

        // 1. [Content_Types].xml
        zip.start_file("[Content_Types].xml", options).map_err(|e| format!("zip err: {}", e))?;
        let content_types = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\">\n  <Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/>\n  <Default Extension=\"model\" ContentType=\"application/vnd.ms-package.3dmanufacturing-3dmodel+xml\"/>\n  <Default Extension=\"config\" ContentType=\"application/xml\"/>\n</Types>";
        zip.write_all(content_types.as_bytes()).map_err(|e| format!("write: {}", e))?;

        // 2. _rels/.rels
        zip.start_file("_rels/.rels", options).map_err(|e| format!("zip err: {}", e))?;
        let rels = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">\n  <Relationship Target=\"/3D/3dmodel.model\" Id=\"rel0\" Type=\"http://schemas.microsoft.com/3dmanufacturing/2013/01/3dmodel\"/>\n</Relationships>";
        zip.write_all(rels.as_bytes()).map_err(|e| format!("write: {}", e))?;

        // 3. Metadata/project_settings.config (for OrcaSlicer / BambuStudio color palette)
        zip.start_file("Metadata/project_settings.config", options).map_err(|e| format!("zip err: {}", e))?;
        let mut filament_colors_json = String::from("{\n  \"filament_colour\": [");
        for (idx, sub) in submeshes.iter().enumerate() {
            let color = sub.color.unwrap_or([0.8, 0.8, 0.8, 1.0]);
            let hex = to_hex_color_rgb(color);
            if idx > 0 {
                filament_colors_json.push_str(", ");
            }
            filament_colors_json.push_str(&format!("\"{}\"", hex));
        }
        filament_colors_json.push_str("],\n  \"default_filament_colour\": [");
        for (idx, sub) in submeshes.iter().enumerate() {
            let color = sub.color.unwrap_or([0.8, 0.8, 0.8, 1.0]);
            let hex = to_hex_color_rgb(color);
            if idx > 0 {
                filament_colors_json.push_str(", ");
            }
            filament_colors_json.push_str(&format!("\"{}\"", hex));
        }
        filament_colors_json.push_str("]\n}");
        zip.write_all(filament_colors_json.as_bytes()).map_err(|e| format!("write: {}", e))?;

        // 4. Metadata/model_settings.config (for OrcaSlicer / BambuStudio part configuration)
        zip.start_file("Metadata/model_settings.config", options).map_err(|e| format!("zip err: {}", e))?;
        let model_settings = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<config>\n  <object id=\"1\">\n    <metadata key=\"name\" value=\"Model\"/>\n    <metadata key=\"extruder\" value=\"1\"/>\n    <part id=\"1\" subtype=\"normal_part\">\n      <metadata key=\"name\" value=\"Model\"/>\n      <metadata key=\"matrix\" value=\"1 0 0 0 0 1 0 0 0 0 1 0 0 0 0 1\"/>\n    </part>\n  </object>\n</config>";
        zip.write_all(model_settings.as_bytes()).map_err(|e| format!("write: {}", e))?;

        // 5. 3D/3dmodel.model
        zip.start_file("3D/3dmodel.model", options).map_err(|e| format!("zip err: {}", e))?;

        let mut total_tris = 0usize;
        let mut total_verts = 0usize;
        for sub in submeshes {
            total_verts += sub.positions.len();
            total_tris += sub.indices.len();
        }

        let mut model_xml = String::with_capacity(total_verts * 60 + total_tris * 85 + 1500);
        model_xml.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<model unit=\"meter\" xml:lang=\"en-US\" xmlns=\"http://schemas.microsoft.com/3dmanufacturing/core/2015/02\" xmlns:m=\"http://schemas.microsoft.com/3dmanufacturing/material/2015/02\">\n  <metadata name=\"Application\">Mesh</metadata>\n  <resources>\n");

        // Write BaseMaterials resource (id="2") linked to PBR display properties (id="4")
        model_xml.push_str("    <basematerials id=\"2\" displaypropertiesid=\"4\">\n");
        for (idx, sub) in submeshes.iter().enumerate() {
            let color = sub.color.unwrap_or([0.8, 0.8, 0.8, 1.0]);
            let hex = to_hex_color(color);
            model_xml.push_str(&format!(
                "      <base name=\"Material_{}\" displaycolor=\"{}\" />\n",
                idx + 1, hex
            ));
        }
        model_xml.push_str("    </basematerials>\n");

        // Write Materials extension colorgroup resource (id="3")
        model_xml.push_str("    <m:colorgroup id=\"3\">\n");
        for sub in submeshes {
            let color = sub.color.unwrap_or([0.8, 0.8, 0.8, 1.0]);
            let hex = to_hex_color(color);
            model_xml.push_str(&format!(
                "      <m:color color=\"{}\" />\n",
                hex
            ));
        }
        model_xml.push_str("    </m:colorgroup>\n");

        // Write PBR metallic display properties (id="4") matching standard roughness/metallic factors
        model_xml.push_str("    <m:pbmetallicdisplayproperties id=\"4\">\n");
        for (idx, _) in submeshes.iter().enumerate() {
            model_xml.push_str(&format!(
                "      <m:pbmetallic name=\"Material_{}\" metallicness=\"0.0\" roughness=\"0.5\" />\n",
                idx + 1
            ));
        }
        model_xml.push_str("    </m:pbmetallicdisplayproperties>\n");

        model_xml.push_str("    <object id=\"1\" pid=\"2\" pindex=\"0\" type=\"model\">\n      <mesh>\n        <vertices>\n");

        // Deduplicate coincident 3D vertex positions across seams (e.g. UV seams)
        // so that the 3MF mesh remains a continuous, watertight solid surface,
        // while preserving separation only for parts that are physically disconnected in 3D space.
        let mut unique_vertices: Vec<[f32; 3]> = Vec::with_capacity(total_verts);
        let mut vertex_map: HashMap<(i64, i64, i64), u32> = HashMap::with_capacity(total_verts);

        let mut get_vertex_idx = |p: [f32; 3]| -> u32 {
            let key = (
                (p[0] * 10000.0).round() as i64,
                (p[1] * 10000.0).round() as i64,
                (p[2] * 10000.0).round() as i64,
            );
            *vertex_map.entry(key).or_insert_with(|| {
                let idx = unique_vertices.len() as u32;
                unique_vertices.push(p);
                idx
            })
        };

        let mut triangles_xml = String::with_capacity(total_tris * 90);
        for (mat_idx, sub) in submeshes.iter().enumerate() {
            let orca_code = orca_filament_code(mat_idx);
            for tri in &sub.indices {
                let i0 = tri[0] as usize;
                let i1 = tri[1] as usize;
                let i2 = tri[2] as usize;
                if i0 < sub.positions.len() && i1 < sub.positions.len() && i2 < sub.positions.len() {
                    let v0 = get_vertex_idx(sub.positions[i0]);
                    let v1 = get_vertex_idx(sub.positions[i1]);
                    let v2 = get_vertex_idx(sub.positions[i2]);
                    triangles_xml.push_str(&format!(
                        "          <triangle v1=\"{}\" v2=\"{}\" v3=\"{}\" pid=\"2\" p1=\"{}\" paint_color=\"{}\" />\n",
                        v0, v1, v2, mat_idx, orca_code
                    ));
                }
            }
        }

        for p in &unique_vertices {
            model_xml.push_str(&format!(
                "          <vertex x=\"{:.6}\" y=\"{:.6}\" z=\"{:.6}\" />\n",
                p[0], p[1], p[2]
            ));
        }

        model_xml.push_str("        </vertices>\n        <triangles>\n");
        model_xml.push_str(&triangles_xml);
        model_xml.push_str("        </triangles>\n      </mesh>\n    </object>\n  </resources>\n  <build>\n    <item objectid=\"1\" />\n  </build>\n</model>");

        zip.write_all(model_xml.as_bytes()).map_err(|e| format!("write: {}", e))?;
        zip.finish().map_err(|e| format!("zip finish: {}", e))?;

        println!("[3MF Format] Successfully exported {} triangles with materials to '{}'", total_tris, path.display());
        Ok(())
    }

    fn supports_animation(&self) -> bool {
        false
    }
}

fn compute_normals(positions: &[[f32; 3]], indices: &[[u32; 3]]) -> Vec<[f32; 3]> {
    let mut normals = vec![[0.0f32; 3]; positions.len()];
    for tri in indices {
        let i0 = tri[0] as usize;
        let i1 = tri[1] as usize;
        let i2 = tri[2] as usize;
        if i0 < positions.len() && i1 < positions.len() && i2 < positions.len() {
            let p0 = glam::Vec3::from(positions[i0]);
            let p1 = glam::Vec3::from(positions[i1]);
            let p2 = glam::Vec3::from(positions[i2]);

            let v01 = p1 - p0;
            let v02 = p2 - p0;
            let v12 = p2 - p1;

            let fnorm = v01.cross(v02);
            let fn_len = fnorm.length();
            if fn_len < 1e-6 {
                continue;
            }
            let fn_norm = fnorm / fn_len;

            let l01 = v01.length();
            let l02 = v02.length();
            let l12 = v12.length();

            if l01 > 1e-6 && l02 > 1e-6 && l12 > 1e-6 {
                let a0 = (v01.dot(v02) / (l01 * l02)).clamp(-1.0, 1.0).acos();
                let a1 = ((-v01.dot(v12)) / (l01 * l12)).clamp(-1.0, 1.0).acos();
                let a2 = ((-v02.dot(-v12)) / (l02 * l12)).clamp(-1.0, 1.0).acos();

                let n0 = glam::Vec3::from(normals[i0]) + fn_norm * a0;
                let n1 = glam::Vec3::from(normals[i1]) + fn_norm * a1;
                let n2 = glam::Vec3::from(normals[i2]) + fn_norm * a2;

                normals[i0] = n0.into();
                normals[i1] = n1.into();
                normals[i2] = n2.into();
            }
        }
    }
    for n in &mut normals {
        let v = glam::Vec3::from(*n);
        *n = if v.length_squared() > 1e-6 {
            v.normalize().into()
        } else {
            [0.0, 0.0, 1.0]
        };
    }
    normals
}
