use std::path::Path;
use std::io::Read;
use std::sync::Arc;

pub mod stl;
pub mod threemf;
pub mod obj;
pub mod gltf;
pub mod fbx;

#[derive(Clone, Debug, PartialEq)]
pub struct TextureData {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

#[derive(Debug, Clone, Default)]
pub struct SubMesh {
    pub positions: Vec<[f32; 3]>,
    #[allow(dead_code)]
    pub normals: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    pub indices: Vec<[u32; 3]>,
    pub color: Option<[f32; 4]>,
    pub texture: Option<Arc<TextureData>>,
}

#[derive(Debug, Clone, Default)]
pub struct MeshData {
    pub positions: Vec<[f32; 3]>,
    #[allow(dead_code)]
    pub normals: Vec<[f32; 3]>,
    #[allow(dead_code)]
    pub uvs: Vec<[f32; 2]>,
    pub indices: Vec<[u32; 3]>,
    pub submeshes: Vec<SubMesh>,
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

    pub fn default_registry() -> Self {
        let mut registry = Self::new();
        registry.register(Box::new(stl::StlLoader {}));
        registry.register(Box::new(threemf::ThreemfLoader {}));
        registry.register(Box::new(obj::ObjLoader {}));
        registry.register(Box::new(gltf::GltfLoader {}));
        registry.register(Box::new(fbx::FbxLoader {}));
        registry
    }
}

pub fn is_supported_file(path: &Path) -> bool {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    matches!(ext.as_str(), "stl" | "3mf" | "obj" | "gltf" | "glb" | "fbx")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_is_supported_file() {
        assert!(is_supported_file(&PathBuf::from("model.stl")));
        assert!(is_supported_file(&PathBuf::from("model.STL")));
        assert!(is_supported_file(&PathBuf::from("/path/to/mesh.3mf")));
        assert!(is_supported_file(&PathBuf::from("something.obj")));
        assert!(is_supported_file(&PathBuf::from("scene.gltf")));
        assert!(is_supported_file(&PathBuf::from("scene.glb")));
        assert!(is_supported_file(&PathBuf::from("character.fbx")));

        assert!(!is_supported_file(&PathBuf::from("notes.txt")));
        assert!(!is_supported_file(&PathBuf::from("image.png")));
        assert!(!is_supported_file(&PathBuf::from("archive.zip")));
        assert!(!is_supported_file(&PathBuf::from("no_extension")));
    }

    #[test]
    fn test_inspect_all_models() {
        let registry = LoaderRegistry::default_registry();
        let files = [
            "assets/models/Suzanne.stl",
            "assets/models/Suzanne.obj",
            "assets/models/Suzanne.glb",
            "assets/models/Suzanne.3mf",
            "assets/models/Suzanne.fbx",
        ];
        for f in files {
            let path = PathBuf::from(f);
            if path.exists() {
                let mesh = registry.load_path(&path).expect(&format!("Failed to load {}", f));
                println!("--- Model: {} ---", f);
                println!("  positions: {}", mesh.positions.len());
                println!("  normals: {}", mesh.normals.len());
                let has_normals = !mesh.normals.is_empty() && mesh.normals.iter().any(|n| n[0] != 0.0 || n[1] != 0.0 || n[2] != 0.0);
                println!("  has non-zero normals: {}", has_normals);
                assert!(!mesh.positions.is_empty());
                assert!(!mesh.submeshes.is_empty());
                for sub in &mesh.submeshes {
                    assert!(!sub.positions.is_empty());
                    assert!(!sub.indices.is_empty());
                    assert_eq!(sub.normals.len(), sub.positions.len());
                    for tri in &sub.indices {
                        assert!((tri[0] as usize) < sub.positions.len());
                        assert!((tri[1] as usize) < sub.positions.len());
                        assert!((tri[2] as usize) < sub.positions.len());
                    }
                }
            }
        }
    }

    #[test]
    fn test_3mf_suzanne_colors_and_submeshes() {
        let registry = LoaderRegistry::default_registry();
        let path = PathBuf::from("assets/models/Suzanne.3mf");
        if !path.exists() {
            return;
        }
        let mesh = registry.load_path(&path).expect("Failed to load Suzanne.3mf");
        assert_eq!(mesh.submeshes.len(), 4, "Suzanne.3mf should have 4 submeshes (Skin, Ear, Eye, Pupil)");

        // Verify pupil has dark/black color (#282828 roughly ~0.157)
        // and eye has bright/white color (#E7D6D6 roughly ~0.9)
        let mut has_dark_pupil = false;
        let mut has_bright_eye = false;

        for sub in &mesh.submeshes {
            if let Some(col) = sub.color {
                if sub.indices.len() == 896 {
                    // 896 triangles is Pupil! Must be dark
                    assert!(col[0] < 0.25 && col[1] < 0.25 && col[2] < 0.25, "Pupil should be dark #282828, got {:?}", col);
                    has_dark_pupil = true;
                } else if sub.indices.len() == 552 {
                    // 552 triangles is Eye! Must be off-white
                    assert!(col[0] > 0.8 && col[1] > 0.7 && col[2] > 0.7, "Eye should be off-white #E7D6D6, got {:?}", col);
                    has_bright_eye = true;
                }
            }
        }

        assert!(has_dark_pupil, "Must have dark pupil submesh");
        assert!(has_bright_eye, "Must have bright eye submesh");
    }
}

