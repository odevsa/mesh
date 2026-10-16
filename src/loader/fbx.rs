use super::{Loader, MeshData, SubMesh};
use std::path::Path;
use std::io::Read;
use std::collections::HashMap;

pub struct FbxLoader {}

fn get_material_color(mat: &ufbx::Material) -> [f32; 4] {
    let (r, g, b, a) = if mat.pbr.base_color.has_value {
        let c = mat.pbr.base_color.value_vec4;
        let factor = if mat.pbr.base_factor.has_value { mat.pbr.base_factor.value_vec4.x as f32 } else { 1.0 };
        (c.x as f32 * factor, c.y as f32 * factor, c.z as f32 * factor, c.w as f32)
    } else if mat.fbx.diffuse_color.has_value {
        let c = mat.fbx.diffuse_color.value_vec4;
        let factor = if mat.fbx.diffuse_factor.has_value { mat.fbx.diffuse_factor.value_vec4.x as f32 } else { 1.0 };
        (c.x as f32 * factor, c.y as f32 * factor, c.z as f32 * factor, c.w as f32)
    } else {
        let c = mat.fbx.diffuse_color.value_vec4;
        (c.x as f32, c.y as f32, c.z as f32, 1.0)
    };

    [
        r.clamp(0.0, 1.0).powf(1.0 / 2.2),
        g.clamp(0.0, 1.0).powf(1.0 / 2.2),
        b.clamp(0.0, 1.0).powf(1.0 / 2.2),
        a,
    ]
}

impl Loader for FbxLoader {
    fn name(&self) -> &str { "fbx" }

    fn can_load(&self, path: &Path, header: &[u8]) -> bool {
        if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
            if ext.eq_ignore_ascii_case("fbx") { return true; }
        }
        if header.starts_with(b"Kaydara FBX Binary") { return true; }
        false
    }

    fn load(&self, path: &Path, _reader: Box<dyn Read>) -> Result<MeshData, String> {
        let mut opts = ufbx::LoadOpts::default();
        opts.target_axes = ufbx::CoordinateAxes {
            up: ufbx::CoordinateAxis::PositiveZ,
            front: ufbx::CoordinateAxis::NegativeY,
            right: ufbx::CoordinateAxis::PositiveX,
        };
        opts.target_unit_meters = 1.0;
        let path_str = path.to_str().ok_or_else(|| "invalid UTF-8 path".to_string())?;
        let scene = ufbx::load_file(path_str, opts).map_err(|e| format!("fbx parse error: {:?}", e))?;

        let base_dir = path.parent().unwrap_or_else(|| Path::new("."));
        let mut tex_cache: HashMap<String, Option<std::sync::Arc<super::TextureData>>> = HashMap::new();

        let load_fbx_texture = |mat: &ufbx::Material, cache: &mut HashMap<String, Option<std::sync::Arc<super::TextureData>>>| -> Option<std::sync::Arc<super::TextureData>> {
            let tex_ref = mat.pbr.base_color.texture.as_ref()
                .or_else(|| mat.fbx.diffuse_color.texture.as_ref())
                .or_else(|| mat.textures.iter().find(|t| t.material_prop == "DiffuseColor" || t.material_prop == "BaseColor").map(|t| &t.texture))
                .or_else(|| mat.textures.first().map(|t| &t.texture));

            let tex = tex_ref?.as_ref();

            // 1. Embedded texture in FBX
            if !tex.content.is_empty() {
                let key = format!("embedded_{}", tex.element.element_id);
                if let Some(cached) = cache.get(&key) {
                    return cached.clone();
                }
                let loaded = super::load_texture_from_memory(&tex.content);
                cache.insert(key, loaded.clone());
                return loaded;
            }

            // 2. Relative filename or filename or absolute filename
            for name in &[&tex.relative_filename, &tex.filename, &tex.absolute_filename] {
                let s = name.as_ref().trim();
                if !s.is_empty() {
                    if let Some(cached) = cache.get(s) {
                        return cached.clone();
                    }
                    if let Some(found_path) = super::resolve_relative_file(base_dir, s) {
                        let loaded = super::load_texture_from_path(&found_path);
                        cache.insert(s.to_string(), loaded.clone());
                        if loaded.is_some() {
                            return loaded;
                        }
                    }
                }
            }

            None
        };

        struct SubMeshAcc {
            positions: Vec<[f32; 3]>,
            normals: Vec<[f32; 3]>,
            uvs: Vec<[f32; 2]>,
            indices: Vec<[u32; 3]>,
            color: Option<[f32; 4]>,
            texture: Option<std::sync::Arc<super::TextureData>>,
        }

        let mut positions = Vec::new();
        let mut normals = Vec::new();
        let mut indices = Vec::new();
        let mut submeshes = Vec::new();

        let mut processed_nodes = 0;

        for node in &scene.nodes {
            let mesh = match &node.mesh {
                Some(m) => m,
                None => continue,
            };
            processed_nodes += 1;

            let get_mat = |idx: usize| -> Option<&ufbx::Material> {
                if let Some(m) = node.materials.get(idx) {
                    Some(m.as_ref())
                } else if let Some(m) = mesh.materials.get(idx) {
                    Some(m.as_ref())
                } else if let Some(m) = node.materials.first() {
                    Some(m.as_ref())
                } else if let Some(m) = mesh.materials.first() {
                    Some(m.as_ref())
                } else {
                    None
                }
            };

            let mut mat_subs: HashMap<Option<u32>, SubMeshAcc> = HashMap::new();

            for (face_idx, face) in mesh.faces.iter().enumerate() {
                let raw_mat_idx = if mesh.face_material.len() > face_idx {
                    mesh.face_material[face_idx]
                } else {
                    0
                };
                let mat_idx = raw_mat_idx as usize;
                let mat_opt = get_mat(mat_idx);
                let mat_key = mat_opt.map(|m| m.element.element_id);

                let sub = mat_subs.entry(mat_key).or_insert_with(|| {
                    let color = mat_opt.map(get_material_color);
                    let texture = mat_opt.and_then(|m| load_fbx_texture(m, &mut tex_cache));
                    SubMeshAcc {
                        positions: Vec::new(),
                        normals: Vec::new(),
                        uvs: Vec::new(),
                        indices: Vec::new(),
                        color,
                        texture,
                    }
                });

                if face.num_indices >= 3 {
                    for t in 0..(face.num_indices - 2) {
                        let corners = [
                            face.index_begin + 0,
                            face.index_begin + t + 1,
                            face.index_begin + t + 2,
                        ];
                        let tri_start = sub.positions.len() as u32;
                        for &corner_idx in &corners {
                            let p_idx = mesh.vertex_indices[corner_idx as usize] as usize;
                            let p = mesh.vertices[p_idx];
                            let tp = ufbx::transform_position(&node.node_to_world, p);
                            sub.positions.push([tp.x as f32, tp.y as f32, tp.z as f32]);

                            let norm = if mesh.vertex_normal.exists {
                                let raw_n = if !mesh.vertex_normal.indices.is_empty() && (corner_idx as usize) < mesh.vertex_normal.indices.len() {
                                    let val_idx = mesh.vertex_normal.indices[corner_idx as usize] as usize;
                                    mesh.vertex_normal.values.get(val_idx).copied()
                                } else if (corner_idx as usize) < mesh.vertex_normal.values.len() {
                                    mesh.vertex_normal.values.get(corner_idx as usize).copied()
                                } else if p_idx < mesh.vertex_normal.values.len() {
                                    mesh.vertex_normal.values.get(p_idx).copied()
                                } else {
                                    None
                                };

                                if let Some(n) = raw_n {
                                    let tn = ufbx::transform_direction(&node.node_to_world, n);
                                    [tn.x as f32, tn.y as f32, tn.z as f32]
                                } else {
                                    [0.0, 0.0, 1.0]
                                }
                            } else {
                                [0.0, 0.0, 1.0]
                            };
                            sub.normals.push(norm);

                            let uv = if mesh.vertex_uv.exists {
                                let raw_u = if !mesh.vertex_uv.indices.is_empty() && (corner_idx as usize) < mesh.vertex_uv.indices.len() {
                                    let val_idx = mesh.vertex_uv.indices[corner_idx as usize] as usize;
                                    mesh.vertex_uv.values.get(val_idx).copied()
                                } else if (corner_idx as usize) < mesh.vertex_uv.values.len() {
                                    mesh.vertex_uv.values.get(corner_idx as usize).copied()
                                } else if p_idx < mesh.vertex_uv.values.len() {
                                    mesh.vertex_uv.values.get(p_idx).copied()
                                } else {
                                    None
                                };

                                if let Some(u) = raw_u {
                                    [u.x as f32, 1.0 - u.y as f32]
                                } else {
                                    [0.0, 0.0]
                                }
                            } else {
                                [0.0, 0.0]
                            };
                            sub.uvs.push(uv);
                        }
                        sub.indices.push([tri_start, tri_start + 1, tri_start + 2]);
                    }
                }
            }

            for (_, acc) in mat_subs {
                let base_vertex = positions.len() as u32;
                for tri in &acc.indices {
                    indices.push([tri[0] + base_vertex, tri[1] + base_vertex, tri[2] + base_vertex]);
                }
                positions.extend_from_slice(&acc.positions);
                normals.extend_from_slice(&acc.normals);

                submeshes.push(SubMesh {
                    positions: acc.positions,
                    normals: acc.normals,
                    uvs: acc.uvs,
                    indices: acc.indices,
                    color: acc.color,
                    texture: acc.texture,
                });
            }
        }

        if processed_nodes == 0 {
            for mesh in &scene.meshes {
                let get_mat = |idx: usize| -> Option<&ufbx::Material> {
                    if let Some(m) = mesh.materials.get(idx) {
                        Some(m.as_ref())
                    } else if let Some(m) = mesh.materials.first() {
                        Some(m.as_ref())
                    } else {
                        None
                    }
                };

                let mut mat_subs: HashMap<Option<u32>, SubMeshAcc> = HashMap::new();

                for (face_idx, face) in mesh.faces.iter().enumerate() {
                    let raw_mat_idx = if mesh.face_material.len() > face_idx {
                        mesh.face_material[face_idx]
                    } else {
                        0
                    };
                    let mat_idx = raw_mat_idx as usize;
                    let mat_opt = get_mat(mat_idx);
                    let mat_key = mat_opt.map(|m| m.element.element_id);

                    let sub = mat_subs.entry(mat_key).or_insert_with(|| {
                        let color = mat_opt.map(get_material_color);
                        let texture = mat_opt.and_then(|m| load_fbx_texture(m, &mut tex_cache));
                        SubMeshAcc {
                            positions: Vec::new(),
                            normals: Vec::new(),
                            uvs: Vec::new(),
                            indices: Vec::new(),
                            color,
                            texture,
                        }
                    });

                    if face.num_indices >= 3 {
                        for t in 0..(face.num_indices - 2) {
                            let corners = [
                                face.index_begin + 0,
                                face.index_begin + t + 1,
                                face.index_begin + t + 2,
                            ];
                            let tri_start = sub.positions.len() as u32;
                            for &corner_idx in &corners {
                                let p_idx = mesh.vertex_indices[corner_idx as usize] as usize;
                                let p = mesh.vertices[p_idx];
                                sub.positions.push([p.x as f32, p.y as f32, p.z as f32]);

                                let norm = if mesh.vertex_normal.exists {
                                    let raw_n = if !mesh.vertex_normal.indices.is_empty() && (corner_idx as usize) < mesh.vertex_normal.indices.len() {
                                        let val_idx = mesh.vertex_normal.indices[corner_idx as usize] as usize;
                                        mesh.vertex_normal.values.get(val_idx).copied()
                                    } else if (corner_idx as usize) < mesh.vertex_normal.values.len() {
                                        mesh.vertex_normal.values.get(corner_idx as usize).copied()
                                    } else if p_idx < mesh.vertex_normal.values.len() {
                                        mesh.vertex_normal.values.get(p_idx).copied()
                                    } else {
                                        None
                                    };

                                    if let Some(n) = raw_n {
                                        [n.x as f32, n.y as f32, n.z as f32]
                                    } else {
                                        [0.0, 0.0, 1.0]
                                    }
                                } else {
                                    [0.0, 0.0, 1.0]
                                };
                                sub.normals.push(norm);

                                let uv = if mesh.vertex_uv.exists {
                                    let raw_u = if !mesh.vertex_uv.indices.is_empty() && (corner_idx as usize) < mesh.vertex_uv.indices.len() {
                                        let val_idx = mesh.vertex_uv.indices[corner_idx as usize] as usize;
                                        mesh.vertex_uv.values.get(val_idx).copied()
                                    } else if (corner_idx as usize) < mesh.vertex_uv.values.len() {
                                        mesh.vertex_uv.values.get(corner_idx as usize).copied()
                                    } else if p_idx < mesh.vertex_uv.values.len() {
                                        mesh.vertex_uv.values.get(p_idx).copied()
                                    } else {
                                        None
                                    };

                                    if let Some(u) = raw_u {
                                        [u.x as f32, 1.0 - u.y as f32]
                                    } else {
                                        [0.0, 0.0]
                                    }
                                } else {
                                    [0.0, 0.0]
                                };
                                sub.uvs.push(uv);
                            }
                            sub.indices.push([tri_start, tri_start + 1, tri_start + 2]);
                        }
                    }
                }

                for (_, acc) in mat_subs {
                    let base_vertex = positions.len() as u32;
                    for tri in &acc.indices {
                        indices.push([tri[0] + base_vertex, tri[1] + base_vertex, tri[2] + base_vertex]);
                    }
                    positions.extend_from_slice(&acc.positions);
                    normals.extend_from_slice(&acc.normals);

                    submeshes.push(SubMesh {
                        positions: acc.positions,
                        normals: acc.normals,
                        uvs: acc.uvs,
                        indices: acc.indices,
                        color: acc.color,
                        texture: acc.texture,
                    });
                }
            }
        }

        if positions.is_empty() {
            return Err("fbx: no positions found in model".into());
        }

        Ok(MeshData {
            positions,
            normals,
            indices,
            submeshes,
            ..Default::default()
        })
    }
}
