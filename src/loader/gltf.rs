use super::{Loader, MeshData};
use std::path::Path;
use std::io::Read;

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
        let data = std::fs::read(path).map_err(|e| format!("open failed: {}", e))?;
        let result = if path.extension().and_then(|s| s.to_str()).map(|s| s.eq_ignore_ascii_case("glb")).unwrap_or(false) || data.starts_with(b"glTF") {
            gltf::import_slice(&data)
        } else {
            gltf::import_slice(&data)
        };

        match result {
            Ok((gltf_doc, buffers, images)) => {
                let loaded_images: Vec<std::sync::Arc<super::TextureData>> = images
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
                            _ => {
                                let mut rgba = Vec::with_capacity((img.width * img.height * 4) as usize);
                                for _ in 0..(img.width * img.height) {
                                    rgba.extend_from_slice(&[255, 255, 255, 255]);
                                }
                                rgba
                            }
                        };
                        std::sync::Arc::new(super::TextureData {
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

                for mesh in gltf_doc.meshes() {
                    for prim in mesh.primitives() {
                        let base_vertex = positions.len() as u32;
                        let mat = prim.material();
                        let pbr = mat.pbr_metallic_roughness();
                        let base_raw = pbr.base_color_factor();
                        let color = Some([
                            base_raw[0].clamp(0.0, 1.0).powf(1.0 / 2.2),
                            base_raw[1].clamp(0.0, 1.0).powf(1.0 / 2.2),
                            base_raw[2].clamp(0.0, 1.0).powf(1.0 / 2.2),
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
                                let pos = [p[0] as f32, -p[2] as f32, p[1] as f32];
                                sub_positions.push(pos);
                                positions.push(pos);
                            }
                        }
                        if let Some(iter) = r.read_normals() {
                            for n in iter {
                                let norm = [n[0] as f32, -n[2] as f32, n[1] as f32];
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

                        submeshes.push(super::SubMesh {
                            positions: sub_positions,
                            normals: sub_normals,
                            uvs: sub_uvs,
                            indices: sub_indices,
                            color,
                            texture,
                        });
                    }
                }

                Ok(MeshData { positions, normals, uvs, indices, submeshes })
            }
            Err(e) => Err(format!("gltf parse error: {}", e)),
        }
    }
}
