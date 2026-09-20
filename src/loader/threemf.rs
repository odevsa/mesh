use super::{Loader, MeshData, SubMesh};
use std::path::Path;
use std::io::Read;
use std::collections::HashMap;

pub struct ThreemfLoader {}

fn srgb_to_linear(c: f32) -> f32 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn parse_hex_color(s: &str) -> Option<[f32; 4]> {
    let s = s.trim().trim_start_matches('#');
    let (r, g, b, a) = if s.len() == 6 {
        let r = u8::from_str_radix(&s[0..2], 16).ok()? as f32 / 255.0;
        let g = u8::from_str_radix(&s[2..4], 16).ok()? as f32 / 255.0;
        let b = u8::from_str_radix(&s[4..6], 16).ok()? as f32 / 255.0;
        (r, g, b, 1.0)
    } else if s.len() == 8 {
        let r = u8::from_str_radix(&s[0..2], 16).ok()? as f32 / 255.0;
        let g = u8::from_str_radix(&s[2..4], 16).ok()? as f32 / 255.0;
        let b = u8::from_str_radix(&s[4..6], 16).ok()? as f32 / 255.0;
        let a = u8::from_str_radix(&s[6..8], 16).ok()? as f32 / 255.0;
        (r, g, b, a)
    } else {
        return None;
    };

    Some([srgb_to_linear(r), srgb_to_linear(g), srgb_to_linear(b), a])
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
        "4" => Some(0),
        "8" => Some(1),
        "0C" | "0" => Some(2),
        "1C" => Some(3),
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
                                if attr.key.into_inner().ends_with(b"id") {
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

                                    submeshes.push(SubMesh {
                                        positions: current_obj_positions.clone(),
                                        normals: Vec::new(),
                                        indices: sub_indices,
                                        color,
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

        Ok(MeshData {
            positions: global_positions,
            normals: Vec::new(),
            indices: global_indices,
            submeshes,
        })
    }
}
