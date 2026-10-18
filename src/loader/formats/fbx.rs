use crate::loader::{
    compute_normals, load_texture_from_memory, load_texture_from_path, resolve_relative_file,
    AnimationChannel, AnimationClip, FileFormatInfo, Interpolation, Joint, Loader, MeshData,
    SceneNode, Skin, SubMesh, TargetProperty, TextureData,
};
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
        r.clamp(0.0, 1.0),
        g.clamp(0.0, 1.0),
        b.clamp(0.0, 1.0),
        a,
    ]
}

fn quat_to_euler_degrees(q: [f32; 4]) -> [f32; 3] {
    let [x, y, z, w] = q;
    let sinr_cosp = 2.0 * (w * x + y * z);
    let cosr_cosp = 1.0 - 2.0 * (x * x + y * y);
    let roll = sinr_cosp.atan2(cosr_cosp);

    let sinp = 2.0 * (w * y - z * x);
    let pitch = if sinp.abs() >= 1.0 {
        std::f32::consts::FRAC_PI_2.copysign(sinp)
    } else {
        sinp.asin()
    };

    let siny_cosp = 2.0 * (w * z + x * y);
    let cosy_cosp = 1.0 - 2.0 * (y * y + z * z);
    let yaw = siny_cosp.atan2(cosy_cosp);

    [roll.to_degrees(), pitch.to_degrees(), yaw.to_degrees()]
}

impl Loader for FbxLoader {
    fn name(&self) -> &str { "fbx" }

    fn formats(&self) -> Vec<FileFormatInfo> {
        vec![FileFormatInfo {
            extension: "fbx",
            label: "Autodesk FBX (*.fbx)",
            default_filename: "model.fbx",
            can_load: true,
            can_export: true,
        }]
    }

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
        let scene = ufbx::load_file(path_str, opts).map_err(|e| {
            for frame in &e.stack[..e.stack_size as usize] {
                eprintln!("  FRAME at line {}: {} ({})", frame.source_line, frame.description, frame.function);
            }
            format!("fbx parse error: {:?}", e)
        })?;

        let base_dir = path.parent().unwrap_or_else(|| Path::new("."));
        let mut tex_cache: HashMap<String, Option<std::sync::Arc<TextureData>>> = HashMap::new();

        let load_fbx_texture = |mat: &ufbx::Material, cache: &mut HashMap<String, Option<std::sync::Arc<TextureData>>>| -> Option<std::sync::Arc<TextureData>> {
            let tex_ref = mat.pbr.base_color.texture.as_ref()
                .or_else(|| mat.fbx.diffuse_color.texture.as_ref())
                .or_else(|| {
                    if mat.textures.count > 0 {
                        mat.textures.iter().find(|t| t.material_prop == "DiffuseColor" || t.material_prop == "BaseColor").map(|t| &t.texture)
                            .or_else(|| mat.textures.first().map(|t| &t.texture))
                    } else {
                        None
                    }
                });

            let tex = tex_ref?.as_ref();

            // 1. Embedded texture in FBX
            if tex.content.size > 0 {
                let key = format!("embedded_{}", tex.element.element_id);
                if let Some(cached) = cache.get(&key) {
                    return cached.clone();
                }
                let loaded = load_texture_from_memory(&tex.content);
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
                    if let Some(found_path) = resolve_relative_file(base_dir, s) {
                        let loaded = load_texture_from_path(&found_path);
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
            rest_positions: Vec<[f32; 3]>,
            normals: Vec<[f32; 3]>,
            rest_normals: Vec<[f32; 3]>,
            uvs: Vec<[f32; 2]>,
            indices: Vec<[u32; 3]>,
            color: Option<[f32; 4]>,
            texture: Option<std::sync::Arc<TextureData>>,
            joint_indices: Option<Vec<[u16; 4]>>,
            joint_weights: Option<Vec<[f32; 4]>>,
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
            let has_skin = mesh.skin_deformers.count > 0;

            let get_mat = |idx: usize| -> Option<&ufbx::Material> {
                if node.materials.count > idx {
                    Some(&node.materials[idx])
                } else if mesh.materials.count > idx {
                    Some(&mesh.materials[idx])
                } else if node.materials.count > 0 {
                    Some(&node.materials[0])
                } else if mesh.materials.count > 0 {
                    Some(&mesh.materials[0])
                } else {
                    None
                }
            };

            let mut mat_subs: HashMap<Option<u32>, SubMeshAcc> = HashMap::new();

            for (face_idx, face) in mesh.faces.iter().enumerate() {
                let raw_mat_idx = if mesh.materials.count > 0 && mesh.face_material.count > face_idx {
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
                        rest_positions: Vec::new(),
                        normals: Vec::new(),
                        rest_normals: Vec::new(),
                        uvs: Vec::new(),
                        indices: Vec::new(),
                        color,
                        texture,
                        joint_indices: if has_skin { Some(Vec::new()) } else { None },
                        joint_weights: if has_skin { Some(Vec::new()) } else { None },
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
                            sub.rest_positions.push([p.x as f32, p.y as f32, p.z as f32]);

                            let raw_n = if mesh.vertex_normal.exists {
                                if !mesh.vertex_normal.indices.is_empty() && (corner_idx as usize) < mesh.vertex_normal.indices.len() {
                                    let val_idx = mesh.vertex_normal.indices[corner_idx as usize] as usize;
                                    mesh.vertex_normal.values.get(val_idx).copied()
                                } else if (corner_idx as usize) < mesh.vertex_normal.values.len() {
                                    mesh.vertex_normal.values.get(corner_idx as usize).copied()
                                } else if p_idx < mesh.vertex_normal.values.len() {
                                    mesh.vertex_normal.values.get(p_idx).copied()
                                } else {
                                    None
                                }
                            } else {
                                None
                            };

                            let norm = if let Some(n) = raw_n {
                                let tn = ufbx::transform_direction(&node.node_to_world, n);
                                [tn.x as f32, tn.y as f32, tn.z as f32]
                            } else {
                                [0.0, 0.0, 1.0]
                            };
                            let rest_norm = if let Some(n) = raw_n {
                                [n.x as f32, n.y as f32, n.z as f32]
                            } else {
                                [0.0, 0.0, 1.0]
                            };
                            sub.normals.push(norm);
                            sub.rest_normals.push(rest_norm);

                            if has_skin {
                                let deformer = &mesh.skin_deformers[0];
                                let (j_indices, j_weights) = if p_idx < deformer.vertices.len() {
                                    let sv = &deformer.vertices[p_idx];
                                    let w_slice = &(*deformer.weights)[sv.weight_begin as usize..(sv.weight_begin + sv.num_weights) as usize];
                                    let mut indices = [0u16; 4];
                                    let mut weights = [0.0f32; 4];
                                    let mut total_w = 0.0f32;
                                    for (i, w) in w_slice.iter().take(4).enumerate() {
                                        indices[i] = w.cluster_index as u16;
                                        weights[i] = w.weight as f32;
                                        total_w += weights[i];
                                    }
                                    if total_w > 1e-6 {
                                        for w in &mut weights {
                                            *w /= total_w;
                                        }
                                    }
                                    (indices, weights)
                                } else {
                                    ([0, 0, 0, 0], [1.0, 0.0, 0.0, 0.0])
                                };
                                if let Some(ref mut jis) = sub.joint_indices {
                                    jis.push(j_indices);
                                }
                                if let Some(ref mut jws) = sub.joint_weights {
                                    jws.push(j_weights);
                                }
                            }

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
                    rest_positions: acc.rest_positions,
                    normals: acc.normals,
                    rest_normals: acc.rest_normals,
                    uvs: acc.uvs,
                    indices: acc.indices,
                    color: acc.color,
                    texture: acc.texture,
                    joint_indices: acc.joint_indices,
                    joint_weights: acc.joint_weights,
                    node_index: Some(node.element.typed_id as usize),
                });
            }
        }

        if processed_nodes == 0 && scene.meshes.count > 0 {
            for mesh in &scene.meshes {
                let get_mat = |idx: usize| -> Option<&ufbx::Material> {
                    if mesh.materials.count > idx {
                        Some(&mesh.materials[idx])
                    } else if mesh.materials.count > 0 {
                        Some(&mesh.materials[0])
                    } else {
                        None
                    }
                };

                let mut mat_subs: HashMap<Option<u32>, SubMeshAcc> = HashMap::new();

                for (face_idx, face) in mesh.faces.iter().enumerate() {
                    let raw_mat_idx = if mesh.materials.count > 0 && mesh.face_material.count > face_idx {
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
                            rest_positions: Vec::new(),
                            normals: Vec::new(),
                            rest_normals: Vec::new(),
                            uvs: Vec::new(),
                            indices: Vec::new(),
                            color,
                            texture,
                            joint_indices: None,
                            joint_weights: None,
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
                                sub.rest_positions.push([p.x as f32, p.y as f32, p.z as f32]);

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
                                sub.rest_normals.push(norm);

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
                        rest_positions: acc.rest_positions,
                        normals: acc.normals,
                        rest_normals: acc.rest_normals,
                        uvs: acc.uvs,
                        indices: acc.indices,
                        color: acc.color,
                        texture: acc.texture,
                        joint_indices: acc.joint_indices,
                        joint_weights: acc.joint_weights,
                        node_index: None,
                    });
                }
            }
        }

        let mut nodes = Vec::new();
        if scene.nodes.count > 0 {
            for node in &scene.nodes {
                let children = if node.children.count > 0 {
                    node.children.iter().map(|c| c.element.typed_id as usize).collect()
                } else {
                    Vec::new()
                };
                nodes.push(SceneNode {
                    name: node.element.name.to_string(),
                    parent: node.parent.as_ref().map(|p| p.element.typed_id as usize),
                    children,
                    initial_translation: [
                        node.local_transform.translation.x as f32,
                        node.local_transform.translation.y as f32,
                        node.local_transform.translation.z as f32,
                    ],
                    initial_rotation: [
                        node.local_transform.rotation.x as f32,
                        node.local_transform.rotation.y as f32,
                        node.local_transform.rotation.z as f32,
                        node.local_transform.rotation.w as f32,
                    ],
                    initial_scale: [
                        node.local_transform.scale.x as f32,
                        node.local_transform.scale.y as f32,
                        node.local_transform.scale.z as f32,
                    ],
                    mesh_index: node.mesh.as_ref().map(|m| m.element.typed_id as usize),
                });
            }
        }

        let mut skins = Vec::new();
        if scene.meshes.count > 0 {
            for mesh in &scene.meshes {
                if mesh.skin_deformers.count > 0 {
                    for deformer in &mesh.skin_deformers {
                        let name = deformer.element.name.to_string();
                        let mut joints = Vec::new();
                        if deformer.clusters.count > 0 {
                            for cluster in &deformer.clusters {
                                let bone_idx = cluster.bone_node.as_ref().map(|b| b.element.typed_id as usize).unwrap_or(0);
                                let m = &cluster.geometry_to_bone;
                                let inv_bind = [
                                    [m.m00 as f32, m.m10 as f32, m.m20 as f32, 0.0],
                                    [m.m01 as f32, m.m11 as f32, m.m21 as f32, 0.0],
                                    [m.m02 as f32, m.m12 as f32, m.m22 as f32, 0.0],
                                    [m.m03 as f32, m.m13 as f32, m.m23 as f32, 1.0],
                                ];
                                joints.push(Joint {
                                    node_index: bone_idx,
                                    inverse_bind_matrix: inv_bind,
                                });
                            }
                        }
                        skins.push(Skin { name, joints });
                    }
                }
            }
        }

        let mut animations = Vec::new();
        if scene.anim_stacks.count > 0 {
            for stack in &scene.anim_stacks {
                let name = stack.element.name.to_string();
                let duration = (stack.time_end - stack.time_begin).max(0.0) as f32;
                let mut channels = Vec::new();

                if let Ok(baked_root) = ufbx::bake_anim(&scene, &stack.anim, ufbx::BakeOpts::default()) {
                    let baked = &*baked_root;
                    if baked.nodes.count > 0 {
                        for baked_node in &baked.nodes {
                            let target_node = baked_node.typed_id as usize;

                            if baked_node.translation_keys.count > 0 {
                                let mut timestamps = Vec::with_capacity(baked_node.translation_keys.count);
                                let mut values = Vec::with_capacity(baked_node.translation_keys.count * 3);
                                for k in &baked_node.translation_keys {
                                    timestamps.push((k.time - stack.time_begin).max(0.0) as f32);
                                    values.push(k.value.x as f32);
                                    values.push(k.value.y as f32);
                                    values.push(k.value.z as f32);
                                }
                                channels.push(AnimationChannel {
                                    target_node,
                                    property: TargetProperty::Translation,
                                    interpolation: Interpolation::Linear,
                                    timestamps,
                                    values,
                                });
                            }

                            if baked_node.rotation_keys.count > 0 {
                                let mut timestamps = Vec::with_capacity(baked_node.rotation_keys.count);
                                let mut values = Vec::with_capacity(baked_node.rotation_keys.count * 4);
                                for k in &baked_node.rotation_keys {
                                    timestamps.push((k.time - stack.time_begin).max(0.0) as f32);
                                    values.push(k.value.x as f32);
                                    values.push(k.value.y as f32);
                                    values.push(k.value.z as f32);
                                    values.push(k.value.w as f32);
                                }
                                channels.push(AnimationChannel {
                                    target_node,
                                    property: TargetProperty::Rotation,
                                    interpolation: Interpolation::Linear,
                                    timestamps,
                                    values,
                                });
                            }

                            if baked_node.scale_keys.count > 0 {
                                let mut timestamps = Vec::with_capacity(baked_node.scale_keys.count);
                                let mut values = Vec::with_capacity(baked_node.scale_keys.count * 3);
                                for k in &baked_node.scale_keys {
                                    timestamps.push((k.time - stack.time_begin).max(0.0) as f32);
                                    values.push(k.value.x as f32);
                                    values.push(k.value.y as f32);
                                    values.push(k.value.z as f32);
                                }
                                channels.push(AnimationChannel {
                                    target_node,
                                    property: TargetProperty::Scale,
                                    interpolation: Interpolation::Linear,
                                    timestamps,
                                    values,
                                });
                            }
                        }
                    }
                }

                if duration > 0.0 || !channels.is_empty() {
                    animations.push(AnimationClip {
                        name: if name.is_empty() { "Take 001".to_string() } else { name },
                        duration,
                        channels,
                    });
                }
            }
        }
        let has_animation = !animations.is_empty();

        Ok(MeshData {
            positions,
            normals,
            indices,
            submeshes,
            has_animation,
            is_y_up: false,
            nodes,
            skins,
            animations,
            ..Default::default()
        })
    }

    fn can_export(&self, path: &Path) -> bool {
        if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
            if ext.eq_ignore_ascii_case("fbx") {
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
                ..Default::default()
            }];
            &fallback_submesh
        } else {
            return Err("Mesh contains no geometry to export".into());
        };

        let parent_dir = path.parent().unwrap_or_else(|| Path::new("."));
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("model");
        let mut tex_files: Vec<Option<String>> = Vec::with_capacity(submeshes.len());

        for (i, sub) in submeshes.iter().enumerate() {
            if let Some(ref tex) = sub.texture {
                let tex_filename = format!("{}_tex_{}.png", stem, i + 1);
                let tex_path = parent_dir.join(&tex_filename);
                if let Some(img_buf) = image::RgbaImage::from_raw(tex.width, tex.height, tex.rgba.clone()) {
                    let _ = img_buf.save(&tex_path);
                }
                tex_files.push(Some(tex_filename));
            } else {
                tex_files.push(None);
            }
        }

        use std::io::Write;
        let file = std::fs::File::create(path).map_err(|e| format!("create fbx file failed: {}", e))?;
        let mut writer = std::io::BufWriter::new(file);

        let mut total_verts = 0usize;
        let mut total_tris = 0usize;
        for sub in submeshes {
            total_verts += sub.positions.len();
            total_tris += sub.indices.len();
        }
        let total_indices = total_tris * 3;

        writeln!(
            writer,
            "; FBX 7.4.0 project file\n; Generated by Mesh App\n\nFBXHeaderExtension:  {{\n\tFBXHeaderVersion: 1003\n\tFBXVersion: 7400\n}}\n\nGlobalSettings:  {{\n\tVersion: 1000\n\tProperties70:  {{\n\t\tP: \"UpAxis\", \"int\", \"Integer\", \"\", 2\n\t\tP: \"UpAxisSign\", \"int\", \"Integer\", \"\", 1\n\t\tP: \"FrontAxis\", \"int\", \"Integer\", \"\", 1\n\t\tP: \"FrontAxisSign\", \"int\", \"Integer\", \"\", -1\n\t\tP: \"CoordAxis\", \"int\", \"Integer\", \"\", 0\n\t\tP: \"CoordAxisSign\", \"int\", \"Integer\", \"\", 1\n\t\tP: \"UnitScaleFactor\", \"double\", \"Number\", \"\", 100.0\n\t\tP: \"OriginalUnitScaleFactor\", \"double\", \"Number\", \"\", 100.0\n\t}}\n}}\n\nObjects:  {{\n\tGeometry: 100000, \"Geometry::Mesh\", \"Mesh\" {{\n\t\tVertices: *{} {{",
            total_verts * 3
        ).map_err(|e| format!("write: {}", e))?;

        write!(writer, "\t\t\ta: ").map_err(|e| format!("write: {}", e))?;
        let mut first = true;
        for sub in submeshes {
            let use_rest = !sub.rest_positions.is_empty() && sub.joint_indices.is_some();
            let pos_slice = if use_rest { &sub.rest_positions } else { &sub.positions };
            for p in pos_slice {
                if !first {
                    write!(writer, ",").map_err(|e| format!("write: {}", e))?;
                }
                first = false;
                write!(writer, "{:.6},{:.6},{:.6}", p[0], p[1], p[2]).map_err(|e| format!("write: {}", e))?;
            }
        }
        writeln!(writer, "\n\t\t}}\n\t\tPolygonVertexIndex: *{} {{", total_indices).map_err(|e| format!("write: {}", e))?;

        write!(writer, "\t\t\ta: ").map_err(|e| format!("write: {}", e))?;
        let mut first_poly = true;
        let mut v_offset = 0i32;
        for sub in submeshes {
            for tri in &sub.indices {
                if !first_poly {
                    write!(writer, ",").map_err(|e| format!("write: {}", e))?;
                }
                first_poly = false;
                let i0 = v_offset + tri[0] as i32;
                let i1 = v_offset + tri[1] as i32;
                let i2 = -(v_offset + tri[2] as i32) - 1; // Bitwise NOT marks polygon end in FBX
                write!(writer, "{},{},{}", i0, i1, i2).map_err(|e| format!("write: {}", e))?;
            }
            v_offset += sub.positions.len() as i32;
        }

        writeln!(
            writer,
            "\n\t\t}}\n\t\tGeometryVersion: 124\n\t\tLayerElementNormal: 0 {{\n\t\t\tVersion: 101\n\t\t\tName: \"\"\n\t\t\tMappingInformationType: \"ByPolygonVertex\"\n\t\t\tReferenceInformationType: \"Direct\"\n\t\t\tNormals: *{} {{",
            total_indices * 3
        ).map_err(|e| format!("write: {}", e))?;

        write!(writer, "\t\t\ta: ").map_err(|e| format!("write: {}", e))?;
        let mut first_norm = true;
        for sub in submeshes {
            let use_rest = !sub.rest_positions.is_empty() && sub.joint_indices.is_some();
            let sub_normals: Vec<[f32; 3]> = if use_rest && sub.rest_normals.len() == sub.positions.len() {
                sub.rest_normals.clone()
            } else if sub.normals.len() == sub.positions.len() {
                sub.normals.clone()
            } else {
                let pos_slice = if use_rest { &sub.rest_positions } else { &sub.positions };
                compute_normals(pos_slice, &sub.indices)
            };
            for tri in &sub.indices {
                for &idx in tri {
                    let n = sub_normals.get(idx as usize).copied().unwrap_or([0.0, 0.0, 1.0]);
                    if !first_norm {
                        write!(writer, ",").map_err(|e| format!("write: {}", e))?;
                    }
                    first_norm = false;
                    write!(writer, "{:.6},{:.6},{:.6}", n[0], n[1], n[2]).map_err(|e| format!("write: {}", e))?;
                }
            }
        }
        writeln!(writer, "\n\t\t\t}}\n\t\t}}").map_err(|e| format!("write: {}", e))?;

        // LayerElementUV
        writeln!(
            writer,
            "\t\tLayerElementUV: 0 {{\n\t\t\tVersion: 101\n\t\t\tName: \"UVMap\"\n\t\t\tMappingInformationType: \"ByPolygonVertex\"\n\t\t\tReferenceInformationType: \"Direct\"\n\t\t\tUV: *{} {{",
            total_indices * 2
        ).map_err(|e| format!("write: {}", e))?;
        write!(writer, "\t\t\ta: ").map_err(|e| format!("write: {}", e))?;

        let mut first_uv = true;
        for sub in submeshes {
            for tri in &sub.indices {
                for &idx in tri {
                    let uv = sub.uvs.get(idx as usize).copied().unwrap_or([0.0, 0.0]);
                    if !first_uv {
                        write!(writer, ",").map_err(|e| format!("write: {}", e))?;
                    }
                    first_uv = false;
                    // In FBX, V=0 is at bottom
                    write!(writer, "{:.6},{:.6}", uv[0], 1.0 - uv[1]).map_err(|e| format!("write: {}", e))?;
                }
            }
        }
        writeln!(writer, "\n\t\t\t}}\n\t\t}}").map_err(|e| format!("write: {}", e))?;

        // LayerElementMaterial
        writeln!(
            writer,
            "\t\tLayerElementMaterial: 0 {{\n\t\t\tVersion: 101\n\t\t\tName: \"\"\n\t\t\tMappingInformationType: \"ByPolygon\"\n\t\t\tReferenceInformationType: \"IndexToDirect\"\n\t\t\tMaterials: *{} {{",
            total_tris
        ).map_err(|e| format!("write: {}", e))?;
        write!(writer, "\t\t\ta: ").map_err(|e| format!("write: {}", e))?;

        let mut first_mat = true;
        for (mat_idx, sub) in submeshes.iter().enumerate() {
            for _ in &sub.indices {
                if !first_mat {
                    write!(writer, ",").map_err(|e| format!("write: {}", e))?;
                }
                first_mat = false;
                write!(writer, "{}", mat_idx).map_err(|e| format!("write: {}", e))?;
            }
        }
        writeln!(writer, "\n\t\t\t}}\n\t\t}}\n\t}}").map_err(|e| format!("write: {}", e))?;

        // Models
        let has_nodes = !mesh.nodes.is_empty();
        let mesh_node_idx = if has_nodes {
            mesh.nodes.iter().position(|n| n.mesh_index.is_some()).unwrap_or(0)
        } else {
            0
        };

        if has_nodes {
            for (i, node) in mesh.nodes.iter().enumerate() {
                let node_id = 500000 + i;
                let name = if node.name.is_empty() { format!("Node_{}", i) } else { node.name.clone() };
                let t = node.initial_translation;
                let r_euler = quat_to_euler_degrees(node.initial_rotation);
                let s = node.initial_scale;
                let model_type = if i == mesh_node_idx { "Mesh" } else { "Null" };

                writeln!(
                    writer,
                    "\tModel: {}, \"Model::{}\", \"{}\" {{\n\t\tVersion: 232\n\t\tProperties70:  {{\n\t\t\tP: \"InheritType\", \"enum\", \"\", \"\",1\n\t\t\tP: \"Lcl Translation\", \"Lcl Translation\", \"\", \"A\", {:.6},{:.6},{:.6}\n\t\t\tP: \"Lcl Rotation\", \"Lcl Rotation\", \"\", \"A\", {:.6},{:.6},{:.6}\n\t\t\tP: \"Lcl Scaling\", \"Lcl Scaling\", \"\", \"A\", {:.6},{:.6},{:.6}\n\t\t}}\n\t}}",
                    node_id, name, model_type, t[0], t[1], t[2], r_euler[0], r_euler[1], r_euler[2], s[0], s[1], s[2]
                ).map_err(|e| format!("write: {}", e))?;
            }
        } else {
            writeln!(
                writer,
                "\tModel: 200000, \"Model::MeshModel\", \"Mesh\" {{\n\t\tVersion: 232\n\t\tProperties70:  {{\n\t\t\tP: \"InheritType\", \"enum\", \"\", \"\",1\n\t\t}}\n\t}}"
            ).map_err(|e| format!("write: {}", e))?;
        }

        // Materials
        for (mat_idx, sub) in submeshes.iter().enumerate() {
            let color = sub.color.unwrap_or([0.8, 0.8, 0.8, 1.0]);
            let mat_id = 300000 + mat_idx;
            writeln!(
                writer,
                "\tMaterial: {}, \"Material::Material_{}\", \"\" {{\n\t\tVersion: 102\n\t\tShadingModel: \"phong\"\n\t\tMultiLayer: 0\n\t\tProperties70:  {{\n\t\t\tP: \"DiffuseColor\", \"Color\", \"\", \"A\", {:.4}, {:.4}, {:.4}\n\t\t\tP: \"DiffuseFactor\", \"Number\", \"\", \"A\", 1.0\n\t\t\tP: \"AmbientColor\", \"Color\", \"\", \"A\", 0.0, 0.0, 0.0\n\t\t\tP: \"SpecularColor\", \"Color\", \"\", \"A\", 0.2, 0.2, 0.2\n\t\t\tP: \"Shininess\", \"Number\", \"\", \"A\", 20.0\n\t\t\tP: \"Opacity\", \"Number\", \"\", \"A\", {:.4}\n\t\t}}\n\t}}",
                mat_id, mat_idx + 1, color[0], color[1], color[2], color[3]
            ).map_err(|e| format!("write: {}", e))?;
        }

        // Textures
        for (mat_idx, file_opt) in tex_files.iter().enumerate() {
            if let Some(filename) = file_opt {
                let tex_id = 400000 + mat_idx;
                writeln!(
                    writer,
                    "\tTexture: {}, \"Texture::Texture_{}\", \"\" {{\n\t\tType: \"TextureVideoClip\"\n\t\tVersion: 202\n\t\tTextureName: \"Texture::Texture_{}\"\n\t\tProperties70:  {{\n\t\t\tP: \"CurrentTextureBlendMode\", \"enum\", \"\", \"\",0\n\t\t\tP: \"UVSet\", \"KString\", \"\", \"\", \"UVMap\"\n\t\t\tP: \"UseMaterial\", \"bool\", \"\", \"\",1\n\t\t}}\n\t\tFileName: \"{}\"\n\t\tRelativeFilename: \"{}\"\n\t\tModelUVTranslation: 0,0\n\t\tModelUVScaling: 1,1\n\t\tTexture_Alpha_Source: \"None\"\n\t}}",
                    tex_id, mat_idx + 1, mat_idx + 1, filename, filename
                ).map_err(|e| format!("write: {}", e))?;
            }
        }

        // Skin Deformers (Skeletal binding)
        let has_skin = mesh.skins.first().map(|s| !s.joints.is_empty()).unwrap_or(false);
        if has_skin {
            let skin = mesh.skins.first().unwrap();
            let skin_id = 800000i64;
            writeln!(
                writer,
                "\tDeformer: {}, \"Deformer::Skin\", \"Skin\" {{\n\t\tVersion: 101\n\t\tLink_DeformAcuracy: 50.0\n\t}}",
                skin_id
            ).map_err(|e| format!("write: {}", e))?;

            for (j_idx, joint) in skin.joints.iter().enumerate() {
                let cluster_id = 800001i64 + j_idx as i64;
                let mut cluster_indices = Vec::new();
                let mut cluster_weights = Vec::new();
                let mut v_base = 0u32;

                for sub in submeshes {
                    if let (Some(jis), Some(jws)) = (&sub.joint_indices, &sub.joint_weights) {
                        for (k, (&indices, &weights)) in jis.iter().zip(jws.iter()).enumerate() {
                            for m in 0..4 {
                                if indices[m] as usize == j_idx && weights[m] > 1e-4 {
                                    cluster_indices.push(v_base + k as u32);
                                    cluster_weights.push(weights[m]);
                                }
                            }
                        }
                    }
                    v_base += sub.positions.len() as u32;
                }

                let inv_bind_mat = glam::Mat4::from_cols_array_2d(&joint.inverse_bind_matrix);
                let bind_to_world = inv_bind_mat.inverse();
                let cols = bind_to_world.to_cols_array();

                write!(
                    writer,
                    "\tDeformer: {}, \"SubDeformer::Cluster\", \"SubDeformer\" {{\n\t\tVersion: 100\n\t\tUserData: \"\", \"\"\n\t\tIndexes: *{} {{\n\t\t\ta: ",
                    cluster_id,
                    cluster_indices.len()
                ).map_err(|e| format!("write: {}", e))?;
                for (ki, idx) in cluster_indices.iter().enumerate() {
                    if ki > 0 { write!(writer, ",").map_err(|e| format!("write: {}", e))?; }
                    write!(writer, "{}", idx).map_err(|e| format!("write: {}", e))?;
                }
                write!(writer, "\n\t\t}}\n\t\tWeights: *{} {{\n\t\t\ta: ", cluster_weights.len()).map_err(|e| format!("write: {}", e))?;
                for (ki, w) in cluster_weights.iter().enumerate() {
                    if ki > 0 { write!(writer, ",").map_err(|e| format!("write: {}", e))?; }
                    write!(writer, "{:.6}", w).map_err(|e| format!("write: {}", e))?;
                }
                writeln!(
                    writer,
                    "\n\t\t}}\n\t\tTransform: *16 {{\n\t\t\ta: 1,0,0,0,0,1,0,0,0,0,1,0,0,0,0,1\n\t\t}}\n\t\tTransformLink: *16 {{\n\t\t\ta: {:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6}\n\t\t}}\n\t}}",
                    cols[0], cols[1], cols[2], cols[3],
                    cols[4], cols[5], cols[6], cols[7],
                    cols[8], cols[9], cols[10], cols[11],
                    cols[12], cols[13], cols[14], cols[15]
                ).map_err(|e| format!("write: {}", e))?;
            }
        }

        // Animations
        for (clip_idx, clip) in mesh.animations.iter().enumerate() {
            let stack_id = 700000 + clip_idx * 10000;
            let layer_id = stack_id + 1;
            let duration_ktime = (clip.duration as f64 * 46186158000.0) as i64;
            let stack_name = if clip.name.is_empty() { format!("Take 00{}", clip_idx + 1) } else { clip.name.clone() };

            writeln!(
                writer,
                "\tAnimationStack: {}, \"AnimStack::{}\", \"\" {{\n\t\tProperties70:  {{\n\t\t\tP: \"LocalStop\", \"KTime\", \"Time\", \"\", {}\n\t\t}}\n\t}}\n\tAnimationLayer: {}, \"AnimLayer::BaseLayer\", \"\" {{\n\t}}",
                stack_id, stack_name, duration_ktime, layer_id
            ).map_err(|e| format!("write: {}", e))?;

            for (ch_idx, ch) in clip.channels.iter().enumerate() {
                if ch.timestamps.is_empty() || ch.values.is_empty() {
                    continue;
                }
                let curve_node_id = stack_id + 10 + ch_idx * 10;
                let type_name = match ch.property {
                    TargetProperty::Translation => "T",
                    TargetProperty::Rotation => "R",
                    TargetProperty::Scale => "S",
                    TargetProperty::Weights => continue,
                };

                writeln!(
                    writer,
                    "\tAnimationCurveNode: {}, \"AnimCurveNode::{}\", \"\" {{\n\t\tProperties70:  {{\n\t\t\tP: \"d|X\", \"Number\", \"\", \"A\", 0.0\n\t\t\tP: \"d|Y\", \"Number\", \"\", \"A\", 0.0\n\t\t\tP: \"d|Z\", \"Number\", \"\", \"A\", 0.0\n\t\t}}\n\t}}",
                    curve_node_id, type_name
                ).map_err(|e| format!("write: {}", e))?;

                let ktimes: Vec<i64> = ch.timestamps.iter().map(|&t| (t as f64 * 46186158000.0) as i64).collect();
                let mut vals_x = Vec::with_capacity(ch.timestamps.len());
                let mut vals_y = Vec::with_capacity(ch.timestamps.len());
                let mut vals_z = Vec::with_capacity(ch.timestamps.len());

                if ch.property == TargetProperty::Rotation {
                    for k in 0..ch.timestamps.len() {
                        let qx = ch.values.get(k * 4).copied().unwrap_or(0.0);
                        let qy = ch.values.get(k * 4 + 1).copied().unwrap_or(0.0);
                        let qz = ch.values.get(k * 4 + 2).copied().unwrap_or(0.0);
                        let qw = ch.values.get(k * 4 + 3).copied().unwrap_or(1.0);
                        let euler = quat_to_euler_degrees([qx, qy, qz, qw]);
                        vals_x.push(euler[0]);
                        vals_y.push(euler[1]);
                        vals_z.push(euler[2]);
                    }
                } else {
                    for k in 0..ch.timestamps.len() {
                        vals_x.push(ch.values.get(k * 3).copied().unwrap_or(0.0));
                        vals_y.push(ch.values.get(k * 3 + 1).copied().unwrap_or(0.0));
                        vals_z.push(ch.values.get(k * 3 + 2).copied().unwrap_or(0.0));
                    }
                }

                for (axis_idx, vals) in [&vals_x, &vals_y, &vals_z].iter().enumerate() {
                    let curve_id = curve_node_id + 1 + axis_idx;
                    write!(writer, "\tAnimationCurve: {}, \"AnimCurve::\", \"\" {{\n\t\tDefault: 0.0\n\t\tKeyVer: 4008\n\t\tKeyTime: *{} {{\n\t\t\ta: ", curve_id, ktimes.len()).map_err(|e| format!("write: {}", e))?;
                    for (ki, &kt) in ktimes.iter().enumerate() {
                        if ki > 0 { write!(writer, ",").map_err(|e| format!("write: {}", e))?; }
                        write!(writer, "{}", kt).map_err(|e| format!("write: {}", e))?;
                    }
                    write!(writer, "\n\t\t}}\n\t\tKeyValueFloat: *{} {{\n\t\t\ta: ", vals.len()).map_err(|e| format!("write: {}", e))?;
                    for (ki, &v) in vals.iter().enumerate() {
                        if ki > 0 { write!(writer, ",").map_err(|e| format!("write: {}", e))?; }
                        write!(writer, "{:.6}", v).map_err(|e| format!("write: {}", e))?;
                    }
                    write!(
                        writer,
                        "\n\t\t}}\n\t\tKeyAttrFlags: *1 {{\n\t\t\ta: 24836\n\t\t}}\n\t\tKeyAttrDataFloat: *4 {{\n\t\t\ta: 0,0,0,0\n\t\t}}\n\t\tKeyAttrRefCount: *1 {{\n\t\t\ta: {}\n\t\t}}\n\t}}\n",
                        ktimes.len()
                    ).map_err(|e| format!("write: {}", e))?;
                }
            }
        }

        writeln!(writer, "}}\n\nConnections:  {{").map_err(|e| format!("write: {}", e))?;

        let main_model_id = if has_nodes { 500000 + mesh_node_idx } else { 200000 };

        if has_nodes {
            for (i, node) in mesh.nodes.iter().enumerate() {
                let node_id = 500000 + i;
                if let Some(parent_idx) = node.parent {
                    if parent_idx < mesh.nodes.len() {
                        writeln!(writer, "\tC: \"OO\",{},{}", node_id, 500000 + parent_idx).map_err(|e| format!("write: {}", e))?;
                    } else {
                        writeln!(writer, "\tC: \"OO\",{},0", node_id).map_err(|e| format!("write: {}", e))?;
                    }
                } else {
                    writeln!(writer, "\tC: \"OO\",{},0", node_id).map_err(|e| format!("write: {}", e))?;
                }
            }
        } else {
            writeln!(writer, "\t;Model::MeshModel, RootNode\n\tC: \"OO\",200000,0").map_err(|e| format!("write: {}", e))?;
        }

        // Geometry to Model
        writeln!(writer, "\t;Geometry::Mesh, Model\n\tC: \"OO\",100000,{}", main_model_id).map_err(|e| format!("write: {}", e))?;

        // Materials to Model
        for (mat_idx, _) in submeshes.iter().enumerate() {
            let mat_id = 300000 + mat_idx;
            writeln!(writer, "\t;Material::Material_{}, Model\n\tC: \"OO\",{},{}", mat_idx + 1, mat_id, main_model_id).map_err(|e| format!("write: {}", e))?;
        }

        // Textures to Materials
        for (mat_idx, file_opt) in tex_files.iter().enumerate() {
            if file_opt.is_some() {
                let tex_id = 400000 + mat_idx;
                let mat_id = 300000 + mat_idx;
                writeln!(writer, "\t;Texture_{}, Material_{}\n\tC: \"OP\",{},{},\"DiffuseColor\"", mat_idx + 1, mat_idx + 1, tex_id, mat_id).map_err(|e| format!("write: {}", e))?;
            }
        }

        // Skin connections
        if has_skin {
            let skin = mesh.skins.first().unwrap();
            let skin_id = 800000i64;
            // Connect Skin to Geometry
            writeln!(writer, "\tC: \"OO\",{},100000", skin_id).map_err(|e| format!("write: {}", e))?;

            for (j_idx, joint) in skin.joints.iter().enumerate() {
                let cluster_id = 800001i64 + j_idx as i64;
                let bone_node_id = if joint.node_index < mesh.nodes.len() {
                    500000 + joint.node_index
                } else {
                    main_model_id
                };
                // Connect Cluster to Skin
                writeln!(writer, "\tC: \"OO\",{},{}", cluster_id, skin_id).map_err(|e| format!("write: {}", e))?;
                // Connect Cluster to Bone Node
                writeln!(writer, "\tC: \"OO\",{},{}", cluster_id, bone_node_id).map_err(|e| format!("write: {}", e))?;
            }
        }

        // Animation connections
        for (clip_idx, clip) in mesh.animations.iter().enumerate() {
            let stack_id = 700000 + clip_idx * 10000;
            let layer_id = stack_id + 1;
            writeln!(writer, "\tC: \"OO\",{},{}", layer_id, stack_id).map_err(|e| format!("write: {}", e))?;

            for (ch_idx, ch) in clip.channels.iter().enumerate() {
                if ch.timestamps.is_empty() || ch.values.is_empty() {
                    continue;
                }
                let curve_node_id = stack_id + 10 + ch_idx * 10;
                let target_node_id = if has_nodes { 500000 + ch.target_node } else { 200000 };
                let prop_name = match ch.property {
                    TargetProperty::Translation => "Lcl Translation",
                    TargetProperty::Rotation => "Lcl Rotation",
                    TargetProperty::Scale => "Lcl Scaling",
                    TargetProperty::Weights => continue,
                };

                writeln!(writer, "\tC: \"OO\",{},{}", curve_node_id, layer_id).map_err(|e| format!("write: {}", e))?;
                writeln!(writer, "\tC: \"OP\",{},{},\"{}\"", curve_node_id, target_node_id, prop_name).map_err(|e| format!("write: {}", e))?;
                writeln!(writer, "\tC: \"OP\",{},{},\"d|X\"", curve_node_id + 1, curve_node_id).map_err(|e| format!("write: {}", e))?;
                writeln!(writer, "\tC: \"OP\",{},{},\"d|Y\"", curve_node_id + 2, curve_node_id).map_err(|e| format!("write: {}", e))?;
                writeln!(writer, "\tC: \"OP\",{},{},\"d|Z\"", curve_node_id + 3, curve_node_id).map_err(|e| format!("write: {}", e))?;
            }
        }

        writeln!(writer, "}}").map_err(|e| format!("write: {}", e))?;

        writer.flush().map_err(|e| format!("flush: {}", e))?;
        println!("[FBX Format] Successfully exported ASCII FBX with {} triangles and {} materials to '{}'", total_tris, submeshes.len(), path.display());
        Ok(())
    }

    fn supports_animation(&self) -> bool {
        println!("[FBX Format] Format natively supports animation tracks (skeletal/skinning)");
        true
    }
}
