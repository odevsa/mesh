use super::{compute_normals, Loader, MeshData, SubMesh};
use std::path::Path;
use std::io::Read;
use std::collections::HashMap;

pub struct FbxLoader {}

fn get_material_color(mat: &ufbx::Material) -> [f32; 4] {
    let (r, g, b, a) = if mat.pbr.base_color.has_value {
        let c = mat.pbr.base_color.value_vec4;
        let factor = if mat.pbr.base_factor.has_value { mat.pbr.base_factor.value_vec4.x as f32 } else { 1.0 };
        (c.x as f32 * factor, c.y as f32 * factor, c.z as f32 * factor, c.w as f32)
    } else if mat.fbx.diffuse_color.has_value {
        let c = mat.fbx.diffuse_color.value_vec4;
        let factor = if mat.fbx.diffuse_factor.has_value { mat.fbx.diffuse_factor.value_vec4.x as f32 } else { 1.0 };
        (c.x as f32 * factor, c.y as f32 * factor, c.z as f32 * factor, c.w as f32)
    } else {
        let c = mat.fbx.diffuse_color.value_vec4;
        (c.x as f32, c.y as f32, c.z as f32, 1.0)
    };

    [
        r.clamp(0.0, 1.0),
        g.clamp(0.0, 1.0),
        b.clamp(0.0, 1.0),
        a,
    ]
}

impl Loader for FbxLoader {
    fn name(&self) -> &str { "fbx" }

    fn can_load(&self, path: &Path, header: &[u8]) -> bool {
        if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
            if ext.eq_ignore_ascii_case("fbx") { return true; }
        }
        if header.starts_with(b"Kaydara FBX Binary") { return true; }
        false
    }

    fn load(&self, path: &Path, _reader: Box<dyn Read>) -> Result<MeshData, String> {
        let mut opts = ufbx::LoadOpts::default();
        opts.target_axes = ufbx::CoordinateAxes {
            up: ufbx::CoordinateAxis::PositiveZ,
            front: ufbx::CoordinateAxis::NegativeY,
            right: ufbx::CoordinateAxis::PositiveX,
        };
        opts.target_unit_meters = 1.0;
        let path_str = path.to_str().ok_or_else(|| "invalid UTF-8 path".to_string())?;
        let scene = ufbx::load_file(path_str, opts).map_err(|e| format!("fbx parse error: {:?}", e))?;

        let base_dir = path.parent().unwrap_or_else(|| Path::new("."));
        let mut tex_cache: HashMap<String, Option<std::sync::Arc<super::TextureData>>> = HashMap::new();

        let load_fbx_texture = |mat: &ufbx::Material, cache: &mut HashMap<String, Option<std::sync::Arc<super::TextureData>>>| -> Option<std::sync::Arc<super::TextureData>> {
            let tex_ref = mat.pbr.base_color.texture.as_ref()
                .or_else(|| mat.fbx.diffuse_color.texture.as_ref())
                .or_else(|| mat.textures.iter().find(|t| t.material_prop == "DiffuseColor" || t.material_prop == "BaseColor").map(|t| &t.texture))
                .or_else(|| mat.textures.first().map(|t| &t.texture));

            let tex = tex_ref?.as_ref();

            // 1. Embedded texture in FBX
            if !tex.content.is_empty() {
                let key = format!("embedded_{}", tex.element.element_id);
                if let Some(cached) = cache.get(&key) {
                    return cached.clone();
                }
                let loaded = super::load_texture_from_memory(&tex.content);
                cache.insert(key, loaded.clone());
                return loaded;
            }

            // 2. Relative filename or filename or absolute filename
            for name in &[&tex.relative_filename, &tex.filename, &tex.absolute_filename] {
                let s = name.as_ref().trim();
                if !s.is_empty() {
                    if let Some(cached) = cache.get(s) {
                        return cached.clone();
                    }
                    if let Some(found_path) = super::resolve_relative_file(base_dir, s) {
                        let loaded = super::load_texture_from_path(&found_path);
                        cache.insert(s.to_string(), loaded.clone());
                        if loaded.is_some() {
                            return loaded;
                        }
                    }
                }
            }

            None
        };

        struct SubMeshAcc {
            positions: Vec<[f32; 3]>,
            normals: Vec<[f32; 3]>,
            uvs: Vec<[f32; 2]>,
            indices: Vec<[u32; 3]>,
            color: Option<[f32; 4]>,
            texture: Option<std::sync::Arc<super::TextureData>>,
        }

        let mut positions = Vec::new();
        let mut normals = Vec::new();
        let mut indices = Vec::new();
        let mut submeshes = Vec::new();

        let mut processed_nodes = 0;

        for node in &scene.nodes {
            let mesh = match &node.mesh {
                Some(m) => m,
                None => continue,
            };
            processed_nodes += 1;

            let get_mat = |idx: usize| -> Option<&ufbx::Material> {
                if let Some(m) = node.materials.get(idx) {
                    Some(m.as_ref())
                } else if let Some(m) = mesh.materials.get(idx) {
                    Some(m.as_ref())
                } else if let Some(m) = node.materials.first() {
                    Some(m.as_ref())
                } else if let Some(m) = mesh.materials.first() {
                    Some(m.as_ref())
                } else {
                    None
                }
            };

            let mut mat_subs: HashMap<Option<u32>, SubMeshAcc> = HashMap::new();

            for (face_idx, face) in mesh.faces.iter().enumerate() {
                let raw_mat_idx = if !mesh.materials.is_empty() && mesh.face_material.len() > face_idx {
                    mesh.face_material[face_idx]
                } else {
                    0
                };
                let mat_idx = raw_mat_idx as usize;
                let mat_opt = get_mat(mat_idx);
                let mat_key = mat_opt.map(|m| m.element.element_id);

                let sub = mat_subs.entry(mat_key).or_insert_with(|| {
                    let color = mat_opt.map(get_material_color);
                    let texture = mat_opt.and_then(|m| load_fbx_texture(m, &mut tex_cache));
                    SubMeshAcc {
                        positions: Vec::new(),
                        normals: Vec::new(),
                        uvs: Vec::new(),
                        indices: Vec::new(),
                        color,
                        texture,
                    }
                });

                if face.num_indices >= 3 {
                    for t in 0..(face.num_indices - 2) {
                        let corners = [
                            face.index_begin + 0,
                            face.index_begin + t + 1,
                            face.index_begin + t + 2,
                        ];
                        let tri_start = sub.positions.len() as u32;
                        for &corner_idx in &corners {
                            let p_idx = mesh.vertex_indices[corner_idx as usize] as usize;
                            let p = mesh.vertices[p_idx];
                            let tp = ufbx::transform_position(&node.node_to_world, p);
                            sub.positions.push([tp.x as f32, tp.y as f32, tp.z as f32]);

                            let norm = if mesh.vertex_normal.exists {
                                let raw_n = if !mesh.vertex_normal.indices.is_empty() && (corner_idx as usize) < mesh.vertex_normal.indices.len() {
                                    let val_idx = mesh.vertex_normal.indices[corner_idx as usize] as usize;
                                    mesh.vertex_normal.values.get(val_idx).copied()
                                } else if (corner_idx as usize) < mesh.vertex_normal.values.len() {
                                    mesh.vertex_normal.values.get(corner_idx as usize).copied()
                                } else if p_idx < mesh.vertex_normal.values.len() {
                                    mesh.vertex_normal.values.get(p_idx).copied()
                                } else {
                                    None
                                };

                                if let Some(n) = raw_n {
                                    let tn = ufbx::transform_direction(&node.node_to_world, n);
                                    [tn.x as f32, tn.y as f32, tn.z as f32]
                                } else {
                                    [0.0, 0.0, 1.0]
                                }
                            } else {
                                [0.0, 0.0, 1.0]
                            };
                            sub.normals.push(norm);

                            let uv = if mesh.vertex_uv.exists {
                                let raw_u = if !mesh.vertex_uv.indices.is_empty() && (corner_idx as usize) < mesh.vertex_uv.indices.len() {
                                    let val_idx = mesh.vertex_uv.indices[corner_idx as usize] as usize;
                                    mesh.vertex_uv.values.get(val_idx).copied()
                                } else if (corner_idx as usize) < mesh.vertex_uv.values.len() {
                                    mesh.vertex_uv.values.get(corner_idx as usize).copied()
                                } else if p_idx < mesh.vertex_uv.values.len() {
                                    mesh.vertex_uv.values.get(p_idx).copied()
                                } else {
                                    None
                                };

                                if let Some(u) = raw_u {
                                    [u.x as f32, 1.0 - u.y as f32]
                                } else {
                                    [0.0, 0.0]
                                }
                            } else {
                                [0.0, 0.0]
                            };
                            sub.uvs.push(uv);
                        }
                        sub.indices.push([tri_start, tri_start + 1, tri_start + 2]);
                    }
                }
            }

            for (_, acc) in mat_subs {
                let base_vertex = positions.len() as u32;
                for tri in &acc.indices {
                    indices.push([tri[0] + base_vertex, tri[1] + base_vertex, tri[2] + base_vertex]);
                }
                positions.extend_from_slice(&acc.positions);
                normals.extend_from_slice(&acc.normals);

                submeshes.push(SubMesh {
                    positions: acc.positions,
                    normals: acc.normals,
                    uvs: acc.uvs,
                    indices: acc.indices,
                    color: acc.color,
                    texture: acc.texture,
                });
            }
        }

        if processed_nodes == 0 {
            for mesh in &scene.meshes {
                let get_mat = |idx: usize| -> Option<&ufbx::Material> {
                    if let Some(m) = mesh.materials.get(idx) {
                        Some(m.as_ref())
                    } else if let Some(m) = mesh.materials.first() {
                        Some(m.as_ref())
                    } else {
                        None
                    }
                };

                let mut mat_subs: HashMap<Option<u32>, SubMeshAcc> = HashMap::new();

                for (face_idx, face) in mesh.faces.iter().enumerate() {
                    let raw_mat_idx = if !mesh.materials.is_empty() && mesh.face_material.len() > face_idx {
                        mesh.face_material[face_idx]
                    } else {
                        0
                    };
                    let mat_idx = raw_mat_idx as usize;
                    let mat_opt = get_mat(mat_idx);
                    let mat_key = mat_opt.map(|m| m.element.element_id);

                    let sub = mat_subs.entry(mat_key).or_insert_with(|| {
                        let color = mat_opt.map(get_material_color);
                        let texture = mat_opt.and_then(|m| load_fbx_texture(m, &mut tex_cache));
                        SubMeshAcc {
                            positions: Vec::new(),
                            normals: Vec::new(),
                            uvs: Vec::new(),
                            indices: Vec::new(),
                            color,
                            texture,
                        }
                    });

                    if face.num_indices >= 3 {
                        for t in 0..(face.num_indices - 2) {
                            let corners = [
                                face.index_begin + 0,
                                face.index_begin + t + 1,
                                face.index_begin + t + 2,
                            ];
                            let tri_start = sub.positions.len() as u32;
                            for &corner_idx in &corners {
                                let p_idx = mesh.vertex_indices[corner_idx as usize] as usize;
                                let p = mesh.vertices[p_idx];
                                sub.positions.push([p.x as f32, p.y as f32, p.z as f32]);

                                let norm = if mesh.vertex_normal.exists {
                                    let raw_n = if !mesh.vertex_normal.indices.is_empty() && (corner_idx as usize) < mesh.vertex_normal.indices.len() {
                                        let val_idx = mesh.vertex_normal.indices[corner_idx as usize] as usize;
                                        mesh.vertex_normal.values.get(val_idx).copied()
                                    } else if (corner_idx as usize) < mesh.vertex_normal.values.len() {
                                        mesh.vertex_normal.values.get(corner_idx as usize).copied()
                                    } else if p_idx < mesh.vertex_normal.values.len() {
                                        mesh.vertex_normal.values.get(p_idx).copied()
                                    } else {
                                        None
                                    };

                                    if let Some(n) = raw_n {
                                        [n.x as f32, n.y as f32, n.z as f32]
                                    } else {
                                        [0.0, 0.0, 1.0]
                                    }
                                } else {
                                    [0.0, 0.0, 1.0]
                                };
                                sub.normals.push(norm);

                                let uv = if mesh.vertex_uv.exists {
                                    let raw_u = if !mesh.vertex_uv.indices.is_empty() && (corner_idx as usize) < mesh.vertex_uv.indices.len() {
                                        let val_idx = mesh.vertex_uv.indices[corner_idx as usize] as usize;
                                        mesh.vertex_uv.values.get(val_idx).copied()
                                    } else if (corner_idx as usize) < mesh.vertex_uv.values.len() {
                                        mesh.vertex_uv.values.get(corner_idx as usize).copied()
                                    } else if p_idx < mesh.vertex_uv.values.len() {
                                        mesh.vertex_uv.values.get(p_idx).copied()
                                    } else {
                                        None
                                    };

                                    if let Some(u) = raw_u {
                                        [u.x as f32, 1.0 - u.y as f32]
                                    } else {
                                        [0.0, 0.0]
                                    }
                                } else {
                                    [0.0, 0.0]
                                };
                                sub.uvs.push(uv);
                            }
                            sub.indices.push([tri_start, tri_start + 1, tri_start + 2]);
                        }
                    }
                }

                for (_, acc) in mat_subs {
                    let base_vertex = positions.len() as u32;
                    for tri in &acc.indices {
                        indices.push([tri[0] + base_vertex, tri[1] + base_vertex, tri[2] + base_vertex]);
                    }
                    positions.extend_from_slice(&acc.positions);
                    normals.extend_from_slice(&acc.normals);

                    submeshes.push(SubMesh {
                        positions: acc.positions,
                        normals: acc.normals,
                        uvs: acc.uvs,
                        indices: acc.indices,
                        color: acc.color,
                        texture: acc.texture,
                    });
                }
            }
        }

        if positions.is_empty() {
            return Err("fbx: no positions found in model".into());
        }

        Ok(MeshData {
            positions,
            normals,
            indices,
            submeshes,
            ..Default::default()
        })
    }

    fn can_export(&self, path: &Path) -> bool {
        if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
            if ext.eq_ignore_ascii_case("fbx") {
                return true;
            }
        }
        false
    }

    fn export(&self, path: &Path, mesh: &MeshData) -> Result<(), String> {
        let fallback_submesh;
        let submeshes: &[SubMesh] = if !mesh.submeshes.is_empty() {
            &mesh.submeshes
        } else if !mesh.positions.is_empty() && !mesh.indices.is_empty() {
            fallback_submesh = vec![SubMesh {
                positions: mesh.positions.clone(),
                normals: mesh.normals.clone(),
                uvs: mesh.uvs.clone(),
                indices: mesh.indices.clone(),
                color: None,
                texture: None,
            }];
            &fallback_submesh
        } else {
            return Err("Mesh contains no geometry to export".into());
        };

        use std::io::Write;
        let file = std::fs::File::create(path).map_err(|e| format!("create fbx file failed: {}", e))?;
        let mut writer = std::io::BufWriter::new(file);

        let mut total_verts = 0usize;
        let mut total_tris = 0usize;
        for sub in submeshes {
            total_verts += sub.positions.len();
            total_tris += sub.indices.len();
        }
        let total_indices = total_tris * 3;

        writeln!(
            writer,
            "; FBX 7.4.0 project file\n; Generated by Mesh App\n\nFBXHeaderExtension:  {{\n\tFBXHeaderVersion: 1003\n\tFBXVersion: 7400\n}}\n\nGlobalSettings:  {{\n\tVersion: 1000\n\tProperties70:  {{\n\t\tP: \"UpAxis\", \"int\", \"Integer\", \"\", 2\n\t\tP: \"UpAxisSign\", \"int\", \"Integer\", \"\", 1\n\t\tP: \"FrontAxis\", \"int\", \"Integer\", \"\", 1\n\t\tP: \"FrontAxisSign\", \"int\", \"Integer\", \"\", -1\n\t\tP: \"CoordAxis\", \"int\", \"Integer\", \"\", 0\n\t\tP: \"CoordAxisSign\", \"int\", \"Integer\", \"\", 1\n\t\tP: \"UnitScaleFactor\", \"double\", \"Number\", \"\", 100.0\n\t\tP: \"OriginalUnitScaleFactor\", \"double\", \"Number\", \"\", 100.0\n\t}}\n}}\n\nObjects:  {{\n\tGeometry: 100000, \"Geometry::Mesh\", \"Mesh\" {{\n\t\tVertices: *{} {{",
            total_verts * 3
        ).map_err(|e| format!("write: {}", e))?;

        write!(writer, "\t\t\ta: ").map_err(|e| format!("write: {}", e))?;
        let mut first = true;
        for sub in submeshes {
            for p in &sub.positions {
                if !first {
                    write!(writer, ",").map_err(|e| format!("write: {}", e))?;
                }
                first = false;
                write!(writer, "{:.6},{:.6},{:.6}", p[0], p[1], p[2]).map_err(|e| format!("write: {}", e))?;
            }
        }
        writeln!(writer, "\n\t\t}}\n\t\tPolygonVertexIndex: *{} {{", total_indices).map_err(|e| format!("write: {}", e))?;

        write!(writer, "\t\t\ta: ").map_err(|e| format!("write: {}", e))?;
        let mut first_poly = true;
        let mut v_offset = 0i32;
        for sub in submeshes {
            for tri in &sub.indices {
                if !first_poly {
                    write!(writer, ",").map_err(|e| format!("write: {}", e))?;
                }
                first_poly = false;
                let i0 = v_offset + tri[0] as i32;
                let i1 = v_offset + tri[1] as i32;
                let i2 = -(v_offset + tri[2] as i32) - 1; // Bitwise NOT marks polygon end in FBX
                write!(writer, "{},{},{}", i0, i1, i2).map_err(|e| format!("write: {}", e))?;
            }
            v_offset += sub.positions.len() as i32;
        }

        writeln!(
            writer,
            "\n\t\t}}\n\t\tGeometryVersion: 124\n\t\tLayerElementNormal: 0 {{\n\t\t\tVersion: 101\n\t\t\tName: \"\"\n\t\t\tMappingInformationType: \"ByPolygonVertex\"\n\t\t\tReferenceInformationType: \"Direct\"\n\t\t\tNormals: *{} {{",
            total_indices * 3
        ).map_err(|e| format!("write: {}", e))?;

        write!(writer, "\t\t\ta: ").map_err(|e| format!("write: {}", e))?;
        let mut first_norm = true;
        for sub in submeshes {
            let sub_normals: Vec<[f32; 3]> = if sub.normals.len() == sub.positions.len() {
                sub.normals.clone()
            } else {
                compute_normals(&sub.positions, &sub.indices)
            };
            for tri in &sub.indices {
                for &idx in tri {
                    let n = sub_normals.get(idx as usize).copied().unwrap_or([0.0, 0.0, 1.0]);
                    if !first_norm {
                        write!(writer, ",").map_err(|e| format!("write: {}", e))?;
                    }
                    first_norm = false;
                    write!(writer, "{:.6},{:.6},{:.6}", n[0], n[1], n[2]).map_err(|e| format!("write: {}", e))?;
                }
            }
        }

        // Close Normals and LayerElementNormal
        writeln!(writer, "\n\t\t\t}}\n\t\t}}").map_err(|e| format!("write: {}", e))?;

        // LayerElementMaterial
        writeln!(
            writer,
            "\t\tLayerElementMaterial: 0 {{\n\t\t\tVersion: 101\n\t\t\tName: \"\"\n\t\t\tMappingInformationType: \"ByPolygon\"\n\t\t\tReferenceInformationType: \"IndexToDirect\"\n\t\t\tMaterials: *{} {{\n\t\t\t\ta: ",
            total_tris
        ).map_err(|e| format!("write: {}", e))?;

        let mut first_mat = true;
        for (mat_idx, sub) in submeshes.iter().enumerate() {
            for _ in &sub.indices {
                if !first_mat {
                    write!(writer, ",").map_err(|e| format!("write: {}", e))?;
                }
                first_mat = false;
                write!(writer, "{}", mat_idx).map_err(|e| format!("write: {}", e))?;
            }
        }

        // Close Materials, LayerElementMaterial, and Geometry
        writeln!(
            writer,
            "\n\t\t\t}}\n\t\t}}\n\t}}\n\tModel: 200000, \"Model::MeshModel\", \"Mesh\" {{\n\t\tVersion: 232\n\t\tProperties70:  {{\n\t\t\tP: \"InheritType\", \"enum\", \"\", \"\",1\n\t\t}}\n\t}}"
        ).map_err(|e| format!("write: {}", e))?;

        // Write Material objects
        for (mat_idx, sub) in submeshes.iter().enumerate() {
            let color = sub.color.unwrap_or([0.8, 0.8, 0.8, 1.0]);
            let mat_id = 300000 + mat_idx;
            writeln!(
                writer,
                "\tMaterial: {}, \"Material::Material_{}\", \"\" {{\n\t\tVersion: 102\n\t\tShadingModel: \"phong\"\n\t\tMultiLayer: 0\n\t\tProperties70:  {{\n\t\t\tP: \"DiffuseColor\", \"Color\", \"\", \"A\", {:.4}, {:.4}, {:.4}\n\t\t\tP: \"DiffuseFactor\", \"Number\", \"\", \"A\", 1.0\n\t\t\tP: \"AmbientColor\", \"Color\", \"\", \"A\", 0.0, 0.0, 0.0\n\t\t\tP: \"SpecularColor\", \"Color\", \"\", \"A\", 0.2, 0.2, 0.2\n\t\t\tP: \"Shininess\", \"Number\", \"\", \"A\", 20.0\n\t\t\tP: \"Opacity\", \"Number\", \"\", \"A\", {:.4}\n\t\t}}\n\t}}",
                mat_id, mat_idx + 1, color[0], color[1], color[2], color[3]
            ).map_err(|e| format!("write: {}", e))?;
        }

        writeln!(writer, "}}\n\nConnections:  {{\n\t;Model::MeshModel, RootNode\n\tC: \"OO\",200000,0\n\t;Geometry::Mesh, Model::MeshModel\n\tC: \"OO\",100000,200000").map_err(|e| format!("write: {}", e))?;

        for (mat_idx, _) in submeshes.iter().enumerate() {
            let mat_id = 300000 + mat_idx;
            writeln!(writer, "\t;Material::Material_{}, Model::MeshModel\n\tC: \"OO\",{},200000", mat_idx + 1, mat_id).map_err(|e| format!("write: {}", e))?;
        }

        writeln!(writer, "}}").map_err(|e| format!("write: {}", e))?;

        writer.flush().map_err(|e| format!("flush: {}", e))?;
        println!("[FBX Format] Successfully exported ASCII FBX with {} triangles and {} materials to '{}'", total_tris, submeshes.len(), path.display());
        Ok(())
    }

    fn supports_animation(&self) -> bool {
        println!("[FBX Format] Format natively supports animation tracks (skeletal/skinning)");
        true
    }
}
