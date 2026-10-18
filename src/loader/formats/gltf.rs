use crate::loader::{compute_normals, FileFormatInfo, Loader, MeshData, SubMesh, TextureData};
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
                    let mut sub_normals = Vec::new();
                    let mut sub_uvs = Vec::new();
                    let mut sub_indices = Vec::new();

                    let r = prim.reader(|buffer| Some(&buffers[buffer.index()]));
                    if let Some(iter) = r.read_positions() {
                        for p in iter {
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

                    submeshes.push(SubMesh {
                        positions: sub_positions,
                        normals: sub_normals,
                        uvs: sub_uvs,
                        indices: sub_indices,
                        color,
                        texture,
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

                Ok(MeshData {
                    positions,
                    normals,
                    uvs,
                    indices,
                    submeshes,
                    ..Default::default()
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
            }];
            &fallback_submesh
        } else {
            return Err("Mesh contains no geometry to export".into());
        };

        let is_glb = path.extension().and_then(|s| s.to_str()).map(|e| e.eq_ignore_ascii_case("glb")).unwrap_or(true);

        let mut bin_data = Vec::new();
        let mut buffer_views = Vec::new();
        let mut accessors = Vec::new();
        let mut primitives = Vec::new();
        let mut materials = Vec::new();

        for (mat_idx, sub) in submeshes.iter().enumerate() {
            if sub.positions.is_empty() || sub.indices.is_empty() {
                continue;
            }

            let num_verts = sub.positions.len();
            let num_indices = sub.indices.len() * 3;

            let mut min_pos = [f32::INFINITY; 3];
            let mut max_pos = [f32::NEG_INFINITY; 3];

            // 1. Positions - Convert Mesh viewer (+Z up, -Y front) to glTF (+Y up, +Z out)
            while bin_data.len() % 4 != 0 {
                bin_data.push(0);
            }
            let pos_offset = bin_data.len();
            for p in &sub.positions {
                let p_gltf = [p[0], p[2], -p[1]];
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

            // 2. Normals - Convert to glTF (+Y up, +Z out)
            let sub_normals: Vec<[f32; 3]> = if sub.normals.len() == sub.positions.len() {
                sub.normals.clone()
            } else {
                compute_normals(&sub.positions, &sub.indices)
            };

            while bin_data.len() % 4 != 0 {
                bin_data.push(0);
            }
            let norm_offset = bin_data.len();
            for n in &sub_normals {
                let n_gltf = [n[0], n[2], -n[1]];
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

            // 5. Material
            let color = sub.color.unwrap_or([0.8, 0.8, 0.8, 1.0]);
            materials.push(serde_json::json!({
                "name": format!("Material_{}", mat_idx + 1),
                "pbrMetallicRoughness": {
                    "baseColorFactor": [color[0], color[1], color[2], color[3]],
                    "metallicFactor": 0.0,
                    "roughnessFactor": 0.5
                },
                "doubleSided": true
            }));

            // 6. Primitive
            primitives.push(serde_json::json!({
                "attributes": {
                    "POSITION": pos_acc_idx,
                    "NORMAL": norm_acc_idx,
                    "TEXCOORD_0": uv_acc_idx
                },
                "indices": idx_acc_idx,
                "material": mat_idx,
                "mode": 4
            }));
        }

        if primitives.is_empty() {
            return Err("No primitives generated for export".into());
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

        let gltf_json = serde_json::json!({
            "asset": { "version": "2.0", "generator": "Mesh" },
            "scene": 0,
            "scenes": [ { "nodes": [0] } ],
            "nodes": [ { "mesh": 0 } ],
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
