use std::path::{Path, PathBuf};
use std::sync::Arc;
use crate::loader::types::TextureData;

/// Robustly resolves a relative file path (like textures or MTL files) against base_dir,
/// handling Windows backslashes, leading './', file-name only fallbacks, common subfolders,
/// and case-insensitive matching on Linux.
pub fn resolve_relative_file(base_dir: &Path, raw_path_str: &str) -> Option<PathBuf> {
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
