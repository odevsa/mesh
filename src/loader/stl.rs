use super::{Loader, MeshData};
use std::path::Path;
use std::io::Read;

pub struct StlLoader {}

impl Loader for StlLoader {
    fn name(&self) -> &str { "stl" }

    fn can_load(&self, path: &Path, header: &[u8]) -> bool {
        if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
            if ext.eq_ignore_ascii_case("stl") { return true }
        }
        if header.starts_with(b"solid") { return true }
        false
    }

    fn load(&self, _path: &Path, reader: Box<dyn Read>) -> Result<MeshData, String> {
        let mut rdr = std::io::BufReader::new(reader);
        let mut buf = Vec::new();
        rdr.read_to_end(&mut buf).map_err(|e| format!("read: {}", e))?;
        let mut cursor = std::io::Cursor::new(buf);
        match stl_io::read_stl(&mut cursor) {
            Ok(im) => {
                let mut positions = Vec::new();
                let mut normals = Vec::new();
                let mut indices = Vec::new();
                for face in im.faces.iter() {
                    let ia = face.vertices[0] as usize;
                    let ib = face.vertices[1] as usize;
                    let ic = face.vertices[2] as usize;
                    for &i in &[ia, ib, ic] {
                        let v = im.vertices[i];
                        positions.push([v[0] as f32, v[1] as f32, v[2] as f32]);
                    }
                    let n = face.normal;
                    normals.push([n[0] as f32, n[1] as f32, n[2] as f32]);
                    normals.push([n[0] as f32, n[1] as f32, n[2] as f32]);
                    normals.push([n[0] as f32, n[1] as f32, n[2] as f32]);
                    let base = (positions.len() - 3) as u32;
                    indices.push([base, base + 1, base + 2]);
                }
                Ok(MeshData { positions, normals, indices })
            }
            Err(e) => Err(format!("stl parse error: {}", e)),
        }
    }
}
