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

/// Robustly resolves a relative file path (like textures or MTL files) against base_dir,
/// handling Windows backslashes, leading './', file-name only fallbacks, common subfolders,
/// and case-insensitive matching on Linux.
pub fn resolve_relative_file(base_dir: &Path, raw_path_str: &str) -> Option<std::path::PathBuf> {
    let clean = raw_path_str.trim().trim_matches('"').trim_matches('\'');
    if clean.is_empty() {
        return None;
    }
    let normalized = clean.replace('\\', "/");
    let norm_path = Path::new(&normalized);

    // 1. Direct path joined to base_dir
    let direct = base_dir.join(norm_path);
    if direct.exists() {
        return Some(direct);
    }

    // 2. Fallbacks based on file_name
    if let Some(file_name) = norm_path.file_name() {
        let in_base = base_dir.join(file_name);
        if in_base.exists() {
            return Some(in_base);
        }

        // Subfolders
        for sub in &["textures", "Textures", "images", "Images", "tex", "Tex", "materials", "Materials"] {
            let in_sub = base_dir.join(sub).join(file_name);
            if in_sub.exists() {
                return Some(in_sub);
            }
        }

        // Case-insensitive in base_dir
        if let Ok(entries) = std::fs::read_dir(base_dir) {
            let target_lower = file_name.to_string_lossy().to_lowercase();
            for entry in entries.flatten() {
                if let Ok(file_type) = entry.file_type() {
                    if file_type.is_file() {
                        let name = entry.file_name();
                        if name.to_string_lossy().to_lowercase() == target_lower {
                            return Some(entry.path());
                        }
                    }
                }
            }
        }

        // Case-insensitive in subfolders
        for sub in &["textures", "Textures", "images", "Images"] {
            let sub_path = base_dir.join(sub);
            if let Ok(entries) = std::fs::read_dir(&sub_path) {
                let target_lower = file_name.to_string_lossy().to_lowercase();
                for entry in entries.flatten() {
                    if let Ok(file_type) = entry.file_type() {
                        if file_type.is_file() {
                            let name = entry.file_name();
                            if name.to_string_lossy().to_lowercase() == target_lower {
                                return Some(entry.path());
                            }
                        }
                    }
                }
            }
        }
    }

    None
}

pub fn load_texture_from_path(path: &Path) -> Option<Arc<TextureData>> {
    let img = image::open(path).ok()?;
    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();
    Some(Arc::new(TextureData {
        width,
        height,
        rgba: rgba.into_raw(),
    }))
}

pub fn load_texture_from_memory(bytes: &[u8]) -> Option<Arc<TextureData>> {
    let img = image::load_from_memory(bytes).ok()?;
    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();
    Some(Arc::new(TextureData {
        width,
        height,
        rgba: rgba.into_raw(),
    }))
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
                let min_x = mesh.positions.iter().map(|p| p[0]).fold(f32::INFINITY, f32::min);
                let max_x = mesh.positions.iter().map(|p| p[0]).fold(f32::NEG_INFINITY, f32::max);
                let min_y = mesh.positions.iter().map(|p| p[1]).fold(f32::INFINITY, f32::min);
                let max_y = mesh.positions.iter().map(|p| p[1]).fold(f32::NEG_INFINITY, f32::max);
                let min_z = mesh.positions.iter().map(|p| p[2]).fold(f32::INFINITY, f32::min);
                let max_z = mesh.positions.iter().map(|p| p[2]).fold(f32::NEG_INFINITY, f32::max);
                println!("  X: [{}, {}], Y: [{}, {}], Z: [{}, {}]", min_x, max_x, min_y, max_y, min_z, max_z);
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
                    // 552 triangles is Eye! Must be off-white (linear of #E7D6D6 is ~0.796)
                    assert!(col[0] > 0.75 && col[1] > 0.65 && col[2] > 0.65, "Eye should be off-white #E7D6D6, got {:?}", col);
                    has_bright_eye = true;
                }
            }
        }

        assert!(has_dark_pupil, "Must have dark pupil submesh");
        assert!(has_bright_eye, "Must have bright eye submesh");
    }

    #[test]
    fn test_resolve_relative_file() {
        let temp_dir = std::env::temp_dir().join("mesh_test_rel_resolve");
        let textures_dir = temp_dir.join("textures");
        let _ = std::fs::create_dir_all(&textures_dir);

        let img_path = textures_dir.join("albedo.png");
        // Create 2x2 dummy image
        let img = image::RgbaImage::new(2, 2);
        let _ = img.save(&img_path);

        // Test 1: Relative with windows backslash
        let r1 = resolve_relative_file(&temp_dir, "textures\\albedo.png");
        assert!(r1.is_some());
        assert_eq!(r1.unwrap().canonicalize().ok(), img_path.canonicalize().ok());

        // Test 2: Just filename (should find inside textures/)
        let r2 = resolve_relative_file(&temp_dir, "albedo.png");
        assert!(r2.is_some());

        // Test 3: Path with ./ prefix
        let r3 = resolve_relative_file(&temp_dir, "./textures/albedo.png");
        assert!(r3.is_some());

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_obj_relative_texture_loading() {
        let temp_dir = std::env::temp_dir().join("mesh_test_obj_tex");
        let textures_dir = temp_dir.join("textures");
        let _ = std::fs::create_dir_all(&textures_dir);

        let img_path = textures_dir.join("diffuse.png");
        let img = image::RgbaImage::from_fn(4, 4, |x, y| {
            image::Rgba([(x * 50) as u8, (y * 50) as u8, 128, 255])
        });
        img.save(&img_path).expect("save test png");

        let mtl_path = temp_dir.join("test.mtl");
        // Using Windows backslash and flags like -s and -bm
        let mtl_content = "newmtl BoxMat\nKd 0.9 0.9 0.9\nmap_Kd -s 1 1 1 -bm 1.0 textures\\diffuse.png\n";
        std::fs::write(&mtl_path, mtl_content).expect("write mtl");

        let obj_path = temp_dir.join("test.obj");
        let obj_content = "mtllib test.mtl\nv 0.0 0.0 0.0\nv 1.0 0.0 0.0\nv 0.0 1.0 0.0\nvt 0.0 0.0\nvt 1.0 0.0\nvt 0.0 1.0\nusemtl BoxMat\nf 1/1 2/2 3/3\n";
        std::fs::write(&obj_path, obj_content).expect("write obj");

        let registry = LoaderRegistry::default_registry();
        let mesh = registry.load_path(&obj_path).expect("load obj");
        assert_eq!(mesh.submeshes.len(), 1);
        let sub = &mesh.submeshes[0];
        assert!(sub.texture.is_some(), "SubMesh should have loaded relative texture from MTL");
        let tex = sub.texture.as_ref().unwrap();
        assert_eq!(tex.width, 4);
        assert_eq!(tex.height, 4);
        assert_eq!(tex.rgba.len(), 4 * 4 * 4);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_gltf_external_reference() {
        let temp_dir = std::env::temp_dir().join("mesh_test_gltf_ext");
        let _ = std::fs::create_dir_all(&temp_dir);

        let bin_path = temp_dir.join("test.bin");
        // 3 vertices [0,0,0], [1,0,0], [0,1,0]
        let mut bin_data = Vec::new();
        for &f in &[0.0f32, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0] {
            bin_data.extend_from_slice(&f.to_le_bytes());
        }
        std::fs::write(&bin_path, &bin_data).expect("write bin");

        let gltf_path = temp_dir.join("test.gltf");
        let gltf_json = r#"{
            "asset": { "version": "2.0" },
            "buffers": [ { "uri": "test.bin", "byteLength": 36 } ],
            "bufferViews": [ { "buffer": 0, "byteOffset": 0, "byteLength": 36 } ],
            "accessors": [ {
                "bufferView": 0,
                "byteOffset": 0,
                "componentType": 5126,
                "count": 3,
                "type": "VEC3",
                "max": [1.0, 1.0, 0.0],
                "min": [0.0, 0.0, 0.0]
            } ],
            "meshes": [ {
                "primitives": [ {
                    "attributes": { "POSITION": 0 }
                } ]
            } ],
            "nodes": [ { "mesh": 0 } ],
            "scenes": [ { "nodes": [0] } ],
            "scene": 0
        }"#;
        std::fs::write(&gltf_path, gltf_json).expect("write gltf");

        let registry = LoaderRegistry::default_registry();
        let mesh = registry.load_path(&gltf_path).expect("load gltf with external .bin");
        assert_eq!(mesh.positions.len(), 3);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}

