use std::path::Path;
use std::io::Read;

pub mod stl;
pub mod threemf;
pub mod obj;
pub mod gltf;

pub struct MeshData {
    pub positions: Vec<[f32; 3]>,
    #[allow(dead_code)]
    pub normals: Vec<[f32; 3]>,
    pub indices: Vec<[u32; 3]>,
}

pub trait Loader: Send + Sync {
    #[allow(dead_code)]
    fn name(&self) -> &str;
    fn can_load(&self, path: &Path, header: &[u8]) -> bool;
    fn load(&self, path: &Path, reader: Box<dyn Read>) -> Result<MeshData, String>;
}

pub struct LoaderRegistry {
    loaders: Vec<Box<dyn Loader>>,
}

impl LoaderRegistry {
    pub fn new() -> Self {
        Self { loaders: Vec::new() }
    }

    pub fn register(&mut self, l: Box<dyn Loader>) {
        self.loaders.push(l);
    }

    pub fn load_path(&self, path: &Path) -> Result<MeshData, String> {
        let mut f = std::fs::File::open(path).map_err(|e| format!("open failed: {}", e))?;
        let mut header = [0u8; 64];
        let n = f.read(&mut header).map_err(|e| format!("read failed: {}", e))?;
        let header = &header[..n];
        for l in &self.loaders {
            if l.can_load(path, header) {
                let rf = std::fs::File::open(path).map_err(|e| format!("open failed: {}", e))?;
                return l.load(path, Box::new(rf));
            }
        }
        Err("no suitable loader".into())
    }
}
