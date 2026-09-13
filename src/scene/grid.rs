use kiss3d::color::Color;
use kiss3d::glamx::Vec3;
use kiss3d::scene::SceneNode3d;

pub fn build_grid(
    scene_root: &mut SceneNode3d,
    half_size: f32,
    divisions: u32,
    color: Color,
    show_axes: [bool; 3],
) -> SceneNode3d {
    let divisions = divisions.max(1);
    let step = (half_size * 2.0) / divisions as f32;

    let mut positions: Vec<Vec3> = Vec::new();
    let mut indices: Vec<[u32; 3]> = Vec::new();

    for i in 0..=divisions {
        let x = -half_size + i as f32 * step;
        if show_axes[1] && x.abs() < 1e-4 {
            continue;
        }
        let i0 = positions.len() as u32;
        positions.push(Vec3::new(x, -half_size, 0.0));
        positions.push(Vec3::new(x, half_size, 0.0));
        positions.push(Vec3::new(x, -half_size, 0.0));
        indices.push([i0, i0 + 1, i0 + 2]);
    }

    for j in 0..=divisions {
        let y = -half_size + j as f32 * step;
        if show_axes[0] && y.abs() < 1e-4 {
            continue;
        }
        let i0 = positions.len() as u32;
        positions.push(Vec3::new(-half_size, y, 0.0));
        positions.push(Vec3::new(half_size, y, 0.0));
        positions.push(Vec3::new(-half_size, y, 0.0));
        indices.push([i0, i0 + 1, i0 + 2]);
    }

    let mut grid_node = scene_root.add_trimesh(
        positions,
        indices,
        Vec3::new(1.0, 1.0, 1.0),
        false,
    );
    grid_node.set_surface_rendering_activation(false);
    grid_node.set_lines_color(Some(color));
    grid_node.set_lines_width(1.0, false);
    grid_node
}

