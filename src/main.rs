#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod camera;
mod config;
mod dialog;
mod i18n;
mod loader;
mod render;
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

    let registry = Arc::new(LoaderRegistry::default_registry());

    let (tx, rx) = channel();

    if let Some(ref path) = arg_path {
        dialog::load_file_async(path, registry.clone(), tx.clone());
    }

    let icon = {
        let icon_bytes = include_bytes!("../packaging/linux/assets/icons/hicolor/256x256/apps/mesh.png");
        image::load_from_memory(icon_bytes).ok().map(|img| {
            let rgba = img.to_rgba8();
            let (width, height) = rgba.dimensions();
            egui::IconData {
                rgba: rgba.into_raw(),
                width,
                height,
            }
        })
    };

    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([800.0, 600.0])
        .with_min_inner_size([320.0, 240.0])
        .with_title(if initial_loading { "Mesh - Loading..." } else { "Mesh" })
        .with_app_id("mesh")
        .with_drag_and_drop(true);

    if let Some(icon) = icon {
        viewport = viewport.with_icon(icon);
    }

    let mut native_options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    #[cfg(target_os = "linux")]
    {
        use winit::platform::x11::EventLoopBuilderExtX11;
        native_options.event_loop_builder = Some(Box::new(|builder| {
            // Under Wayland, winit 0.30's Wayland backend lacks WindowEvent::DroppedFile support entirely.
            // On almost all Wayland desktops (GNOME, KDE, etc.), XWayland is available via DISPLAY.
            // When DISPLAY is set, using X11 allows XDnD to work seamlessly across file managers.
            if std::env::var("DISPLAY").map(|v| !v.is_empty()).unwrap_or(false)
                && std::env::var("MESH_FORCE_WAYLAND").is_err()
            {
                builder.with_x11();
            }
        }));
    }

    if let Err(e) = eframe::run_native(
        "Mesh",
        native_options,
        Box::new(move |cc| {
            Ok(Box::new(App::new(cc, rx, tx, registry, cfg, initial_loading)))
        }),
    ) {
        eprintln!("Failed to run eframe application: {}", e);
    }
}