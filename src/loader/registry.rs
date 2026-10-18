use std::path::Path;
use std::io::Read;
use crate::loader::types::MeshData;
use crate::loader::animation::AnimationController;
use crate::loader::formats::*;

/// Description and metadata for a supported 3D file format.
/// Serves as the Single Source of Truth (SSoT) for the entire application.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileFormatInfo {
    /// File extension without leading dot (e.g., "stl", "obj", "dae", "glb")
    pub extension: &'static str,
    /// User-facing display label for UI and file dialog filters (e.g., "Stereolithography (*.stl)")
    pub label: &'static str,
    /// Default filename suggested when exporting (e.g., "model.stl")
    pub default_filename: &'static str,
    /// Whether the format plugin supports loading this format
    pub can_load: bool,
    /// Whether the format plugin supports exporting to this format
    pub can_export: bool,
}

/// Trait defining a file format plugin (codec) for loading, exporting, and inspecting format capabilities.
pub trait FormatPlugin: Send + Sync {
    fn name(&self) -> &str;

    /// Formats and extensions supported by this plugin.
    fn formats(&self) -> Vec<FileFormatInfo>;

    fn can_load(&self, path: &Path, header: &[u8]) -> bool;
    fn load(&self, path: &Path, reader: Box<dyn Read>) -> Result<MeshData, String>;

    fn can_export(&self, path: &Path) -> bool {
        let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
        self.formats().iter().any(|f| f.can_export && f.extension.eq_ignore_ascii_case(ext))
    }

    fn export(&self, path: &Path, mesh: &MeshData) -> Result<(), String> {
        let _ = (path, mesh);
        Err(format!("Exporting is not supported by '{}' format", self.name()))
    }

    fn supports_animation(&self) -> bool {
        false
    }
}

/// Backward compatibility alias: `Loader` is synonymous with `FormatPlugin`.
pub use FormatPlugin as Loader;

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

    pub fn default_registry() -> Self {
        let mut registry = Self::new();
        registry.register(Box::new(StlLoader {}));
        registry.register(Box::new(ThreemfLoader {}));
        registry.register(Box::new(ObjLoader {}));
        registry.register(Box::new(GltfLoader {}));
        registry.register(Box::new(FbxLoader {}));
        registry.register(Box::new(ColladaLoader {}));
        registry
    }

    /// Returns all format infos declared by all registered loaders.
    pub fn all_formats(&self) -> Vec<FileFormatInfo> {
        let mut formats = Vec::new();
        for l in &self.loaders {
            formats.extend(l.formats());
        }
        formats
    }

    /// Returns list of all loadable file extensions (e.g. ["stl", "3mf", "obj", "gltf", "glb", "fbx", "dae"]).
    pub fn loadable_extensions(&self) -> Vec<&'static str> {
        let mut exts = Vec::new();
        for fmt in self.all_formats() {
            if fmt.can_load && !exts.contains(&fmt.extension) {
                exts.push(fmt.extension);
            }
        }
        exts
    }

    /// Builds a dynamic file dialog filter title, e.g. "3D Models (*.stl, *.3mf, *.obj, *.gltf, *.glb, *.fbx, *.dae)".
    pub fn load_filter_description(&self) -> String {
        let patterns: Vec<String> = self.loadable_extensions()
            .iter()
            .map(|ext| format!("*.{}", ext))
            .collect();
        format!("3D Models ({})", patterns.join(", "))
    }

    /// Returns all formats that support exporting.
    pub fn exportable_formats(&self) -> Vec<FileFormatInfo> {
        self.all_formats()
            .into_iter()
            .filter(|f| f.can_export)
            .collect()
    }

    /// Finds format info matching the given format extension or identifier.
    pub fn find_export_format(&self, ext_or_id: &str) -> Option<FileFormatInfo> {
        let target = ext_or_id.trim().trim_start_matches('.').to_ascii_lowercase();
        self.exportable_formats()
            .into_iter()
            .find(|f| f.extension.eq_ignore_ascii_case(&target))
    }

    pub fn is_supported_extension(&self, ext: &str) -> bool {
        let target = ext.trim().trim_start_matches('.').to_ascii_lowercase();
        self.loadable_extensions()
            .iter()
            .any(|e| e.eq_ignore_ascii_case(&target))
    }

    #[allow(dead_code)]
    pub fn is_supported_file(&self, path: &Path) -> bool {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");
        self.is_supported_extension(ext)
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
    LoaderRegistry::default_registry().is_supported_extension(&ext)
}
