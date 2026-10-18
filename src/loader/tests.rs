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
    assert!(is_supported_file(&PathBuf::from("model.dae")));
    assert!(is_supported_file(&PathBuf::from("model.DAE")));

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
        "assets/models/Suzanne.dae",
    ];
    for f in files {
        let path = PathBuf::from(f);
        if path.exists() {
            let mesh = registry.load_path(&path).unwrap_or_else(|e| panic!("Failed to load {}: {}", f, e));
            println!("--- Model: {} ---", f);
            println!("  positions: {}", mesh.positions.len());
            println!("  normals: {}", mesh.normals.len());
            println!("  submeshes: {}", mesh.submeshes.len());
            for (i, sub) in mesh.submeshes.iter().enumerate() {
                println!("    submesh[{}]: color={:?}, tex={:?}, verts={}", i, sub.color, sub.texture.is_some(), sub.positions.len());
            }
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

    let mut has_dark_pupil = false;
    let mut has_bright_eye = false;

    for sub in &mesh.submeshes {
        if let Some(col) = sub.color {
            if sub.indices.len() == 896 {
                assert!(col[0] < 0.25 && col[1] < 0.25 && col[2] < 0.25, "Pupil should be dark #282828, got {:?}", col);
                has_dark_pupil = true;
            } else if sub.indices.len() == 552 {
                assert!(col[0] > 0.75 && col[1] > 0.65 && col[2] > 0.65, "Eye should be off-white #E7D6D6, got {:?}", col);
                has_bright_eye = true;
            }
        }
    }

    assert!(has_dark_pupil, "Must have dark pupil submesh");
    assert!(has_bright_eye, "Must have bright eye submesh");
}

#[test]
fn test_collada_suzanne_colors_and_submeshes() {
    let registry = LoaderRegistry::default_registry();
    let path = PathBuf::from("assets/models/Suzanne.dae");
    if !path.exists() {
        return;
    }
    let mesh = registry.load_path(&path).expect("Failed to load Suzanne.dae");
    assert_eq!(mesh.submeshes.len(), 4, "Suzanne.dae should have 4 submeshes (Skin, Eye, Pupil, Ear)");

    let mut has_dark_pupil = false;
    let mut has_bright_eye = false;
    let mut has_red_ear = false;

    for sub in &mesh.submeshes {
        if let Some(col) = sub.color {
            // Check that it's not all black
            if sub.indices.len() == 896 {
                assert!(col[0] < 0.1 && col[1] < 0.1 && col[2] < 0.1, "Pupil should be dark, got {:?}", col);
                has_dark_pupil = true;
            } else if sub.indices.len() == 552 {
                assert!(col[0] > 0.7 && col[1] > 0.6 && col[2] > 0.6, "Eye should be bright/off-white, got {:?}", col);
                has_bright_eye = true;
            } else if sub.indices.len() == 1712 {
                assert!(col[0] > 0.8 && col[1] > 0.2 && col[2] > 0.2, "Ear should be reddish, got {:?}", col);
                has_red_ear = true;
            }
        }
    }

    assert!(has_dark_pupil, "Must have dark pupil submesh in COLLADA");
    assert!(has_bright_eye, "Must have bright eye submesh in COLLADA");
    assert!(has_red_ear, "Must have reddish ear submesh in COLLADA");
}

#[test]
fn test_collada_export_roundtrip() {
    let registry = LoaderRegistry::default_registry();
    let path = PathBuf::from("assets/models/Suzanne.dae");
    if !path.exists() {
        return;
    }
    let original = registry.load_path(&path).expect("Failed to load Suzanne.dae");

    let temp_dir = std::env::temp_dir().join("mesh_collada_roundtrip");
    let _ = std::fs::create_dir_all(&temp_dir);
    let export_path = temp_dir.join("suzanne_export.dae");

    registry.export_path(&export_path, &original).expect("Failed to export COLLADA");
    assert!(export_path.exists());

    let reloaded = registry.load_path(&export_path).expect("Failed to reload exported COLLADA");
    assert_eq!(reloaded.submeshes.len(), original.submeshes.len());
    for (orig_sub, reload_sub) in original.submeshes.iter().zip(reloaded.submeshes.iter()) {
        assert_eq!(orig_sub.indices.len(), reload_sub.indices.len());
        assert_eq!(orig_sub.positions.len(), reload_sub.positions.len());
        if let (Some(c1), Some(c2)) = (orig_sub.color, reload_sub.color) {
            assert!((c1[0] - c2[0]).abs() < 0.01);
            assert!((c1[1] - c2[1]).abs() < 0.01);
            assert!((c1[2] - c2[2]).abs() < 0.01);
        }
    }
    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_resolve_relative_file() {
    let temp_dir = std::env::temp_dir().join("mesh_test_rel_resolve");
    let textures_dir = temp_dir.join("textures");
    let _ = std::fs::create_dir_all(&textures_dir);

    let img_path = textures_dir.join("albedo.png");
    let img = image::RgbaImage::new(2, 2);
    let _ = img.save(&img_path);

    let r1 = resolve_relative_file(&temp_dir, "textures\\albedo.png");
    assert!(r1.is_some());
    assert_eq!(r1.unwrap().canonicalize().ok(), img_path.canonicalize().ok());

    let r2 = resolve_relative_file(&temp_dir, "albedo.png");
    assert!(r2.is_some());

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
        ("model.dae", true),
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
        ("suzanne_exp.dae", true),
    ];

    for (filename, expect_materials) in targets {
        let out_path = temp_dir.join(filename);
        registry.export_path(&out_path, &orig).unwrap_or_else(|e| panic!("export to {}: {}", filename, e));
        assert!(out_path.exists(), "exported file {} must exist", filename);

        let reloaded = registry.load_path(&out_path).unwrap_or_else(|e| panic!("reload {}: {}", filename, e));
        assert!(!reloaded.positions.is_empty(), "reloaded {} should have positions", filename);
        assert!(!reloaded.submeshes.is_empty(), "reloaded {} should have submeshes", filename);

        if expect_materials {
            let has_material_colors = reloaded.submeshes.iter().any(|s| s.color.is_some());
            assert!(has_material_colors, "reloaded {} must preserve material colors", filename);
        }
    }

    let _ = std::fs::remove_dir_all(&temp_dir);
}
