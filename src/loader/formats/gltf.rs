use crate::loader::{
    compute_normals, AnimationChannel, AnimationClip, FileFormatInfo, Interpolation, Joint, Loader,
    MeshData, SceneNode, Skin, SubMesh, TargetProperty, TextureData,
};
use std::path::Path;
use std::io::Read;
use std::sync::Arc;
use glam::{Mat4, Vec3};

pub struct GltfLoader {}

impl Loader for GltfLoader {
    fn name(&self) -> &str { "gltf" }

    fn formats(&self) -> Vec<FileFormatInfo> {
        vec![
            FileFormatInfo {
                extension: "glb",
                label: "glTF Binary (*.glb)",
                default_filename: "model.glb",
                can_load: true,
                can_export: true,
            },
            FileFormatInfo {
                extension: "gltf",
                label: "glTF Embedded (*.gltf)",
                default_filename: "model.gltf",
                can_load: true,
                can_export: true,
            },
        ]
    }

    fn can_load(&self, path: &Path, header: &[u8]) -> bool {
        if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
            if ext.eq_ignore_ascii_case("gltf") || ext.eq_ignore_ascii_case("glb") { return true }
        }
        if header.starts_with(b"glTF") { return true }
        false
    }

    fn load(&self, path: &Path, _reader: Box<dyn Read>) -> Result<MeshData, String> {
        let result = if path.exists() {
            gltf::import(path).map_err(|e| format!("gltf parse error: {}", e))
        } else {
            let data = std::fs::read(path).map_err(|e| format!("open failed: {}", e))?;
            gltf::import_slice(&data).map_err(|e| format!("gltf parse error: {}", e))
        };

        match result {
            Ok((gltf_doc, buffers, images)) => {
                let loaded_images: Vec<Arc<TextureData>> = images
                    .into_iter()
                    .map(|img| {
                        let rgba = match img.format {
                            gltf::image::Format::R8G8B8A8 => img.pixels,
                            gltf::image::Format::R8G8B8 => {
                                let mut rgba = Vec::with_capacity((img.width * img.height * 4) as usize);
                                for rgb in img.pixels.chunks(3) {
                                    rgba.extend_from_slice(&[rgb[0], rgb[1], rgb[2], 255]);
                                }
                                rgba
                            }
                            gltf::image::Format::R8 => {
                                let mut rgba = Vec::with_capacity((img.width * img.height * 4) as usize);
                                for &r in &img.pixels {
                                    rgba.extend_from_slice(&[r, r, r, 255]);
                                }
                                rgba
                            }
                            _ => {
                                let mut rgba = Vec::with_capacity((img.width * img.height * 4) as usize);
                                for _ in 0..(img.width * img.height) {
                                    rgba.extend_from_slice(&[255, 255, 255, 255]);
                                }
                                rgba
                            }
                        };
                        Arc::new(TextureData {
                            width: img.width,
                            height: img.height,
                            rgba,
                        })
                    })
                    .collect();

                let mut positions = Vec::new();
                let mut normals = Vec::new();
                let mut uvs = Vec::new();
                let mut indices = Vec::new();
                let mut submeshes = Vec::new();

                fn process_primitive(
                    prim: &gltf::Primitive,
                    node_index: usize,
                    world_transform: Mat4,
                    buffers: &[gltf::buffer::Data],
                    loaded_images: &[Arc<TextureData>],
                    positions: &mut Vec<[f32; 3]>,
                    normals: &mut Vec<[f32; 3]>,
                    uvs: &mut Vec<[f32; 2]>,
                    indices: &mut Vec<[u32; 3]>,
                    submeshes: &mut Vec<SubMesh>,
                ) {
                    let base_vertex = positions.len() as u32;
                    let mat = prim.material();
                    let pbr = mat.pbr_metallic_roughness();
                    let base_raw = pbr.base_color_factor();
                    let color = Some([
                        base_raw[0].clamp(0.0, 1.0),
                        base_raw[1].clamp(0.0, 1.0),
                        base_raw[2].clamp(0.0, 1.0),
                        base_raw[3],
                    ]);
                    let texture = pbr.base_color_texture().and_then(|t| {
                        loaded_images.get(t.texture().source().index()).cloned()
                    });

                    let mut sub_positions = Vec::new();
                    let mut sub_rest_positions = Vec::new();
                    let mut sub_normals = Vec::new();
                    let mut sub_rest_normals = Vec::new();
                    let mut sub_uvs = Vec::new();
                    let mut sub_indices = Vec::new();

                    let r = prim.reader(|buffer| Some(&buffers[buffer.index()]));
                    if let Some(iter) = r.read_positions() {
                        for p in iter {
                            sub_rest_positions.push([p[0], p[1], p[2]]);
                            let p_gltf = world_transform.transform_point3(Vec3::new(p[0], p[1], p[2]));
                            // Convert glTF (+Y up, +Z out) to Mesh viewer (+Z up, -Y front)
                            let pos = [p_gltf.x, -p_gltf.z, p_gltf.y];
                            sub_positions.push(pos);
                            positions.push(pos);
                        }
                    }
                    if let Some(iter) = r.read_normals() {
                        let normal_mat = world_transform.inverse().transpose();
                        for n in iter {
                            sub_rest_normals.push([n[0], n[1], n[2]]);
                            let n_gltf = normal_mat.transform_vector3(Vec3::new(n[0], n[1], n[2])).normalize_or_zero();
                            let norm = [n_gltf.x, -n_gltf.z, n_gltf.y];
                            sub_normals.push(norm);
                            normals.push(norm);
                        }
                    }
                    if let Some(iter) = r.read_tex_coords(0) {
                        for uv in iter.into_f32() {
                            sub_uvs.push([uv[0], uv[1]]);
                            uvs.push([uv[0], uv[1]]);
                        }
                    } else {
                        for _ in 0..sub_positions.len() {
                            sub_uvs.push([0.0, 0.0]);
                            uvs.push([0.0, 0.0]);
                        }
                    }

                    if let Some(read_indices) = r.read_indices() {
                        let collected: Vec<u32> = read_indices.into_u32().collect();
                        for chunk in collected.chunks(3) {
                            if chunk.len() == 3 {
                                let tri = [chunk[0], chunk[1], chunk[2]];
                                sub_indices.push(tri);
                                indices.push([tri[0] + base_vertex, tri[1] + base_vertex, tri[2] + base_vertex]);
                            }
                        }
                    } else {
                        let count = sub_positions.len() as u32;
                        for i in (0..count).step_by(3) {
                            if i + 2 < count {
                                let tri = [i, i + 1, i + 2];
                                sub_indices.push(tri);
                                indices.push([tri[0] + base_vertex, tri[1] + base_vertex, tri[2] + base_vertex]);
                            }
                        }
                    }

                    let joint_indices: Option<Vec<[u16; 4]>> = r.read_joints(0).map(|iter| {
                        iter.into_u16().collect()
                    });
                    let joint_weights: Option<Vec<[f32; 4]>> = r.read_weights(0).map(|iter| {
                        iter.into_f32().collect()
                    });

                    submeshes.push(SubMesh {
                        positions: sub_positions,
                        rest_positions: sub_rest_positions,
                        normals: sub_normals,
                        rest_normals: sub_rest_normals,
                        uvs: sub_uvs,
                        indices: sub_indices,
                        color,
                        texture,
                        joint_indices,
                        joint_weights,
                        node_index: Some(node_index),
                    });
                }

                fn process_node(
                    node: &gltf::Node,
                    parent_transform: Mat4,
                    buffers: &[gltf::buffer::Data],
                    loaded_images: &[Arc<TextureData>],
                    positions: &mut Vec<[f32; 3]>,
                    normals: &mut Vec<[f32; 3]>,
                    uvs: &mut Vec<[f32; 2]>,
                    indices: &mut Vec<[u32; 3]>,
                    submeshes: &mut Vec<SubMesh>,
                ) {
                    let local_matrix = Mat4::from_cols_array_2d(&node.transform().matrix());
                    let world_transform = parent_transform * local_matrix;

                    if let Some(mesh) = node.mesh() {
                        for prim in mesh.primitives() {
                            process_primitive(
                                &prim,
                                node.index(),
                                world_transform,
                                buffers,
                                loaded_images,
                                positions,
                                normals,
                                uvs,
                                indices,
                                submeshes,
                            );
                        }
                    }

                    for child in node.children() {
                        process_node(
                            &child,
                            world_transform,
                            buffers,
                            loaded_images,
                            positions,
                            normals,
                            uvs,
                            indices,
                            submeshes,
                        );
                    }
                }

                let default_scene = gltf_doc.default_scene().or_else(|| gltf_doc.scenes().next());
                if let Some(scene) = default_scene {
                    for node in scene.nodes() {
                        process_node(
                            &node,
                            Mat4::IDENTITY,
                            &buffers,
                            &loaded_images,
                            &mut positions,
                            &mut normals,
                            &mut uvs,
                            &mut indices,
                            &mut submeshes,
                        );
                    }
                } else {
                    for mesh in gltf_doc.meshes() {
                        for prim in mesh.primitives() {
                            process_primitive(
                                &prim,
                                0,
                                Mat4::IDENTITY,
                                &buffers,
                                &loaded_images,
                                &mut positions,
                                &mut normals,
                                &mut uvs,
                                &mut indices,
                                &mut submeshes,
                            );
                        }
                    }
                }

                // Extract scene nodes hierarchy
                let mut nodes = Vec::new();
                for node in gltf_doc.nodes() {
                    let (t, r, s) = node.transform().decomposed();
                    nodes.push(SceneNode {
                        name: node.name().unwrap_or("").to_string(),
                        parent: None,
                        children: node.children().map(|c| c.index()).collect(),
                        initial_translation: [t[0], t[1], t[2]],
                        initial_rotation: [r[0], r[1], r[2], r[3]],
                        initial_scale: [s[0], s[1], s[2]],
                        mesh_index: node.mesh().map(|m| m.index()),
                    });
                }
                for i in 0..nodes.len() {
                    let chs = nodes[i].children.clone();
                    for ch in chs {
                        if ch < nodes.len() {
                            nodes[ch].parent = Some(i);
                        }
                    }
                }

                // Extract skins & inverse bind matrices
                let mut skins = Vec::new();
                for skin in gltf_doc.skins() {
                    let name = skin.name().unwrap_or("").to_string();
                    let r = skin.reader(|buffer| Some(&buffers[buffer.index()]));
                    let inv_bind_matrices: Vec<[[f32; 4]; 4]> = r
                        .read_inverse_bind_matrices()
                        .map(|iter| iter.collect())
                        .unwrap_or_default();
                    let mut joints = Vec::new();
                    for (i, j_node) in skin.joints().enumerate() {
                        let inv_bind = inv_bind_matrices
                            .get(i)
                            .copied()
                            .unwrap_or(Mat4::IDENTITY.to_cols_array_2d());
                        joints.push(Joint {
                            node_index: j_node.index(),
                            inverse_bind_matrix: inv_bind,
                        });
                    }
                    skins.push(Skin { name, joints });
                }

                // Extract animation clips & channels
                let mut animations = Vec::new();
                for anim in gltf_doc.animations() {
                    let name = anim.name().unwrap_or("Animation").to_string();
                    let mut channels = Vec::new();
                    let mut max_time: f32 = 0.0;

                    for ch in anim.channels() {
                        let target_node = ch.target().node().index();
                        let property = match ch.target().property() {
                            gltf::animation::Property::Translation => TargetProperty::Translation,
                            gltf::animation::Property::Rotation => TargetProperty::Rotation,
                            gltf::animation::Property::Scale => TargetProperty::Scale,
                            gltf::animation::Property::MorphTargetWeights => TargetProperty::Weights,
                        };
                        let sampler = ch.sampler();
                        let interpolation = match sampler.interpolation() {
                            gltf::animation::Interpolation::Linear => Interpolation::Linear,
                            gltf::animation::Interpolation::Step => Interpolation::Step,
                            gltf::animation::Interpolation::CubicSpline => Interpolation::CubicSpline,
                        };
                        let r = ch.reader(|buffer| Some(&buffers[buffer.index()]));
                        let timestamps: Vec<f32> = r.read_inputs().map(|it| it.collect()).unwrap_or_default();
                        for &t in &timestamps {
                            if t > max_time {
                                max_time = t;
                            }
                        }
                        let values: Vec<f32> = match r.read_outputs() {
                            Some(gltf::animation::util::ReadOutputs::Translations(iter)) => {
                                iter.flat_map(|v| [v[0], v[1], v[2]]).collect()
                            }
                            Some(gltf::animation::util::ReadOutputs::Rotations(iter)) => {
                                iter.into_f32().flat_map(|q| [q[0], q[1], q[2], q[3]]).collect()
                            }
                            Some(gltf::animation::util::ReadOutputs::Scales(iter)) => {
                                iter.flat_map(|v| [v[0], v[1], v[2]]).collect()
                            }
                            Some(gltf::animation::util::ReadOutputs::MorphTargetWeights(iter)) => {
                                iter.into_f32().collect()
                            }
                            None => Vec::new(),
                        };
                        if !timestamps.is_empty() && !values.is_empty() {
                            channels.push(AnimationChannel {
                                target_node,
                                property,
                                interpolation,
                                timestamps,
                                values,
                            });
                        }
                    }
                    if !channels.is_empty() {
                        animations.push(AnimationClip {
                            name,
                            duration: max_time,
                            channels,
                        });
                    }
                }

                let has_animation = !animations.is_empty();

                Ok(MeshData {
                    positions,
                    normals,
                    uvs,
                    indices,
                    submeshes,
                    source_path: None,
                    has_animation,
                    is_y_up: true,
                    nodes,
                    skins,
                    animations,
                })
            }
            Err(e) => Err(format!("gltf parse error: {}", e)),
        }
    }

    fn can_export(&self, path: &Path) -> bool {
        if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
            if ext.eq_ignore_ascii_case("gltf") || ext.eq_ignore_ascii_case("glb") {
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
            let n = if !mesh.normals.is_empty() {
                mesh.normals.clone()
            } else {
                compute_normals(&mesh.positions, &mesh.indices)
            };
            let u = if !mesh.uvs.is_empty() {
                mesh.uvs.clone()
            } else {
                vec![[0.0, 0.0]; mesh.positions.len()]
            };
            fallback_submesh = vec![SubMesh {
                positions: mesh.positions.clone(),
                normals: n,
                uvs: u,
                indices: mesh.indices.clone(),
                color: None,
                texture: None,
                ..Default::default()
            }];
            &fallback_submesh
        } else {
            return Err("Mesh contains no geometry to export".into());
        };

        let is_glb = path.extension().and_then(|s| s.to_str()).map(|e| e.eq_ignore_ascii_case("glb")).unwrap_or(true);
        let export_dir = path.parent().unwrap_or_else(|| Path::new("."));
        let base_name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("model");

        let mut bin_data = Vec::new();
        let mut buffer_views = Vec::new();
        let mut accessors = Vec::new();
        let mut primitives = Vec::new();
        let mut materials = Vec::new();
        let mut images = Vec::new();
        let mut textures = Vec::new();

        for (mat_idx, sub) in submeshes.iter().enumerate() {
            if sub.positions.is_empty() || sub.indices.is_empty() {
                continue;
            }

            let num_verts = sub.positions.len();
            let num_indices = sub.indices.len() * 3;

            let mut min_pos = [f32::INFINITY; 3];
            let mut max_pos = [f32::NEG_INFINITY; 3];

            // 1. Positions - If model is already Y-up and has rest positions, use them directly in glTF space
            let use_rest = !sub.rest_positions.is_empty();
            let pos_slice = if use_rest { &sub.rest_positions } else { &sub.positions };

            while bin_data.len() % 4 != 0 {
                bin_data.push(0);
            }
            let pos_offset = bin_data.len();
            for p in pos_slice {
                let p_gltf = if mesh.is_y_up && use_rest {
                    *p
                } else {
                    [p[0], p[2], -p[1]]
                };
                min_pos[0] = min_pos[0].min(p_gltf[0]);
                min_pos[1] = min_pos[1].min(p_gltf[1]);
                min_pos[2] = min_pos[2].min(p_gltf[2]);
                max_pos[0] = max_pos[0].max(p_gltf[0]);
                max_pos[1] = max_pos[1].max(p_gltf[1]);
                max_pos[2] = max_pos[2].max(p_gltf[2]);

                bin_data.extend_from_slice(&p_gltf[0].to_le_bytes());
                bin_data.extend_from_slice(&p_gltf[1].to_le_bytes());
                bin_data.extend_from_slice(&p_gltf[2].to_le_bytes());
            }
            let pos_len = bin_data.len() - pos_offset;
            let pos_view_idx = buffer_views.len();
            buffer_views.push(serde_json::json!({
                "buffer": 0,
                "byteOffset": pos_offset,
                "byteLength": pos_len,
                "target": 34962
            }));
            let pos_acc_idx = accessors.len();
            accessors.push(serde_json::json!({
                "bufferView": pos_view_idx,
                "byteOffset": 0,
                "componentType": 5126,
                "count": num_verts,
                "type": "VEC3",
                "min": min_pos,
                "max": max_pos
            }));

            // 2. Normals
            let computed_normals;
            let norm_slice: &[[f32; 3]] = if use_rest && sub.rest_normals.len() == sub.positions.len() {
                &sub.rest_normals
            } else if sub.normals.len() == sub.positions.len() {
                &sub.normals
            } else {
                computed_normals = compute_normals(&sub.positions, &sub.indices);
                &computed_normals
            };

            while bin_data.len() % 4 != 0 {
                bin_data.push(0);
            }
            let norm_offset = bin_data.len();
            for n in norm_slice {
                let n_gltf = if mesh.is_y_up && use_rest {
                    *n
                } else {
                    [n[0], n[2], -n[1]]
                };
                bin_data.extend_from_slice(&n_gltf[0].to_le_bytes());
                bin_data.extend_from_slice(&n_gltf[1].to_le_bytes());
                bin_data.extend_from_slice(&n_gltf[2].to_le_bytes());
            }
            let norm_len = bin_data.len() - norm_offset;
            let norm_view_idx = buffer_views.len();
            buffer_views.push(serde_json::json!({
                "buffer": 0,
                "byteOffset": norm_offset,
                "byteLength": norm_len,
                "target": 34962
            }));
            let norm_acc_idx = accessors.len();
            accessors.push(serde_json::json!({
                "bufferView": norm_view_idx,
                "byteOffset": 0,
                "componentType": 5126,
                "count": num_verts,
                "type": "VEC3"
            }));

            // 3. TexCoords (UVs)
            while bin_data.len() % 4 != 0 {
                bin_data.push(0);
            }
            let uv_offset = bin_data.len();
            for i in 0..num_verts {
                let uv = sub.uvs.get(i).copied().unwrap_or([0.0, 0.0]);
                bin_data.extend_from_slice(&uv[0].to_le_bytes());
                bin_data.extend_from_slice(&uv[1].to_le_bytes());
            }
            let uv_len = bin_data.len() - uv_offset;
            let uv_view_idx = buffer_views.len();
            buffer_views.push(serde_json::json!({
                "buffer": 0,
                "byteOffset": uv_offset,
                "byteLength": uv_len,
                "target": 34962
            }));
            let uv_acc_idx = accessors.len();
            accessors.push(serde_json::json!({
                "bufferView": uv_view_idx,
                "byteOffset": 0,
                "componentType": 5126,
                "count": num_verts,
                "type": "VEC2"
            }));

            // 4. Indices
            while bin_data.len() % 4 != 0 {
                bin_data.push(0);
            }
            let idx_offset = bin_data.len();
            for tri in &sub.indices {
                bin_data.extend_from_slice(&tri[0].to_le_bytes());
                bin_data.extend_from_slice(&tri[1].to_le_bytes());
                bin_data.extend_from_slice(&tri[2].to_le_bytes());
            }
            let idx_len = bin_data.len() - idx_offset;
            let idx_view_idx = buffer_views.len();
            buffer_views.push(serde_json::json!({
                "buffer": 0,
                "byteOffset": idx_offset,
                "byteLength": idx_len,
                "target": 34963
            }));
            let idx_acc_idx = accessors.len();
            accessors.push(serde_json::json!({
                "bufferView": idx_view_idx,
                "byteOffset": 0,
                "componentType": 5125,
                "count": num_indices,
                "type": "SCALAR"
            }));

            // 5. Skinning attributes (JOINTS_0 and WEIGHTS_0)
            let mut joints_acc_idx = None;
            let mut weights_acc_idx = None;
            if let (Some(j_indices), Some(j_weights)) = (&sub.joint_indices, &sub.joint_weights) {
                while bin_data.len() % 4 != 0 {
                    bin_data.push(0);
                }
                let joints_offset = bin_data.len();
                for i in 0..num_verts {
                    let quads = j_indices.get(i).copied().unwrap_or([0, 0, 0, 0]);
                    bin_data.extend_from_slice(&quads[0].to_le_bytes());
                    bin_data.extend_from_slice(&quads[1].to_le_bytes());
                    bin_data.extend_from_slice(&quads[2].to_le_bytes());
                    bin_data.extend_from_slice(&quads[3].to_le_bytes());
                }
                let joints_len = bin_data.len() - joints_offset;
                let joints_view_idx = buffer_views.len();
                buffer_views.push(serde_json::json!({
                    "buffer": 0,
                    "byteOffset": joints_offset,
                    "byteLength": joints_len,
                    "target": 34962
                }));
                let j_acc = accessors.len();
                accessors.push(serde_json::json!({
                    "bufferView": joints_view_idx,
                    "byteOffset": 0,
                    "componentType": 5123,
                    "count": num_verts,
                    "type": "VEC4"
                }));
                joints_acc_idx = Some(j_acc);

                while bin_data.len() % 4 != 0 {
                    bin_data.push(0);
                }
                let weights_offset = bin_data.len();
                for i in 0..num_verts {
                    let wgts = j_weights.get(i).copied().unwrap_or([1.0, 0.0, 0.0, 0.0]);
                    bin_data.extend_from_slice(&wgts[0].to_le_bytes());
                    bin_data.extend_from_slice(&wgts[1].to_le_bytes());
                    bin_data.extend_from_slice(&wgts[2].to_le_bytes());
                    bin_data.extend_from_slice(&wgts[3].to_le_bytes());
                }
                let weights_len = bin_data.len() - weights_offset;
                let weights_view_idx = buffer_views.len();
                buffer_views.push(serde_json::json!({
                    "buffer": 0,
                    "byteOffset": weights_offset,
                    "byteLength": weights_len,
                    "target": 34962
                }));
                let w_acc = accessors.len();
                accessors.push(serde_json::json!({
                    "bufferView": weights_view_idx,
                    "byteOffset": 0,
                    "componentType": 5126,
                    "count": num_verts,
                    "type": "VEC4"
                }));
                weights_acc_idx = Some(w_acc);
            }

            // 6. Material with diffuse texture
            let color = sub.color.unwrap_or([1.0, 1.0, 1.0, 1.0]);
            let mut pbr = serde_json::json!({
                "baseColorFactor": [color[0], color[1], color[2], color[3]],
                "metallicFactor": 0.0,
                "roughnessFactor": 0.5
            });

            if let Some(ref tex) = sub.texture {
                let img_idx = images.len();
                if is_glb {
                    let mut png_bytes = Vec::new();
                    if let Some(img_buf) = image::RgbaImage::from_raw(tex.width, tex.height, tex.rgba.clone()) {
                        let mut cursor = std::io::Cursor::new(&mut png_bytes);
                        let _ = img_buf.write_to(&mut cursor, image::ImageFormat::Png);
                    }
                    while bin_data.len() % 4 != 0 {
                        bin_data.push(0);
                    }
                    let img_offset = bin_data.len();
                    bin_data.extend_from_slice(&png_bytes);
                    let img_len = bin_data.len() - img_offset;
                    let view_idx = buffer_views.len();
                    buffer_views.push(serde_json::json!({
                        "buffer": 0,
                        "byteOffset": img_offset,
                        "byteLength": img_len
                    }));
                    images.push(serde_json::json!({
                        "bufferView": view_idx,
                        "mimeType": "image/png"
                    }));
                } else {
                    let tex_filename = format!("{}_tex_{}.png", base_name, mat_idx + 1);
                    let tex_path = export_dir.join(&tex_filename);
                    if let Some(img_buf) = image::RgbaImage::from_raw(tex.width, tex.height, tex.rgba.clone()) {
                        let _ = img_buf.save(&tex_path);
                    }
                    images.push(serde_json::json!({
                        "uri": tex_filename
                    }));
                }
                let t_idx = textures.len();
                textures.push(serde_json::json!({
                    "source": img_idx
                }));
                pbr["baseColorTexture"] = serde_json::json!({
                    "index": t_idx
                });
            }

            materials.push(serde_json::json!({
                "name": format!("Material_{}", mat_idx + 1),
                "pbrMetallicRoughness": pbr,
                "doubleSided": true
            }));

            // 7. Primitive
            let mut prim_attrs = serde_json::json!({
                "POSITION": pos_acc_idx,
                "NORMAL": norm_acc_idx,
                "TEXCOORD_0": uv_acc_idx
            });
            if let (Some(ja), Some(wa)) = (joints_acc_idx, weights_acc_idx) {
                prim_attrs["JOINTS_0"] = serde_json::json!(ja);
                prim_attrs["WEIGHTS_0"] = serde_json::json!(wa);
            }

            primitives.push(serde_json::json!({
                "attributes": prim_attrs,
                "indices": idx_acc_idx,
                "material": mat_idx,
                "mode": 4
            }));
        }

        if primitives.is_empty() {
            return Err("No primitives generated for export".into());
        }

        // Skins
        let mut gltf_skins = Vec::new();
        if let Some(skin) = mesh.skins.first() {
            if !skin.joints.is_empty() {
                while bin_data.len() % 4 != 0 {
                    bin_data.push(0);
                }
                let ibm_offset = bin_data.len();
                for joint in &skin.joints {
                    for col in &joint.inverse_bind_matrix {
                        for &val in col {
                            bin_data.extend_from_slice(&val.to_le_bytes());
                        }
                    }
                }
                let ibm_len = bin_data.len() - ibm_offset;
                let ibm_view_idx = buffer_views.len();
                buffer_views.push(serde_json::json!({
                    "buffer": 0,
                    "byteOffset": ibm_offset,
                    "byteLength": ibm_len
                }));
                let ibm_acc_idx = accessors.len();
                accessors.push(serde_json::json!({
                    "bufferView": ibm_view_idx,
                    "byteOffset": 0,
                    "componentType": 5126,
                    "count": skin.joints.len(),
                    "type": "MAT4"
                }));
                gltf_skins.push(serde_json::json!({
                    "name": if skin.name.is_empty() { "Armature".to_string() } else { skin.name.clone() },
                    "joints": skin.joints.iter().map(|j| j.node_index).collect::<Vec<_>>(),
                    "inverseBindMatrices": ibm_acc_idx
                }));
            }
        }

        // Nodes
        let mut gltf_nodes = Vec::new();
        let scene_roots;
        if !mesh.nodes.is_empty() {
            let mesh_node_idx = mesh.nodes.iter().position(|n| n.mesh_index.is_some()).unwrap_or(0);
            for (i, node) in mesh.nodes.iter().enumerate() {
                let mut n_json = serde_json::json!({
                    "name": if node.name.is_empty() { format!("Node_{}", i) } else { node.name.clone() },
                    "translation": node.initial_translation,
                    "rotation": node.initial_rotation,
                    "scale": node.initial_scale,
                });
                if !node.children.is_empty() {
                    n_json["children"] = serde_json::json!(node.children);
                }
                if i == mesh_node_idx {
                    n_json["mesh"] = serde_json::json!(0);
                    if !gltf_skins.is_empty() {
                        n_json["skin"] = serde_json::json!(0);
                    }
                }
                gltf_nodes.push(n_json);
            }
            let roots: Vec<usize> = mesh.nodes.iter().enumerate()
                .filter_map(|(i, n)| if n.parent.is_none() { Some(i) } else { None })
                .collect();
            scene_roots = if roots.is_empty() { vec![0] } else { roots };
        } else {
            gltf_nodes.push(serde_json::json!({
                "name": "Model",
                "mesh": 0
            }));
            scene_roots = vec![0];
        }

        // Animations
        let mut gltf_animations = Vec::new();
        for clip in &mesh.animations {
            let mut gltf_channels = Vec::new();
            let mut gltf_samplers = Vec::new();

            for ch in &clip.channels {
                if ch.timestamps.is_empty() || ch.values.is_empty() {
                    continue;
                }
                while bin_data.len() % 4 != 0 {
                    bin_data.push(0);
                }
                let time_offset = bin_data.len();
                let mut min_t = f32::INFINITY;
                let mut max_t = f32::NEG_INFINITY;
                for &t in &ch.timestamps {
                    min_t = min_t.min(t);
                    max_t = max_t.max(t);
                    bin_data.extend_from_slice(&t.to_le_bytes());
                }
                let time_len = bin_data.len() - time_offset;
                let time_view_idx = buffer_views.len();
                buffer_views.push(serde_json::json!({
                    "buffer": 0,
                    "byteOffset": time_offset,
                    "byteLength": time_len
                }));
                let time_acc_idx = accessors.len();
                accessors.push(serde_json::json!({
                    "bufferView": time_view_idx,
                    "byteOffset": 0,
                    "componentType": 5126,
                    "count": ch.timestamps.len(),
                    "type": "SCALAR",
                    "min": [min_t],
                    "max": [max_t]
                }));

                while bin_data.len() % 4 != 0 {
                    bin_data.push(0);
                }
                let val_offset = bin_data.len();
                for &v in &ch.values {
                    bin_data.extend_from_slice(&v.to_le_bytes());
                }
                let val_len = bin_data.len() - val_offset;
                let val_view_idx = buffer_views.len();
                buffer_views.push(serde_json::json!({
                    "buffer": 0,
                    "byteOffset": val_offset,
                    "byteLength": val_len
                }));
                let (acc_type, path_str) = match ch.property {
                    TargetProperty::Translation => ("VEC3", "translation"),
                    TargetProperty::Rotation => ("VEC4", "rotation"),
                    TargetProperty::Scale => ("VEC3", "scale"),
                    TargetProperty::Weights => ("SCALAR", "weights"),
                };
                let val_acc_idx = accessors.len();
                accessors.push(serde_json::json!({
                    "bufferView": val_view_idx,
                    "byteOffset": 0,
                    "componentType": 5126,
                    "count": ch.timestamps.len(),
                    "type": acc_type
                }));

                let sampler_idx = gltf_samplers.len();
                gltf_samplers.push(serde_json::json!({
                    "input": time_acc_idx,
                    "interpolation": match ch.interpolation {
                        Interpolation::Step => "STEP",
                        Interpolation::CubicSpline => "CUBICSPLINE",
                        _ => "LINEAR"
                    },
                    "output": val_acc_idx
                }));

                gltf_channels.push(serde_json::json!({
                    "sampler": sampler_idx,
                    "target": {
                        "node": ch.target_node,
                        "path": path_str
                    }
                }));
            }

            if !gltf_channels.is_empty() {
                gltf_animations.push(serde_json::json!({
                    "name": if clip.name.is_empty() { "Animation".to_string() } else { clip.name.clone() },
                    "channels": gltf_channels,
                    "samplers": gltf_samplers
                }));
            }
        }

        let total_bin_len = bin_data.len();
        let bin_filename = path.file_stem().and_then(|s| s.to_str()).unwrap_or("model").to_string() + ".bin";

        let buffers_json = if is_glb {
            serde_json::json!([
                { "byteLength": total_bin_len }
            ])
        } else {
            serde_json::json!([
                { "byteLength": total_bin_len, "uri": bin_filename }
            ])
        };

        let mut gltf_json = serde_json::json!({
            "asset": { "version": "2.0", "generator": "Mesh" },
            "scene": 0,
            "scenes": [ { "nodes": scene_roots } ],
            "nodes": gltf_nodes,
            "meshes": [
                {
                    "name": "Model",
                    "primitives": primitives
                }
            ],
            "materials": materials,
            "accessors": accessors,
            "bufferViews": buffer_views,
            "buffers": buffers_json
        });

        if !images.is_empty() {
            gltf_json["images"] = serde_json::json!(images);
            gltf_json["textures"] = serde_json::json!(textures);
        }
        if !gltf_skins.is_empty() {
            gltf_json["skins"] = serde_json::json!(gltf_skins);
        }
        if !gltf_animations.is_empty() {
            gltf_json["animations"] = serde_json::json!(gltf_animations);
        }

        use std::io::Write;
        if is_glb {
            let json_str = serde_json::to_string(&gltf_json).map_err(|e| format!("json err: {}", e))?;
            let mut json_bytes = json_str.into_bytes();
            while json_bytes.len() % 4 != 0 {
                json_bytes.push(b' ');
            }
            while bin_data.len() % 4 != 0 {
                bin_data.push(0);
            }

            let total_glb_len = 12 + 8 + json_bytes.len() + 8 + bin_data.len();

            let file = std::fs::File::create(path).map_err(|e| format!("create glb file failed: {}", e))?;
            let mut writer = std::io::BufWriter::new(file);

            // 1. Header (12 bytes)
            writer.write_all(b"glTF").map_err(|e| format!("write header: {}", e))?;
            writer.write_all(&2u32.to_le_bytes()).map_err(|e| format!("write version: {}", e))?;
            writer.write_all(&(total_glb_len as u32).to_le_bytes()).map_err(|e| format!("write length: {}", e))?;

            // 2. JSON Chunk
            writer.write_all(&(json_bytes.len() as u32).to_le_bytes()).map_err(|e| format!("write json len: {}", e))?;
            writer.write_all(b"JSON").map_err(|e| format!("write json tag: {}", e))?;
            writer.write_all(&json_bytes).map_err(|e| format!("write json data: {}", e))?;

            // 3. BIN Chunk
            writer.write_all(&(bin_data.len() as u32).to_le_bytes()).map_err(|e| format!("write bin len: {}", e))?;
            writer.write_all(b"BIN\0").map_err(|e| format!("write bin tag: {}", e))?;
            writer.write_all(&bin_data).map_err(|e| format!("write bin data: {}", e))?;

            writer.flush().map_err(|e| format!("flush: {}", e))?;
            println!("[glTF Format] Successfully exported GLB to '{}'", path.display());
        } else {
            // Write JSON glTF file
            let json_str = serde_json::to_string_pretty(&gltf_json).map_err(|e| format!("json err: {}", e))?;
            std::fs::write(path, json_str).map_err(|e| format!("write gltf failed: {}", e))?;

            // Write companion .bin file
            let bin_path = path.with_file_name(&bin_filename);
            std::fs::write(&bin_path, bin_data).map_err(|e| format!("write bin failed: {}", e))?;
            println!("[glTF Format] Successfully exported glTF to '{}' and '{}'", path.display(), bin_path.display());
        }

        Ok(())
    }

    fn supports_animation(&self) -> bool {
        println!("[glTF Format] Format natively supports animation tracks (skeletal/morph)");
        true
    }
}
