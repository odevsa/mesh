use std::path::PathBuf;
use std::sync::Arc;

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
    pub source_path: Option<PathBuf>,
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
