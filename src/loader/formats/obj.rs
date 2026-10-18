use crate::loader::{
    load_texture_from_path, resolve_relative_file, FileFormatInfo, Loader, MeshData, SubMesh,
    TextureData,
};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read};
use std::path::Path;
use std::sync::Arc;

pub struct ObjLoader {}

impl Loader for ObjLoader {
    fn name(&self) -> &str { "obj" }

    fn formats(&self) -> Vec<FileFormatInfo> {
        vec![FileFormatInfo {
            extension: "obj",
            label: "Wavefront OBJ (*.obj)",
            default_filename: "model.obj",
            can_load: true,
            can_export: true,
        }]
    }

    fn can_load(&self, path: &Path, _header: &[u8]) -> bool {
        if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
            if ext.eq_ignore_ascii_case("obj") { return true }
        }
        false
    }

    fn load(&self, path: &Path, reader: Box<dyn Read>) -> Result<MeshData, String> {
        let mut buf_reader = BufReader::new(reader);
        let mut raw_positions: Vec<[f32; 3]> = Vec::new();
        let mut raw_normals: Vec<[f32; 3]> = Vec::new();
        let mut raw_uvs: Vec<[f32; 2]> = Vec::new();

        let mut mtl_colors: HashMap<String, [f32; 4]> = HashMap::new();
        let mut mtl_textures: HashMap<String, Arc<TextureData>> = HashMap::new();

        let mut current_mat_name: Option<String> = None;
        let mut mat_faces: HashMap<Option<String>, Vec<Vec<(i32, Option<i32>, Option<i32>)>>> = HashMap::new();

        let base_dir = path.parent().unwrap_or_else(|| Path::new("."));

        let mut line = String::new();
        while buf_reader.read_line(&mut line).map_err(|e| format!("read obj: {}", e))? > 0 {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                line.clear();
                continue;
            }

            let mut parts = trimmed.split_whitespace();
            if let Some(kw) = parts.next() {
                match kw {
                    "v" => {
                        let x: f32 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0.0);
                        let y: f32 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0.0);
                        let z: f32 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0.0);
                        raw_positions.push([x, -z, y]);
                    }
                    "vn" => {
                        let nx: f32 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0.0);
                        let ny: f32 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0.0);
                        let nz: f32 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0.0);
                        raw_normals.push([nx, -nz, ny]);
                    }
                    "vt" => {
                        let u: f32 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0.0);
                        let v: f32 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0.0);
                        raw_uvs.push([u, 1.0 - v]);
                    }
                    "mtllib" => {
                        let mtl_file = trimmed["mtllib".len()..].trim();
                        if let Some(mtl_path) = resolve_relative_file(base_dir, mtl_file) {
                            if let Ok(f) = std::fs::File::open(&mtl_path) {
                                let mtl_dir = mtl_path.parent().unwrap_or(base_dir);
                                let mut mtl_reader = BufReader::new(f);
                                let mut mtl_line = String::new();
                                let mut cur_mat = String::new();
                                while mtl_reader.read_line(&mut mtl_line).unwrap_or(0) > 0 {
                                    let mtrimmed = mtl_line.trim();
                                    let mut mparts = mtrimmed.split_whitespace();
                                    if let Some(mkw) = mparts.next() {
                                        if mkw == "newmtl" {
                                            if let Some(name) = mparts.next() {
                                                cur_mat = name.to_string();
                                            }
                                        } else if mkw == "Kd" && !cur_mat.is_empty() {
                                            let r: f32 = mparts.next().and_then(|s| s.parse().ok()).unwrap_or(0.8);
                                            let g: f32 = mparts.next().and_then(|s| s.parse().ok()).unwrap_or(0.8);
                                            let b: f32 = mparts.next().and_then(|s| s.parse().ok()).unwrap_or(0.8);
                                            // MTL Kd is linear diffuse color
                                            let r = r.clamp(0.0, 1.0);
                                            let g = g.clamp(0.0, 1.0);
                                            let b = b.clamp(0.0, 1.0);
                                            mtl_colors.insert(cur_mat.clone(), [r, g, b, 1.0]);
                                        } else if (mkw.eq_ignore_ascii_case("map_Kd") || mkw.eq_ignore_ascii_case("map_Ka") || mkw.eq_ignore_ascii_case("map_BaseColor")) && !cur_mat.is_empty() {
                                            // Extract texture path, skipping options like -s, -o, -bm, -clamp, etc.
                                            let mut tex_file = None;
                                            while let Some(tok) = mparts.next() {
                                                if tok.starts_with('-') {
                                                    match tok {
                                                        "-s" | "-o" | "-t" => {
                                                            let _ = mparts.next();
                                                            let _ = mparts.next();
                                                            let _ = mparts.next();
                                                        }
                                                        "-mm" => {
                                                            let _ = mparts.next();
                                                            let _ = mparts.next();
                                                        }
                                                        "-bm" | "-boost" | "-clamp" | "-blendu" | "-blendv" | "-imfchan" | "-type" => {
                                                            let _ = mparts.next();
                                                        }
                                                        _ => {}
                                                    }
                                                } else {
                                                    tex_file = Some(tok);
                                                }
                                            }

                                            // Fallback to substring after command if options parsing didn't find standard token
                                            let tex_path_str = tex_file.map(|s| s.to_string()).unwrap_or_else(|| {
                                                mtrimmed[mkw.len()..].trim().to_string()
                                            });

                                            if !tex_path_str.is_empty() {
                                                let resolved = resolve_relative_file(mtl_dir, &tex_path_str)
                                                    .or_else(|| resolve_relative_file(base_dir, &tex_path_str));
                                                if let Some(tex_path) = resolved {
                                                    if let Some(tex_data) = load_texture_from_path(&tex_path) {
                                                        mtl_textures.insert(cur_mat.clone(), tex_data);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    mtl_line.clear();
                                }
                            }
                        }
                    }
                    "usemtl" => {
                        if let Some(mat_name) = parts.next() {
                            current_mat_name = Some(mat_name.to_string());
                        }
                    }
                    "f" => {
                        let mut poly_verts = Vec::new();
                        for vert_str in parts {
                            let mut chunks = vert_str.split('/');
                            let p_idx: i32 = chunks.next().and_then(|s| s.parse().ok()).unwrap_or(0);
                            let uv_idx: Option<i32> = chunks.next().and_then(|s| s.parse().ok());
                            let norm_idx: Option<i32> = chunks.next().and_then(|s| s.parse().ok());
                            if p_idx != 0 {
                                poly_verts.push((p_idx, uv_idx, norm_idx));
                            }
                        }

                        if poly_verts.len() >= 3 {
                            mat_faces.entry(current_mat_name.clone()).or_default().push(poly_verts);
                        }
                    }
                    _ => {}
                }
            }
            line.clear();
        }

        if raw_positions.is_empty() {
            return Err("obj: no vertices found".into());
        }

        let mut global_positions: Vec<[f32; 3]> = Vec::new();
        let mut global_normals: Vec<[f32; 3]> = Vec::new();
        let mut global_uvs: Vec<[f32; 2]> = Vec::new();
        let mut global_indices: Vec<[u32; 3]> = Vec::new();
        let mut submeshes: Vec<SubMesh> = Vec::new();

        for (mat_opt, faces) in mat_faces {
            let color = mat_opt.as_ref().and_then(|m| mtl_colors.get(m)).copied();
            let texture = mat_opt.as_ref().and_then(|m| mtl_textures.get(m)).cloned();

            let mut sub_positions: Vec<[f32; 3]> = Vec::new();
            let mut sub_normals: Vec<[f32; 3]> = Vec::new();
            let mut sub_uvs: Vec<[f32; 2]> = Vec::new();
            let mut sub_indices: Vec<[u32; 3]> = Vec::new();
            let mut vertex_remap: HashMap<(usize, usize, usize), u32> = HashMap::new();

            for poly in faces {
                let mut tri_vert_indices = Vec::new();
                for (p_idx, uv_idx, n_idx) in poly {
                    let p_pos = if p_idx > 0 { (p_idx - 1) as usize } else { (raw_positions.len() as i32 + p_idx) as usize };
                    let uv_pos = uv_idx.map(|idx| if idx > 0 { (idx - 1) as usize } else { (raw_uvs.len() as i32 + idx) as usize }).unwrap_or(usize::MAX);
                    let norm_pos = n_idx.map(|idx| if idx > 0 { (idx - 1) as usize } else { (raw_normals.len() as i32 + idx) as usize }).unwrap_or(usize::MAX);

                    let key = (p_pos, uv_pos, norm_pos);
                    let idx = *vertex_remap.entry(key).or_insert_with(|| {
                        let new_sub_idx = sub_positions.len() as u32;
                        let pos = raw_positions.get(p_pos).copied().unwrap_or([0.0, 0.0, 0.0]);
                        let norm = if norm_pos != usize::MAX {
                            raw_normals.get(norm_pos).copied().unwrap_or([0.0, 0.0, 1.0])
                        } else {
                            [0.0, 0.0, 1.0]
                        };
                        let uv = if uv_pos != usize::MAX {
                            raw_uvs.get(uv_pos).copied().unwrap_or([0.0, 0.0])
                        } else {
                            [0.0, 0.0]
                        };

                        sub_positions.push(pos);
                        sub_normals.push(norm);
                        sub_uvs.push(uv);

                        global_positions.push(pos);
                        global_normals.push(norm);
                        global_uvs.push(uv);

                        new_sub_idx
                    });
                    tri_vert_indices.push(idx);
                }

                for i in 1..tri_vert_indices.len() - 1 {
                    let tri = [tri_vert_indices[0], tri_vert_indices[i], tri_vert_indices[i + 1]];
                    sub_indices.push(tri);
                }
            }

            if !sub_positions.is_empty() && !sub_indices.is_empty() {
                submeshes.push(SubMesh {
                    positions: sub_positions,
                    normals: sub_normals,
                    uvs: sub_uvs,
                    indices: sub_indices,
                    color,
                    texture,
                    ..Default::default()
                });
            }
        }

        for sub in &submeshes {
            global_indices.extend_from_slice(&sub.indices);
        }

        if submeshes.is_empty() {
            let default_indices: Vec<[u32; 3]> = (0..raw_positions.len() as u32 / 3)
                .map(|i| [i * 3, i * 3 + 1, i * 3 + 2])
                .collect();
            submeshes.push(SubMesh {
                positions: raw_positions.clone(),
                normals: raw_normals.clone(),
                uvs: raw_uvs.clone(),
                indices: default_indices.clone(),
                color: None,
                texture: None,
                ..Default::default()
            });
            global_positions = raw_positions;
            global_normals = raw_normals;
            global_uvs = raw_uvs;
            global_indices = default_indices;
        }

        Ok(MeshData {
            positions: global_positions,
            normals: global_normals,
            uvs: global_uvs,
            indices: global_indices,
            submeshes,
            ..Default::default()
        })
    }

    fn can_export(&self, path: &Path) -> bool {
        if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
            if ext.eq_ignore_ascii_case("obj") {
                return true;
            }
        }
        false
    }

    fn export(&self, path: &Path, mesh: &MeshData) -> Result<(), String> {
        use std::io::Write;
        let file = std::fs::File::create(path).map_err(|e| format!("create obj file failed: {}", e))?;
        let mut writer = std::io::BufWriter::new(file);

        writeln!(writer, "# Wavefront OBJ exported by Mesh").map_err(|e| format!("write obj: {}", e))?;

        let mtl_path = path.with_extension("mtl");
        let mtl_filename = mtl_path.file_name().and_then(|s| s.to_str()).unwrap_or("model.mtl");
        let has_materials = mesh.submeshes.iter().any(|s| s.color.is_some());

        if has_materials {
            if let Ok(mtl_file) = std::fs::File::create(&mtl_path) {
                let mut mtl_writer = std::io::BufWriter::new(mtl_file);
                let _ = writeln!(mtl_writer, "# Wavefront MTL exported by Mesh");
                for (idx, sub) in mesh.submeshes.iter().enumerate() {
                    if let Some(c) = sub.color {
                        let _ = writeln!(mtl_writer, "newmtl Material_{}", idx + 1);
                        let _ = writeln!(mtl_writer, "Ns 250.000000");
                        let _ = writeln!(mtl_writer, "Ka 1.000000 1.000000 1.000000");
                        let _ = writeln!(mtl_writer, "Kd {:.4} {:.4} {:.4}", c[0], c[1], c[2]);
                        let _ = writeln!(mtl_writer, "Ks 0.500000 0.500000 0.500000");
                        let _ = writeln!(mtl_writer, "Ni 1.500000");
                        let _ = writeln!(mtl_writer, "d {:.4}", c[3]);
                        let _ = writeln!(mtl_writer, "illum 2");
                    }
                }
            }
            let _ = writeln!(writer, "mtllib {}", mtl_filename);
        }

        let mut v_offset = 1usize;

        if !mesh.submeshes.is_empty() {
            for (idx, sub) in mesh.submeshes.iter().enumerate() {
                writeln!(writer, "g Submesh_{}", idx + 1).map_err(|e| format!("write: {}", e))?;
                if sub.color.is_some() {
                    writeln!(writer, "usemtl Material_{}", idx + 1).map_err(|e| format!("write: {}", e))?;
                }

                for p in &sub.positions {
                    // Coordinates back to OBJ space (Y-up)
                    writeln!(writer, "v {:.6} {:.6} {:.6}", p[0], p[2], -p[1]).map_err(|e| format!("write: {}", e))?;
                }
                for n in &sub.normals {
                    writeln!(writer, "vn {:.6} {:.6} {:.6}", n[0], n[2], -n[1]).map_err(|e| format!("write: {}", e))?;
                }
                for uv in &sub.uvs {
                    writeln!(writer, "vt {:.6} {:.6}", uv[0], uv[1]).map_err(|e| format!("write: {}", e))?;
                }

                let has_n = !sub.normals.is_empty();
                let has_uv = !sub.uvs.is_empty();

                writeln!(writer, "s 1").map_err(|e| format!("write: {}", e))?;

                for tri in &sub.indices {
                    let v0 = v_offset + tri[0] as usize;
                    let v1 = v_offset + tri[1] as usize;
                    let v2 = v_offset + tri[2] as usize;

                    if has_uv && has_n {
                        writeln!(writer, "f {}/{}/{} {}/{}/{} {}/{}/{}", v0, v0, v0, v1, v1, v1, v2, v2, v2).map_err(|e| format!("write: {}", e))?;
                    } else if has_n {
                        writeln!(writer, "f {}//{} {}//{} {}//{}", v0, v0, v1, v1, v2, v2).map_err(|e| format!("write: {}", e))?;
                    } else if has_uv {
                        writeln!(writer, "f {}/{} {}/{} {}/{}", v0, v0, v1, v1, v2, v2).map_err(|e| format!("write: {}", e))?;
                    } else {
                        writeln!(writer, "f {} {} {}", v0, v1, v2).map_err(|e| format!("write: {}", e))?;
                    }
                }

                v_offset += sub.positions.len();
            }
        } else {
            for p in &mesh.positions {
                writeln!(writer, "v {:.6} {:.6} {:.6}", p[0], p[2], -p[1]).map_err(|e| format!("write: {}", e))?;
            }
            for n in &mesh.normals {
                writeln!(writer, "vn {:.6} {:.6} {:.6}", n[0], n[2], -n[1]).map_err(|e| format!("write: {}", e))?;
            }
            for uv in &mesh.uvs {
                writeln!(writer, "vt {:.6} {:.6}", uv[0], uv[1]).map_err(|e| format!("write: {}", e))?;
            }

            let has_n = !mesh.normals.is_empty();
            let has_uv = !mesh.uvs.is_empty();

            writeln!(writer, "s 1").map_err(|e| format!("write: {}", e))?;

            for tri in &mesh.indices {
                let v0 = 1 + tri[0] as usize;
                let v1 = 1 + tri[1] as usize;
                let v2 = 1 + tri[2] as usize;

                if has_uv && has_n {
                    writeln!(writer, "f {}/{}/{} {}/{}/{} {}/{}/{}", v0, v0, v0, v1, v1, v1, v2, v2, v2).map_err(|e| format!("write: {}", e))?;
                } else if has_n {
                    writeln!(writer, "f {}//{} {}//{} {}//{}", v0, v0, v1, v1, v2, v2).map_err(|e| format!("write: {}", e))?;
                } else if has_uv {
                    writeln!(writer, "f {}/{} {}/{} {}/{}", v0, v0, v1, v1, v2, v2).map_err(|e| format!("write: {}", e))?;
                } else {
                    writeln!(writer, "f {} {} {}", v0, v1, v2).map_err(|e| format!("write: {}", e))?;
                }
            }
        }

        writer.flush().map_err(|e| format!("flush: {}", e))?;
        println!("[OBJ Format] Successfully exported model to '{}'", path.display());
        Ok(())
    }

    fn supports_animation(&self) -> bool {
        false
    }
}
