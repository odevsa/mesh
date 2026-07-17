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
            Ok((gltf_doc, buffers, _images)) => {
                let mut positions = Vec::new();
                let mut normals = Vec::new();
                let mut indices = Vec::new();

                for mesh in gltf_doc.meshes() {
                    for prim in mesh.primitives() {
                        let r = prim.reader(|buffer| Some(&buffers[buffer.index()]));
                        if let Some(iter) = r.read_positions() {
                            for p in iter {
                                positions.push([p[0] as f32, p[1] as f32, p[2] as f32]);
                            }
                        }
                        if let Some(iter) = r.read_normals() {
                            for n in iter {
                                normals.push([n[0] as f32, n[1] as f32, n[2] as f32]);
                            }
                        }
                        if let Some(read_indices) = r.read_indices() {
                            let collected: Vec<u32> = read_indices.into_u32().collect();
                            for chunk in collected.chunks(3) {
                                if chunk.len() == 3 {
                                    indices.push([chunk[0], chunk[1], chunk[2]]);
                                }
                            }
                        } else {
                            let count = positions.len() as u32;
                            for i in (0..count).step_by(3) {
                                indices.push([i, i + 1, i + 2]);
                            }
                        }
                    }
                }

                Ok(MeshData { positions, normals, indices })
            }
            Err(e) => Err(format!("gltf parse error: {}", e)),
        }
    }
}
