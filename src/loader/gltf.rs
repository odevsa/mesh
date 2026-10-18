use super::{Loader, MeshData, SubMesh, TextureData};
use std::path::Path;
use std::io::Read;
use std::sync::Arc;
use glam::{Mat4, Vec3};

pub struct GltfLoader {}

impl Loader for GltfLoader {
    fn name(&self) -> &str { "gltf" }

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

    fn export(&self, path: &Path, _mesh: &MeshData) -> Result<(), String> {
        println!("[glTF Format] Exporting mesh to '{}' (dummy print)", path.display());
        Ok(())
    }

    fn supports_animation(&self) -> bool {
        println!("[glTF Format] Format natively supports animation tracks (skeletal/morph)");
        true
    }
}
