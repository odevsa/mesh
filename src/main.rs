#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod camera;
mod config;
mod dialog;
mod i18n;
mod loader;
mod scene;
mod ui;

use app::App;
use config::Config;
use loader::LoaderRegistry;
use std::path::PathBuf;
use std::sync::mpsc::channel;
use std::sync::Arc;

fn main() {
    if std::env::var_os("WGPU_BACKEND").is_none() {
        unsafe {
            #[cfg(target_os = "windows")]
            std::env::set_var("WGPU_BACKEND", "dx12");
            #[cfg(target_os = "linux")]
            std::env::set_var("WGPU_BACKEND", "vulkan");
            #[cfg(target_os = "macos")]
            std::env::set_var("WGPU_BACKEND", "metal");
        }
    }

    let arg_path: Option<PathBuf> = std::env::args().nth(1).map(PathBuf::from);
    let initial_loading = arg_path.is_some();

    let cfg = match Config::load_or_create() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to load/create config: {}. Using defaults.", e);
            Config::default()
        }
    };

    let mut registry = LoaderRegistry::new();
    registry.register(Box::new(loader::stl::StlLoader {}));
    registry.register(Box::new(loader::threemf::ThreemfLoader {}));
    registry.register(Box::new(loader::obj::ObjLoader {}));
    registry.register(Box::new(loader::gltf::GltfLoader {}));
    registry.register(Box::new(loader::fbx::FbxLoader {}));
    let registry = Arc::new(registry);

    let (tx, rx) = channel();

    if let Some(path) = arg_path {
        dialog::load_file_async(&path, registry.clone(), tx.clone());
    }

    let app = App::new(rx, tx, registry, cfg, initial_loading);
    app.run();
}