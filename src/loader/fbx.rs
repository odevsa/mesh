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

        struct SubMeshAcc {
            positions: Vec<[f32; 3]>,
            normals: Vec<[f32; 3]>,
            uvs: Vec<[f32; 2]>,
            indices: Vec<[u32; 3]>,
            color: Option<[f32; 4]>,
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

            let mut mat_subs: HashMap<Option<[u8; 4]>, SubMeshAcc> = HashMap::new();

            for (face_idx, face) in mesh.faces.iter().enumerate() {
                let raw_mat_idx = if mesh.face_material.len() > face_idx {
                    mesh.face_material[face_idx]
                } else {
                    0
                };
                let mat_idx = raw_mat_idx as usize;
                let mat_color = get_mat(mat_idx).map(get_material_color);
                let color_key = mat_color.map(|c| [
                    (c[0].clamp(0.0, 1.0) * 255.0) as u8,
                    (c[1].clamp(0.0, 1.0) * 255.0) as u8,
                    (c[2].clamp(0.0, 1.0) * 255.0) as u8,
                    (c[3].clamp(0.0, 1.0) * 255.0) as u8,
                ]);

                let sub = mat_subs.entry(color_key).or_insert_with(|| SubMeshAcc {
                    positions: Vec::new(),
                    normals: Vec::new(),
                    uvs: Vec::new(),
                    indices: Vec::new(),
                    color: mat_color,
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
                                let n = mesh.vertex_normal[corner_idx as usize];
                                let tn = ufbx::transform_direction(&node.node_to_world, n);
                                [tn.x as f32, tn.y as f32, tn.z as f32]
                            } else {
                                [0.0, 0.0, 1.0]
                            };
                            sub.normals.push(norm);

                            let uv = if mesh.vertex_uv.exists {
                                let u = mesh.vertex_uv[corner_idx as usize];
                                [u.x as f32, 1.0 - u.y as f32]
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
                    ..Default::default()
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

                let mut mat_subs: HashMap<Option<[u8; 4]>, SubMeshAcc> = HashMap::new();

                for (face_idx, face) in mesh.faces.iter().enumerate() {
                    let raw_mat_idx = if mesh.face_material.len() > face_idx {
                        mesh.face_material[face_idx]
                    } else {
                        0
                    };
                    let mat_idx = raw_mat_idx as usize;
                    let mat_color = get_mat(mat_idx).map(get_material_color);
                    let color_key = mat_color.map(|c| [
                        (c[0].clamp(0.0, 1.0) * 255.0) as u8,
                        (c[1].clamp(0.0, 1.0) * 255.0) as u8,
                        (c[2].clamp(0.0, 1.0) * 255.0) as u8,
                        (c[3].clamp(0.0, 1.0) * 255.0) as u8,
                    ]);

                    let sub = mat_subs.entry(color_key).or_insert_with(|| SubMeshAcc {
                        positions: Vec::new(),
                        normals: Vec::new(),
                        uvs: Vec::new(),
                        indices: Vec::new(),
                        color: mat_color,
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
                                    let n = mesh.vertex_normal[corner_idx as usize];
                                    [n.x as f32, n.y as f32, n.z as f32]
                                } else {
                                    [0.0, 0.0, 1.0]
                                };
                                sub.normals.push(norm);

                                let uv = if mesh.vertex_uv.exists {
                                    let u = mesh.vertex_uv[corner_idx as usize];
                                    [u.x as f32, 1.0 - u.y as f32]
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
                        ..Default::default()
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
