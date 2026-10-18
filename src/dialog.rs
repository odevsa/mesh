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
            "3D Models (*.stl, *.3mf, *.obj, *.gltf, *.glb, *.fbx)",
            &["stl", "3mf", "obj", "gltf", "glb", "fbx"],
        )
        .pick_file()
    {
        load_file_async(&path, registry, tx);
        true
    } else {
        false
    }
}

pub fn trigger_export_dialog(
    format: &str,
    registry: Arc<LoaderRegistry>,
    mesh: &MeshData,
) -> bool {
    let (filter_name, ext, default_name) = match format {
        "stl" => ("Stereolithography (*.stl)", "stl", "model.stl"),
        "obj" => ("Wavefront OBJ (*.obj)", "obj", "model.obj"),
        "glb" => ("glTF Binary (*.glb)", "glb", "model.glb"),
        "gltf" => ("glTF Embedded (*.gltf)", "gltf", "model.gltf"),
        "3mf" => ("3D Manufacturing (*.3mf)", "3mf", "model.3mf"),
        "fbx" => ("Autodesk FBX (*.fbx)", "fbx", "model.fbx"),
        _ => return false,
    };

    let mut dialog = FileDialog::new()
        .add_filter(filter_name, &[ext])
        .set_file_name(default_name);

    if let Some(ref source) = mesh.source_path {
        if let Some(stem) = source.file_stem().and_then(|s| s.to_str()) {
            dialog = dialog.set_file_name(&format!("{}.{}", stem, ext));
        }
        if let Some(parent) = source.parent() {
            dialog = dialog.set_directory(parent);
        }
    }

    if let Some(path) = dialog.save_file() {
        match registry.export_path(&path, mesh) {
            Ok(()) => {
                println!("Successfully exported model to '{}'", path.display());
                true
            }
            Err(e) => {
                eprintln!("Failed to export model to '{}': {}", path.display(), e);
                false
            }
        }
    } else {
        false
    }
}

