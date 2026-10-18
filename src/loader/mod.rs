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
    pub source_path: Option<std::path::PathBuf>,
    pub has_animation: bool,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct TriangleVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    #[allow(dead_code)]
    pub uv: [f32; 2],
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct ExportTriangle {
    pub vertices: [TriangleVertex; 3],
    pub normal: [f32; 3],
    #[allow(dead_code)]
    pub color: Option<[f32; 4]>,
}

#[allow(dead_code)]
pub fn compute_face_normal(v0: [f32; 3], v1: [f32; 3], v2: [f32; 3]) -> [f32; 3] {
    let e1 = glam::Vec3::from(v1) - glam::Vec3::from(v0);
    let e2 = glam::Vec3::from(v2) - glam::Vec3::from(v0);
    let c = e1.cross(e2);
    if c.length_squared() > 1e-20 {
        c.normalize().into()
    } else {
        [0.0, 0.0, 1.0]
    }
}

pub fn compute_normals(positions: &[[f32; 3]], indices: &[[u32; 3]]) -> Vec<[f32; 3]> {
    let mut normals = vec![[0.0f32; 3]; positions.len()];
    for tri in indices {
        let i0 = tri[0] as usize;
        let i1 = tri[1] as usize;
        let i2 = tri[2] as usize;
        if i0 < positions.len() && i1 < positions.len() && i2 < positions.len() {
            let p0 = glam::Vec3::from(positions[i0]);
            let p1 = glam::Vec3::from(positions[i1]);
            let p2 = glam::Vec3::from(positions[i2]);

            let v01 = p1 - p0;
            let v02 = p2 - p0;
            let v12 = p2 - p1;

            let fnorm = v01.cross(v02);
            let fn_len = fnorm.length();
            if fn_len < 1e-6 {
                continue;
            }
            let fn_norm = fnorm / fn_len;

            let l01 = v01.length();
            let l02 = v02.length();
            let l12 = v12.length();

            if l01 > 1e-6 && l02 > 1e-6 && l12 > 1e-6 {
                let a0 = (v01.dot(v02) / (l01 * l02)).clamp(-1.0, 1.0).acos();
                let a1 = ((-v01.dot(v12)) / (l01 * l12)).clamp(-1.0, 1.0).acos();
                let a2 = ((-v02.dot(-v12)) / (l02 * l12)).clamp(-1.0, 1.0).acos();

                let n0 = glam::Vec3::from(normals[i0]) + fn_norm * a0;
                let n1 = glam::Vec3::from(normals[i1]) + fn_norm * a1;
                let n2 = glam::Vec3::from(normals[i2]) + fn_norm * a2;

                normals[i0] = n0.into();
                normals[i1] = n1.into();
                normals[i2] = n2.into();
            }
        }
    }
    for n in &mut normals {
        let v = glam::Vec3::from(*n);
        *n = if v.length_squared() > 1e-6 {
            v.normalize().into()
        } else {
            [0.0, 0.0, 1.0]
        };
    }
    normals
}

impl MeshData {
    pub fn collect_triangles(&self) -> Vec<ExportTriangle> {
        let mut triangles = Vec::new();
        if !self.submeshes.is_empty() {
            for sub in &self.submeshes {
                for tri in &sub.indices {
                    let i0 = tri[0] as usize;
                    let i1 = tri[1] as usize;
                    let i2 = tri[2] as usize;
                    if i0 < sub.positions.len() && i1 < sub.positions.len() && i2 < sub.positions.len() {
                        let p0 = sub.positions[i0];
                        let mut p1 = sub.positions[i1];
                        let mut p2 = sub.positions[i2];

                        let n0 = if i0 < sub.normals.len() { sub.normals[i0] } else { [0.0, 0.0, 1.0] };
                        let mut n1 = if i1 < sub.normals.len() { sub.normals[i1] } else { n0 };
                        let mut n2 = if i2 < sub.normals.len() { sub.normals[i2] } else { n0 };

                        let uv0 = if i0 < sub.uvs.len() { sub.uvs[i0] } else { [0.0, 0.0] };
                        let mut uv1 = if i1 < sub.uvs.len() { sub.uvs[i1] } else { [0.0, 0.0] };
                        let mut uv2 = if i2 < sub.uvs.len() { sub.uvs[i2] } else { [0.0, 0.0] };

                        let e1 = glam::Vec3::from(p1) - glam::Vec3::from(p0);
                        let e2 = glam::Vec3::from(p2) - glam::Vec3::from(p0);
                        let c = e1.cross(e2);
                        let avg_n = (glam::Vec3::from(n0) + glam::Vec3::from(n1) + glam::Vec3::from(n2)) / 3.0;

                        let mut fnorm = if c.length_squared() > 1e-20 {
                            c.normalize()
                        } else if avg_n.length_squared() > 1e-20 {
                            avg_n.normalize()
                        } else {
                            glam::Vec3::Z
                        };

                        // If geometric normal points opposite to vertex normals, flip normal and swap vertices
                        if avg_n.length_squared() > 1e-20 && fnorm.dot(avg_n) < 0.0 {
                            fnorm = -fnorm;
                            std::mem::swap(&mut p1, &mut p2);
                            std::mem::swap(&mut n1, &mut n2);
                            std::mem::swap(&mut uv1, &mut uv2);
                        }

                        triangles.push(ExportTriangle {
                            vertices: [
                                TriangleVertex { position: p0, normal: n0, uv: uv0 },
                                TriangleVertex { position: p1, normal: n1, uv: uv1 },
                                TriangleVertex { position: p2, normal: n2, uv: uv2 },
                            ],
                            normal: fnorm.into(),
                            color: sub.color,
                        });
                    }
                }
            }
        } else {
            for tri in &self.indices {
                let i0 = tri[0] as usize;
                let i1 = tri[1] as usize;
                let i2 = tri[2] as usize;
                if i0 < self.positions.len() && i1 < self.positions.len() && i2 < self.positions.len() {
                    let p0 = self.positions[i0];
                    let mut p1 = self.positions[i1];
                    let mut p2 = self.positions[i2];

                    let n0 = if i0 < self.normals.len() { self.normals[i0] } else { [0.0, 0.0, 1.0] };
                    let mut n1 = if i1 < self.normals.len() { self.normals[i1] } else { n0 };
                    let mut n2 = if i2 < self.normals.len() { self.normals[i2] } else { n0 };

                    let uv0 = if i0 < self.uvs.len() { self.uvs[i0] } else { [0.0, 0.0] };
                    let mut uv1 = if i1 < self.uvs.len() { self.uvs[i1] } else { [0.0, 0.0] };
                    let mut uv2 = if i2 < self.uvs.len() { self.uvs[i2] } else { [0.0, 0.0] };

                    let e1 = glam::Vec3::from(p1) - glam::Vec3::from(p0);
                    let e2 = glam::Vec3::from(p2) - glam::Vec3::from(p0);
                    let c = e1.cross(e2);
                    let avg_n = (glam::Vec3::from(n0) + glam::Vec3::from(n1) + glam::Vec3::from(n2)) / 3.0;

                    let mut fnorm = if c.length_squared() > 1e-20 {
                        c.normalize()
                    } else if avg_n.length_squared() > 1e-20 {
                        avg_n.normalize()
                    } else {
                        glam::Vec3::Z
                    };

                    if avg_n.length_squared() > 1e-20 && fnorm.dot(avg_n) < 0.0 {
                        fnorm = -fnorm;
                        std::mem::swap(&mut p1, &mut p2);
                        std::mem::swap(&mut n1, &mut n2);
                        std::mem::swap(&mut uv1, &mut uv2);
                    }

                    triangles.push(ExportTriangle {
                        vertices: [
                            TriangleVertex { position: p0, normal: n0, uv: uv0 },
                            TriangleVertex { position: p1, normal: n1, uv: uv1 },
                            TriangleVertex { position: p2, normal: n2, uv: uv2 },
                        ],
                        normal: fnorm.into(),
                        color: None,
                    });
                }
            }
        }
        triangles
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PlaybackState {
    #[default]
    Stopped,
    Playing,
    Paused,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Default)]
pub struct AnimationTrack {
    pub name: String,
    pub duration: f32,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Default)]
pub struct AnimationData {
    pub tracks: Vec<AnimationTrack>,
}

/// Runtime controller that manages playback state (playing, paused, stopped) and timeline.
#[derive(Debug, Clone, Default)]
pub struct AnimationController {
    state: PlaybackState,
    #[allow(dead_code)]
    current_time: f32,
    #[allow(dead_code)]
    speed: f32,
    has_animation: bool,
}

impl AnimationController {
    pub fn new(has_animation: bool) -> Self {
        Self {
            state: PlaybackState::Stopped,
            current_time: 0.0,
            speed: 1.0,
            has_animation,
        }
    }

    #[allow(dead_code)]
    pub fn has_animation(&self) -> bool {
        self.has_animation
    }

    #[allow(dead_code)]
    pub fn set_has_animation(&mut self, has_animation: bool) {
        self.has_animation = has_animation;
        if !has_animation {
            self.state = PlaybackState::Stopped;
            self.current_time = 0.0;
        }
    }

    #[allow(dead_code)]
    pub fn state(&self) -> PlaybackState {
        self.state
    }

    #[allow(dead_code)]
    pub fn current_time(&self) -> f32 {
        self.current_time
    }

    #[allow(dead_code)]
    pub fn speed(&self) -> f32 {
        self.speed
    }

    #[allow(dead_code)]
    pub fn set_speed(&mut self, speed: f32) {
        self.speed = speed;
    }

    pub fn play(&mut self) {
        if !self.has_animation {
            println!("[AnimationController] Cannot play: model does not have animations.");
            return;
        }
        self.state = PlaybackState::Playing;
        println!("[AnimationController] Play animation (dummy print)");
    }

    pub fn pause(&mut self) {
        if self.state == PlaybackState::Playing {
            self.state = PlaybackState::Paused;
            println!("[AnimationController] Pause animation (dummy print)");
        } else {
            println!("[AnimationController] Pause animation called while state is {:?}", self.state);
        }
    }

    pub fn stop(&mut self) {
        self.state = PlaybackState::Stopped;
        self.current_time = 0.0;
        println!("[AnimationController] Stop animation (dummy print)");
    }
}

/// Trait defining a file format plugin (codec) for loading, exporting, and inspecting format capabilities.
pub trait FormatPlugin: Send + Sync {
    #[allow(dead_code)]
    fn name(&self) -> &str;
    fn can_load(&self, path: &Path, header: &[u8]) -> bool;
    fn load(&self, path: &Path, reader: Box<dyn Read>) -> Result<MeshData, String>;

    #[allow(dead_code)]
    fn can_export(&self, path: &Path) -> bool {
        let _ = path;
        false
    }

    #[allow(dead_code)]
    fn export(&self, path: &Path, mesh: &MeshData) -> Result<(), String> {
        let _ = (path, mesh);
        Err(format!("Exporting is not supported by '{}' format", self.name()))
    }

    #[allow(dead_code)]
    fn supports_animation(&self) -> bool {
        false
    }
}

/// Backward compatibility alias: `Loader` is synonymous with `FormatPlugin`.
pub use FormatPlugin as Loader;

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
                let mut mesh = l.load(path, Box::new(rf))?;
                mesh.source_path = Some(path.to_path_buf());
                mesh.has_animation = l.supports_animation();
                return Ok(mesh);
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

    #[allow(dead_code)]
    pub fn find_loader_for_path(&self, path: &Path) -> Option<&dyn FormatPlugin> {
        for l in &self.loaders {
            if l.can_export(path) {
                return Some(l.as_ref());
            }
        }
        if let Ok(mut f) = std::fs::File::open(path) {
            let mut header = [0u8; 64];
            if let Ok(n) = f.read(&mut header) {
                let header = &header[..n];
                for l in &self.loaders {
                    if l.can_load(path, header) {
                        return Some(l.as_ref());
                    }
                }
            }
        }
        None
    }

    #[allow(dead_code)]
    pub fn can_export(&self, path: &Path) -> bool {
        self.loaders.iter().any(|l| l.can_export(path))
    }

    #[allow(dead_code)]
    pub fn export_path(&self, path: &Path, mesh: &MeshData) -> Result<(), String> {
        if let Some(loader) = self.find_loader_for_path(path) {
            loader.export(path, mesh)
        } else {
            Err(format!("No loader found capable of exporting '{}'", path.display()))
        }
    }

    #[allow(dead_code)]
    pub fn supports_animation(&self, path: &Path) -> bool {
        self.find_loader_for_path(path)
            .map(|l| l.supports_animation())
            .unwrap_or(false)
    }

    #[allow(dead_code)]
    pub fn has_animation(&self, path: &Path) -> bool {
        self.supports_animation(path)
    }

    #[allow(dead_code)]
    pub fn create_animation_controller(&self, path: &Path) -> AnimationController {
        let has_anim = self.supports_animation(path);
        AnimationController::new(has_anim)
    }

    #[allow(dead_code)]
    pub fn create_animation_controller_for_mesh(&self, mesh: &MeshData) -> AnimationController {
        AnimationController::new(mesh.has_animation)
    }

    #[allow(dead_code)]
    pub fn play_animation(&self, path: &Path) {
        let mut ctrl = self.create_animation_controller(path);
        ctrl.play();
    }

    #[allow(dead_code)]
    pub fn pause_animation(&self, path: &Path) {
        let mut ctrl = self.create_animation_controller(path);
        ctrl.pause();
    }

    #[allow(dead_code)]
    pub fn stop_animation(&self, path: &Path) {
        let mut ctrl = self.create_animation_controller(path);
        ctrl.stop();
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

    #[test]
    fn test_loader_export_and_animation_capabilities() {
        let registry = LoaderRegistry::default_registry();
        let test_mesh = MeshData {
            positions: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            normals: vec![[0.0, 0.0, 1.0], [0.0, 0.0, 1.0], [0.0, 0.0, 1.0]],
            uvs: vec![[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]],
            indices: vec![[0, 1, 2]],
            ..Default::default()
        };

        let temp_dir = std::env::temp_dir().join("mesh_export_test");
        let _ = std::fs::create_dir_all(&temp_dir);

        let extensions = [
            ("model.stl", false),
            ("model.obj", false),
            ("model.3mf", false),
            ("model.gltf", true),
            ("model.glb", true),
            ("model.fbx", true),
        ];

        for (filename, expected_animation) in extensions {
            let path = temp_dir.join(filename);
            assert!(registry.can_export(&path), "Should support export for {}", filename);
            assert_eq!(
                registry.export_path(&path, &test_mesh),
                Ok(()),
                "Export should succeed for {}",
                filename
            );
            assert!(path.exists(), "Exported file should exist: {}", path.display());
            assert_eq!(
                registry.has_animation(&path),
                expected_animation,
                "Animation flag mismatch for {}",
                filename
            );

            // Test loading the exported file back
            let loaded = registry.load_path(&path);
            assert!(loaded.is_ok(), "Failed to load back exported file {}: {:?}", filename, loaded.err());
            let loaded_mesh = loaded.unwrap();
            assert!(!loaded_mesh.positions.is_empty(), "Loaded mesh should have positions for {}", filename);

            // Calling animation controls should not panic
            registry.play_animation(&path);
            registry.pause_animation(&path);
            registry.stop_animation(&path);
        }

        let unsupported = PathBuf::from("model.xyz");
        assert!(!registry.can_export(&unsupported));
        assert!(registry.export_path(&unsupported, &test_mesh).is_err());
        assert!(!registry.has_animation(&unsupported));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_animation_controller_lifecycle() {
        let mut controller = AnimationController::new(true);
        assert_eq!(controller.state(), PlaybackState::Stopped);
        assert!(controller.has_animation());

        controller.play();
        assert_eq!(controller.state(), PlaybackState::Playing);

        controller.pause();
        assert_eq!(controller.state(), PlaybackState::Paused);

        controller.stop();
        assert_eq!(controller.state(), PlaybackState::Stopped);

        let mut no_anim_ctrl = AnimationController::new(false);
        no_anim_ctrl.play(); // Refuses to play without animations
        assert_eq!(no_anim_ctrl.state(), PlaybackState::Stopped);
    }

    #[test]
    fn test_suzanne_export_materials_and_orientation() {
        let registry = LoaderRegistry::default_registry();
        let suzanne_path = PathBuf::from("assets/models/Suzanne.glb");
        if !suzanne_path.exists() {
            return;
        }

        let orig = registry.load_path(&suzanne_path).expect("load Suzanne.glb");
        assert!(!orig.submeshes.is_empty(), "original Suzanne should have submeshes");
        assert!(orig.submeshes.iter().any(|s| s.color.is_some()), "original Suzanne should have material colors");

        let temp_dir = std::env::temp_dir().join("mesh_suzanne_export_test");
        let _ = std::fs::create_dir_all(&temp_dir);

        let targets = [
            ("suzanne_exp.glb", true),
            ("suzanne_exp.gltf", true),
            ("suzanne_exp.3mf", true),
            ("suzanne_exp.fbx", true),
            ("suzanne_exp.obj", true),
            ("suzanne_exp.stl", false),
        ];

        for (filename, expect_materials) in targets {
            let out_path = temp_dir.join(filename);
            registry.export_path(&out_path, &orig).expect(&format!("export to {}", filename));
            assert!(out_path.exists(), "exported file {} must exist", filename);

            let reloaded = registry.load_path(&out_path).expect(&format!("reload {}", filename));
            assert!(!reloaded.positions.is_empty(), "reloaded {} should have positions", filename);
            assert!(!reloaded.submeshes.is_empty(), "reloaded {} should have submeshes", filename);

            if expect_materials {
                let has_material_colors = reloaded.submeshes.iter().any(|s| s.color.is_some());
                assert!(has_material_colors, "reloaded {} must preserve material colors", filename);
            }

            if filename.ends_with(".3mf") {
                // Verify that coincident vertices on UV seams are welded so the mesh is not cracked:
                // Raw un-welded glTF vertices were 8169; welded vertices should be <= 7700.
                assert!(
                    reloaded.positions.len() <= 7700,
                    "3MF should weld UV seam duplicate vertices: got {} vertices (expected <= 7700)",
                    reloaded.positions.len()
                );

                // Verify 3MF archive structure for Blender ThreeMF_io and OrcaSlicer compatibility
                let file = std::fs::File::open(&out_path).unwrap();
                let mut zip = zip::ZipArchive::new(file).unwrap();
                use std::io::Read;

                let mut ct_str = String::new();
                zip.by_name("[Content_Types].xml").unwrap().read_to_string(&mut ct_str).unwrap();
                assert!(ct_str.contains("application/vnd.ms-package.3dmanufacturing-3dmodel+xml"), "3MF must have valid model MIME type with '+'");

                let mut proj_str = String::new();
                zip.by_name("Metadata/project_settings.config").unwrap().read_to_string(&mut proj_str).unwrap();
                assert!(proj_str.contains("filament_colour"), "3MF must include filament_colour for OrcaSlicer");

                let mut model_str = String::new();
                zip.by_name("3D/3dmodel.model").unwrap().read_to_string(&mut model_str).unwrap();
                assert!(model_str.contains("paint_color="), "3MF must have paint_color attributes on triangles for OrcaSlicer");
                assert!(model_str.contains("xmlns:m="), "3MF must declare materials extension namespace");
            }

            if filename.ends_with(".fbx") {
                let fbx_content = std::fs::read_to_string(&out_path).unwrap();
                assert!(
                    fbx_content.contains("\"UnitScaleFactor\", \"double\", \"Number\", \"\", 100.0"),
                    "FBX must set UnitScaleFactor to 100.0 for meter scale in Blender"
                );
            }

            if filename.ends_with(".glb") {
                // Verify orientation consistency: bounding box max Z and Y must be close to original
                let orig_min_z = orig.positions.iter().map(|p| p[2]).fold(f32::INFINITY, f32::min);
                let orig_max_z = orig.positions.iter().map(|p| p[2]).fold(f32::NEG_INFINITY, f32::max);
                let rel_min_z = reloaded.positions.iter().map(|p| p[2]).fold(f32::INFINITY, f32::min);
                let rel_max_z = reloaded.positions.iter().map(|p| p[2]).fold(f32::NEG_INFINITY, f32::max);

                assert!((orig_min_z - rel_min_z).abs() < 0.05, "Z min orientation mismatch: {} vs {}", orig_min_z, rel_min_z);
                assert!((orig_max_z - rel_max_z).abs() < 0.05, "Z max orientation mismatch: {} vs {}", orig_max_z, rel_max_z);
            }
        }

        let _ = std::fs::copy(temp_dir.join("suzanne_exp.3mf"), "/home/odevsa/Suzanne.3mf");
        let _ = std::fs::copy(temp_dir.join("suzanne_exp.fbx"), "/home/odevsa/Suzanne.fbx");
        let _ = std::fs::copy(temp_dir.join("suzanne_exp.obj"), "/home/odevsa/Suzanne.obj");
        let _ = std::fs::copy(temp_dir.join("suzanne_exp.mtl"), "/home/odevsa/Suzanne.mtl");

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}

