use super::{Loader, MeshData};
use std::path::Path;
use std::io::Read;

pub struct FbxLoader {}

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
        let opts = ufbx::LoadOpts::default();
        let path_str = path.to_str().ok_or_else(|| "invalid UTF-8 path".to_string())?;
        let scene = ufbx::load_file(path_str, opts).map_err(|e| format!("fbx parse error: {:?}", e))?;

        let mut positions = Vec::new();
        let mut normals = Vec::new();
        let mut indices = Vec::new();

        for mesh in &scene.meshes {
            let base_vertex = positions.len() as u32;

            for p in &mesh.vertices {
                positions.push([p.x as f32, p.y as f32, p.z as f32]);
            }

            if mesh.vertex_normal.exists {
                for n in &mesh.vertex_normal.values {
                    normals.push([n.x as f32, n.y as f32, n.z as f32]);
                }
            }

            for face in &mesh.faces {
                if face.num_indices >= 3 {
                    for t in 0..(face.num_indices - 2) {
                        let i0 = mesh.vertex_indices[(face.index_begin + 0) as usize];
                        let i1 = mesh.vertex_indices[(face.index_begin + t + 1) as usize];
                        let i2 = mesh.vertex_indices[(face.index_begin + t + 2) as usize];
                        indices.push([i0 + base_vertex, i1 + base_vertex, i2 + base_vertex]);
                    }
                }
            }
        }

        if positions.is_empty() {
            return Err("fbx: no positions found in model".into());
        }

        Ok(MeshData { positions, normals, indices })
    }
}
