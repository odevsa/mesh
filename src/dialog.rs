use crate::loader::{LoaderRegistry, MeshData};
use rfd::FileDialog;
use std::path::Path;
use std::sync::mpsc::Sender;
use std::sync::Arc;

pub fn load_file_async(
    path: &Path,
    registry: Arc<LoaderRegistry>,
    tx: Sender<Result<MeshData, String>>,
) {
    let path = path.to_path_buf();
    std::thread::spawn(move || {
        match registry.load_path(&path) {
            Ok(mesh) => {
                let _ = tx.send(Ok(mesh));
            }
            Err(e) => {
                eprintln!("Failed to load mesh {}: {}", path.display(), e);
                let _ = tx.send(Err(format!(
                    "Could not open '{}'. The format may be unsupported or the file may be corrupted.",
                    path.display()
                )));
            }
        }
    });
}

pub fn trigger_file_dialog(
    registry: Arc<LoaderRegistry>,
    tx: Sender<Result<MeshData, String>>,
) -> bool {
    if let Some(path) = FileDialog::new()
        .add_filter(
            "3D Models (*.stl, *.3mf, *.obj, *.gltf, *.glb)",
            &["stl", "3mf", "obj", "gltf", "glb"],
        )
        .pick_file()
    {
        load_file_async(&path, registry, tx);
        true
    } else {
        false
    }
}

