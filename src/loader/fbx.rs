use super::{Loader, MeshData, SubMesh};
use std::path::Path;
use std::io::Read;
use std::collections::HashMap;

pub struct FbxLoader {}

fn get_material_color(mat: &ufbx::Material) -> [f32; 4] {
    if mat.pbr.base_color.has_value {
        let c = mat.pbr.base_color.value_vec4;
        let factor = if mat.pbr.base_factor.has_value { mat.pbr.base_factor.value_vec4.x as f32 } else { 1.0 };
        return [(c.x as f32 * factor).clamp(0.0, 1.0), (c.y as f32 * factor).clamp(0.0, 1.0), (c.z as f32 * factor).clamp(0.0, 1.0), c.w as f32];
    }
    if mat.fbx.diffuse_color.has_value {
        let c = mat.fbx.diffuse_color.value_vec4;
        let factor = if mat.fbx.diffuse_factor.has_value { mat.fbx.diffuse_factor.value_vec4.x as f32 } else { 1.0 };
        return [(c.x as f32 * factor).clamp(0.0, 1.0), (c.y as f32 * factor).clamp(0.0, 1.0), (c.z as f32 * factor).clamp(0.0, 1.0), c.w as f32];
    }
    let c = mat.fbx.diffuse_color.value_vec4;
    [c.x as f32, c.y as f32, c.z as f32, 1.0]
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
        let path_str = path.to_str().ok_or_else(|| "invalid UTF-8 path".to_string())?;
        let scene = ufbx::load_file(path_str, opts).map_err(|e| format!("fbx parse error: {:?}", e))?;

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

            let base_vertex = positions.len() as u32;

            let mut sub_positions = Vec::new();
            for p in &mesh.vertices {
                let tp = ufbx::transform_position(&node.node_to_world, *p);
                let pos = [tp.x as f32, tp.y as f32, tp.z as f32];
                sub_positions.push(pos);
                positions.push(pos);
            }

            let mut sub_normals = Vec::new();
            if mesh.vertex_normal.exists {
                for n in &mesh.vertex_normal.values {
                    let tn = ufbx::transform_direction(&node.node_to_world, *n);
                    let norm = [tn.x as f32, tn.y as f32, tn.z as f32];
                    sub_normals.push(norm);
                    normals.push(norm);
                }
            }

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

            let mut mat_faces: HashMap<Option<[u8; 4]>, Vec<[u32; 3]>> = HashMap::new();

            for (face_idx, face) in mesh.faces.iter().enumerate() {
                let raw_mat_idx = if mesh.face_material.len() > face_idx {
                    mesh.face_material[face_idx]
                } else {
                    0
                };
                let mat_idx = raw_mat_idx as usize;
                let mat_color = get_mat(mat_idx).map(|m| get_material_color(m));
                let color_key = mat_color.map(|c| [
                    (c[0].clamp(0.0, 1.0) * 255.0) as u8,
                    (c[1].clamp(0.0, 1.0) * 255.0) as u8,
                    (c[2].clamp(0.0, 1.0) * 255.0) as u8,
                    (c[3].clamp(0.0, 1.0) * 255.0) as u8,
                ]);

                if face.num_indices >= 3 {
                    for t in 0..(face.num_indices - 2) {
                        let i0 = mesh.vertex_indices[(face.index_begin + 0) as usize];
                        let i1 = mesh.vertex_indices[(face.index_begin + t + 1) as usize];
                        let i2 = mesh.vertex_indices[(face.index_begin + t + 2) as usize];
                        let tri = [i0, i1, i2];
                        mat_faces.entry(color_key).or_default().push(tri);
                        indices.push([tri[0] + base_vertex, tri[1] + base_vertex, tri[2] + base_vertex]);
                    }
                }
            }

            for (color_key, sub_indices) in mat_faces {
                let color = color_key.map(|c| [
                    c[0] as f32 / 255.0,
                    c[1] as f32 / 255.0,
                    c[2] as f32 / 255.0,
                    c[3] as f32 / 255.0,
                ]);
                submeshes.push(SubMesh {
                    positions: sub_positions.clone(),
                    normals: sub_normals.clone(),
                    indices: sub_indices,
                    color,
                });
            }
        }

        if processed_nodes == 0 {
            for mesh in &scene.meshes {
                let base_vertex = positions.len() as u32;

                let mut sub_positions = Vec::new();
                for p in &mesh.vertices {
                    let pos = [p.x as f32, p.y as f32, p.z as f32];
                    sub_positions.push(pos);
                    positions.push(pos);
                }

                let mut sub_normals = Vec::new();
                if mesh.vertex_normal.exists {
                    for n in &mesh.vertex_normal.values {
                        let norm = [n.x as f32, n.y as f32, n.z as f32];
                        sub_normals.push(norm);
                        normals.push(norm);
                    }
                }

                let get_mat = |idx: usize| -> Option<&ufbx::Material> {
                    if let Some(m) = mesh.materials.get(idx) {
                        Some(m.as_ref())
                    } else if let Some(m) = mesh.materials.first() {
                        Some(m.as_ref())
                    } else {
                        None
                    }
                };

                let mut mat_faces: HashMap<Option<[u8; 4]>, Vec<[u32; 3]>> = HashMap::new();

                for (face_idx, face) in mesh.faces.iter().enumerate() {
                    let raw_mat_idx = if mesh.face_material.len() > face_idx {
                        mesh.face_material[face_idx]
                    } else {
                        0
                    };
                    let mat_idx = raw_mat_idx as usize;
                    let mat_color = get_mat(mat_idx).map(|m| get_material_color(m));
                    let color_key = mat_color.map(|c| [
                        (c[0].clamp(0.0, 1.0) * 255.0) as u8,
                        (c[1].clamp(0.0, 1.0) * 255.0) as u8,
                        (c[2].clamp(0.0, 1.0) * 255.0) as u8,
                        (c[3].clamp(0.0, 1.0) * 255.0) as u8,
                    ]);

                    if face.num_indices >= 3 {
                        for t in 0..(face.num_indices - 2) {
                            let i0 = mesh.vertex_indices[(face.index_begin + 0) as usize];
                            let i1 = mesh.vertex_indices[(face.index_begin + t + 1) as usize];
                            let i2 = mesh.vertex_indices[(face.index_begin + t + 2) as usize];
                            let tri = [i0, i1, i2];
                            mat_faces.entry(color_key).or_default().push(tri);
                            indices.push([tri[0] + base_vertex, tri[1] + base_vertex, tri[2] + base_vertex]);
                        }
                    }
                }

                for (color_key, sub_indices) in mat_faces {
                    let color = color_key.map(|c| [
                        c[0] as f32 / 255.0,
                        c[1] as f32 / 255.0,
                        c[2] as f32 / 255.0,
                        c[3] as f32 / 255.0,
                    ]);
                    submeshes.push(SubMesh {
                        positions: sub_positions.clone(),
                        normals: sub_normals.clone(),
                        indices: sub_indices,
                        color,
                    });
                }
            }
        }

        if positions.is_empty() {
            return Err("fbx: no positions found in model".into());
        }

        Ok(MeshData { positions, normals, indices, submeshes })
    }
}
