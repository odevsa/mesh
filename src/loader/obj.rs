use super::{Loader, MeshData, SubMesh, TextureData};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read};
use std::path::Path;
use std::sync::Arc;

pub struct ObjLoader {}

impl Loader for ObjLoader {
    fn name(&self) -> &str { "obj" }

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
                        if let Some(mtl_file) = parts.next() {
                            let mtl_path = base_dir.join(mtl_file);
                            if let Ok(f) = std::fs::File::open(&mtl_path) {
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
                                            // Convert linear MTL Kd to sRGB color
                                            let r = r.clamp(0.0, 1.0).powf(1.0 / 2.2);
                                            let g = g.clamp(0.0, 1.0).powf(1.0 / 2.2);
                                            let b = b.clamp(0.0, 1.0).powf(1.0 / 2.2);
                                            mtl_colors.insert(cur_mat.clone(), [r, g, b, 1.0]);
                                        } else if (mkw == "map_Kd" || mkw == "map_Ka") && !cur_mat.is_empty() {
                                            if let Some(tex_file) = mparts.next() {
                                                let tex_path = base_dir.join(tex_file);
                                                if let Ok(img) = image::open(&tex_path) {
                                                    let rgba = img.to_rgba8();
                                                    let (w, h) = rgba.dimensions();
                                                    mtl_textures.insert(cur_mat.clone(), Arc::new(TextureData {
                                                        width: w,
                                                        height: h,
                                                        rgba: rgba.into_raw(),
                                                    }));
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
        })
    }
}
