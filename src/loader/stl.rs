use super::{Loader, MeshData, SubMesh};
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
                let positions: Vec<[f32; 3]> = im.vertices.iter().map(|v| [v[0] as f32, v[1] as f32, v[2] as f32]).collect();
                let indices: Vec<[u32; 3]> = im.faces.iter().map(|f| [f.vertices[0] as u32, f.vertices[1] as u32, f.vertices[2] as u32]).collect();

                let mut sub_positions = Vec::with_capacity(im.faces.len() * 3);
                let mut sub_normals = Vec::with_capacity(im.faces.len() * 3);
                let mut sub_indices = Vec::with_capacity(im.faces.len());

                for face in &im.faces {
                    let ia = face.vertices[0] as usize;
                    let ib = face.vertices[1] as usize;
                    let ic = face.vertices[2] as usize;
                    if ia < im.vertices.len() && ib < im.vertices.len() && ic < im.vertices.len() {
                        let va = im.vertices[ia];
                        let vb = im.vertices[ib];
                        let vc = im.vertices[ic];
                        sub_positions.push([va[0] as f32, va[1] as f32, va[2] as f32]);
                        sub_positions.push([vb[0] as f32, vb[1] as f32, vb[2] as f32]);
                        sub_positions.push([vc[0] as f32, vc[1] as f32, vc[2] as f32]);

                        let n = face.normal;
                        let fnorm = [n[0] as f32, n[1] as f32, n[2] as f32];
                        sub_normals.push(fnorm);
                        sub_normals.push(fnorm);
                        sub_normals.push(fnorm);

                        let base = (sub_positions.len() - 3) as u32;
                        sub_indices.push([base, base + 1, base + 2]);
                    }
                }

                let submesh = SubMesh {
                    positions: sub_positions,
                    normals: sub_normals,
                    indices: sub_indices,
                    color: None,
                };
                Ok(MeshData { positions, normals: Vec::new(), indices, submeshes: vec![submesh] })
            }
            Err(e) => Err(format!("stl parse error: {}", e)),
        }
    }
}
