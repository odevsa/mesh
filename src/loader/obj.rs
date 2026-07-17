use super::{Loader, MeshData};
use std::path::Path;
use std::io::Read;

pub struct ObjLoader {}

impl Loader for ObjLoader {
    fn name(&self) -> &str { "obj" }

    fn can_load(&self, path: &Path, _header: &[u8]) -> bool {
        if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
            if ext.eq_ignore_ascii_case("obj") { return true }
        }
        false
    }

    fn load(&self, path: &Path, reader: Box<dyn Read>) -> Result<MeshData, String> {
        use std::io::BufReader;
        let mut buf = Vec::new();
        let mut r = BufReader::new(reader);
        r.read_to_end(&mut buf).map_err(|e| format!("read: {}", e))?;
        let mut cursor = std::io::Cursor::new(buf);
        let material_loader = |p: &std::path::Path| -> tobj::MTLLoadResult {
            let base = path.parent().unwrap_or_else(|| std::path::Path::new("."));
            let mtl_path = base.join(p);
            if mtl_path.exists() {
            let f = std::fs::File::open(&mtl_path).map_err(|_| tobj::LoadError::OpenFileFailed)?;
                let mut br = std::io::BufReader::new(f);
                tobj::load_mtl_buf(&mut br)
            } else {
                Ok((Vec::new(), std::collections::HashMap::new()))
            }
        };

        match tobj::load_obj_buf(&mut cursor, true, material_loader) {
            Ok((models, _mtls)) => {
                let mut positions = Vec::new();
                let mut normals = Vec::new();
                let mut indices = Vec::new();
                for m in models {
                    let mesh = m.mesh;
                    for v in mesh.positions.chunks(3) {
                        positions.push([v[0] as f32, v[1] as f32, v[2] as f32]);
                    }
                    for n in mesh.normals.chunks(3) {
                        normals.push([n[0] as f32, n[1] as f32, n[2] as f32]);
                    }
                    for idx_chunk in mesh.indices.chunks(3) {
                        if idx_chunk.len() == 3 {
                            indices.push([idx_chunk[0] as u32, idx_chunk[1] as u32, idx_chunk[2] as u32]);
                        }
                    }
                }
                Ok(MeshData { positions, normals, indices })
            }
            Err(e) => Err(format!("obj parse error: {}", e)),
        }
    }
}
