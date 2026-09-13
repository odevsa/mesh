#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod camera;
mod config;
mod dialog;
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
    let registry = Arc::new(registry);

    let (tx, rx) = channel();

    if let Some(path) = arg_path {
        dialog::load_file_async(&path, registry.clone(), tx.clone());
    }

    let mut app = App::new(rx, tx, registry, cfg, initial_loading);
    app.run();
}