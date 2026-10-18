#[allow(dead_code)]
pub fn compute_face_normal(v0: [f32; 3], v1: [f32; 3], v2: [f32; 3]) -> [f32; 3] {
    let e1 = glam::Vec3::from(v1) - glam::Vec3::from(v0);
    let e2 = glam::Vec3::from(v2) - glam::Vec3::from(v0);
    let c = e1.cross(e2);
    if c.length_squared() > 1e-20 {
        c.normalize().into()
    } else {
        [0.0, 0.0, 1.0]
    }
}

pub fn compute_normals(positions: &[[f32; 3]], indices: &[[u32; 3]]) -> Vec<[f32; 3]> {
    let mut normals = vec![[0.0f32; 3]; positions.len()];
    for tri in indices {
        let i0 = tri[0] as usize;
        let i1 = tri[1] as usize;
        let i2 = tri[2] as usize;
        if i0 < positions.len() && i1 < positions.len() && i2 < positions.len() {
            let p0 = glam::Vec3::from(positions[i0]);
            let p1 = glam::Vec3::from(positions[i1]);
            let p2 = glam::Vec3::from(positions[i2]);

            let v01 = p1 - p0;
            let v02 = p2 - p0;
            let v12 = p2 - p1;

            let fnorm = v01.cross(v02);
            let fn_len = fnorm.length();
            if fn_len < 1e-6 {
                continue;
            }
            let fn_norm = fnorm / fn_len;

            let l01 = v01.length();
            let l02 = v02.length();
            let l12 = v12.length();

            if l01 > 1e-6 && l02 > 1e-6 && l12 > 1e-6 {
                let a0 = (v01.dot(v02) / (l01 * l02)).clamp(-1.0, 1.0).acos();
                let a1 = ((-v01.dot(v12)) / (l01 * l12)).clamp(-1.0, 1.0).acos();
                let a2 = ((-v02.dot(-v12)) / (l02 * l12)).clamp(-1.0, 1.0).acos();

                let n0 = glam::Vec3::from(normals[i0]) + fn_norm * a0;
                let n1 = glam::Vec3::from(normals[i1]) + fn_norm * a1;
                let n2 = glam::Vec3::from(normals[i2]) + fn_norm * a2;

                normals[i0] = n0.into();
                normals[i1] = n1.into();
                normals[i2] = n2.into();
            }
        }
    }
    for n in &mut normals {
        let v = glam::Vec3::from(*n);
        *n = if v.length_squared() > 1e-6 {
            v.normalize().into()
        } else {
            [0.0, 0.0, 1.0]
        };
    }
    normals
}
