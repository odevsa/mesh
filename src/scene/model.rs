use crate::config::ModelPosition;
use kiss3d::glamx::Vec3;

pub fn compute_model_z_offset(position: ModelPosition, model_height: f32, scale: f32) -> f32 {
    let half_height = (model_height * scale) / 2.0;
    match position {
        ModelPosition::Above => half_height,
        ModelPosition::Center => 0.0,
        ModelPosition::Below => -half_height,
    }
}

pub fn compute_bounds(positions: &[[f32; 3]]) -> (Vec3, Vec3) {
    let mut min = Vec3::new(f32::INFINITY, f32::INFINITY, f32::INFINITY);
    let mut max = Vec3::new(f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY);
    for p in positions {
        min.x = min.x.min(p[0]);
        min.y = min.y.min(p[1]);
        min.z = min.z.min(p[2]);
        max.x = max.x.max(p[0]);
        max.y = max.y.max(p[1]);
        max.z = max.z.max(p[2]);
    }
    (min, max)
}

pub fn center_vertices(positions: &[[f32; 3]], center_offset: Vec3) -> Vec<Vec3> {
    positions
        .iter()
        .map(|p| Vec3::new(p[0], p[1], p[2]) - center_offset)
        .collect()
}

