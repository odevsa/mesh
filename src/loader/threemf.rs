use super::{Loader, MeshData};
use std::path::Path;
use std::io::Read;

pub struct ThreemfLoader {}

impl Loader for ThreemfLoader {
    fn name(&self) -> &str { "3mf" }

    fn can_load(&self, path: &Path, header: &[u8]) -> bool {
        if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
            if ext.eq_ignore_ascii_case("3mf") { return true }
        }
        if header.starts_with(b"PK") { return true }
        false
    }

    fn load(&self, _path: &Path, reader: Box<dyn Read>) -> Result<MeshData, String> {
        let mut file_buf = Vec::new();
        let mut r = std::io::BufReader::new(reader);
        r.read_to_end(&mut file_buf).map_err(|e| format!("read: {}", e))?;
        let cursor = std::io::Cursor::new(file_buf);

        let mut archive = zip::ZipArchive::new(cursor).map_err(|e| format!("zip open: {}", e))?;

        let mut model_bytes: Option<Vec<u8>> = None;
        let mut model_name: Option<String> = None;
        let mut first_model_candidate: Option<Vec<u8>> = None;
        for i in 0..archive.len() {
            if let Ok(mut file) = archive.by_index(i) {
                let name = file.name().to_string();
                let lname = name.to_lowercase();
                if lname.ends_with(".model") {
                    let mut v = Vec::new();
                    std::io::copy(&mut file, &mut v).map_err(|e| format!("read model: {}", e))?;
                    if v.windows(5).any(|w| w == b"<mesh") || v.windows(9).any(|w| w == b"<vertices") || lname.contains("/objects/") || lname.contains("object_") {
                        model_name = Some(name.clone());
                        model_bytes = Some(v);
                        break;
                    }
                    if first_model_candidate.is_none() {
                        first_model_candidate = Some(v);
                    }
                }
            }
        }
        if model_bytes.is_none() {
            model_bytes = first_model_candidate;
        }
        if model_name.is_none() && model_bytes.is_some() {
            model_name = Some("(first .model)".to_string());
        }

        let model_bytes = match model_bytes {
            Some(b) => {
                if model_name.is_none() {
                    eprintln!("3mf: selected model (unknown), {} bytes", b.len());
                }
                b
            }
            None => return Err("3mf: no .model file found in archive".into()),
        };

        let mut reader = quick_xml::Reader::from_reader(model_bytes.as_slice());
        reader.trim_text(true);

        let mut positions: Vec<[f32; 3]> = Vec::new();
        let mut indices: Vec<[u32; 3]> = Vec::new();

        let mut _dbg_count = 0usize;
        loop {
            use quick_xml::events::Event;
            match reader.read_event() {
                Ok(Event::Start(e)) | Ok(Event::Empty(e)) => {
                    let name = e.name().as_ref().to_vec();
                    if _dbg_count < 20 {
                        _dbg_count += 1;
                    }
                    if name.ends_with(b"vertex") {
                        let mut x = None::<f32>;
                        let mut y = None::<f32>;
                        let mut z = None::<f32>;
                        for attr in e.attributes().with_checks(false) {
                            if let Ok(a) = attr {
                                let key_bytes = a.key.as_ref();
                                if let Ok(val) = a.unescape_value() {
                                    let s = val.as_ref();
                                    if key_bytes.ends_with(b"x") {
                                        x = s.parse::<f32>().ok();
                                    } else if key_bytes.ends_with(b"y") {
                                        y = s.parse::<f32>().ok();
                                    } else if key_bytes.ends_with(b"z") {
                                        z = s.parse::<f32>().ok();
                                    }
                                }
                            }
                        }
                        if let (Some(x), Some(y), Some(z)) = (x, y, z) {
                            positions.push([x, y, z]);
                        }
                    } else if name.ends_with(b"triangle") {
                        let mut v1 = None::<u32>;
                        let mut v2 = None::<u32>;
                        let mut v3 = None::<u32>;
                        for attr in e.attributes().with_checks(false) {
                            if let Ok(a) = attr {
                                let key_bytes = a.key.as_ref();
                                if let Ok(val) = a.unescape_value() {
                                    let s = val.as_ref();
                                    if key_bytes.ends_with(b"v1") {
                                        v1 = s.parse::<u32>().ok();
                                    } else if key_bytes.ends_with(b"v2") {
                                        v2 = s.parse::<u32>().ok();
                                    } else if key_bytes.ends_with(b"v3") {
                                        v3 = s.parse::<u32>().ok();
                                    }
                                }
                            }
                        }
                        if let (Some(a), Some(b), Some(c)) = (v1, v2, v3) {
                            indices.push([a, b, c]);
                        }
                    }
                }
                Ok(Event::Eof) => break,
                Err(e) => return Err(format!("3mf xml parse error: {}", e)),
                _ => {}
            }
        }

        if positions.is_empty() || indices.is_empty() {
            return Err("3mf: no geometry (vertices/triangles) found".into());
        }

        Ok(MeshData { positions, normals: Vec::new(), indices })
    }
}
