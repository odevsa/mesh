use super::{Loader, MeshData, SubMesh};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read};
use std::path::Path;

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
        let mut positions = Vec::new();
        let mut normals = Vec::new();
        let mut raw_normals = Vec::new();
        let mut indices = Vec::new();

        let mut mtl_colors: HashMap<String, [f32; 4]> = HashMap::new();
        let mut current_color: Option<[f32; 4]> = None;
        let mut color_groups: HashMap<Option<[u8; 4]>, Vec<[u32; 3]>> = HashMap::new();

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
                        positions.push([x, -z, y]);
                    }
                    "vn" => {
                        let nx: f32 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0.0);
                        let ny: f32 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0.0);
                        let nz: f32 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0.0);
                        raw_normals.push([nx, -nz, ny]);
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
                                            mtl_colors.insert(cur_mat.clone(), [r, g, b, 1.0]);
                                        }
                                    }
                                    mtl_line.clear();
                                }
                            }
                        }
                    }
                    "usemtl" => {
                        if let Some(mat_name) = parts.next() {
                            current_color = mtl_colors.get(mat_name).copied();
                        }
                    }
                    "f" => {
                        let mut poly_verts = Vec::new();
                        for vert_str in parts {
                            let first_num = vert_str.split('/').next().unwrap_or("");
                            if let Ok(idx) = first_num.parse::<i32>() {
                                let pos_idx = if idx > 0 {
                                    (idx - 1) as u32
                                } else if idx < 0 {
                                    (positions.len() as i32 + idx) as u32
                                } else {
                                    0
                                };
                                poly_verts.push(pos_idx);
                            }
                        }

                        if poly_verts.len() >= 3 {
                            let color_key = current_color.map(|c| [
                                (c[0].clamp(0.0, 1.0) * 255.0) as u8,
                                (c[1].clamp(0.0, 1.0) * 255.0) as u8,
                                (c[2].clamp(0.0, 1.0) * 255.0) as u8,
                                (c[3].clamp(0.0, 1.0) * 255.0) as u8,
                            ]);
                            let grp_tris = color_groups.entry(color_key).or_default();

                            for i in 1..poly_verts.len() - 1 {
                                let tri = [poly_verts[0], poly_verts[i], poly_verts[i + 1]];
                                indices.push(tri);
                                grp_tris.push(tri);
                            }
                        }
                    }
                    _ => {}
                }
            }
            line.clear();
        }

        if !raw_normals.is_empty() {
            normals = raw_normals;
        }

        let mut submeshes = Vec::new();
        for (color_key, sub_ind) in color_groups {
            let color = color_key.map(|c| [
                c[0] as f32 / 255.0,
                c[1] as f32 / 255.0,
                c[2] as f32 / 255.0,
                c[3] as f32 / 255.0,
            ]);
            submeshes.push(SubMesh {
                positions: positions.clone(),
                normals: normals.clone(),
                indices: sub_ind,
                color,
            });
        }

        if submeshes.is_empty() {
            submeshes.push(SubMesh {
                positions: positions.clone(),
                normals: normals.clone(),
                indices: indices.clone(),
                color: None,
            });
        }

        Ok(MeshData {
            positions,
            normals,
            indices,
            submeshes,
        })
    }
}
