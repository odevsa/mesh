use kiss3d::color::Color;
use kiss3d::glamx::{Pose3, Quat, Vec3};
use kiss3d::scene::SceneNode3d;

pub fn build_axes(
    scene_root: &mut SceneNode3d,
    half_size: f32,
    show_axes: [bool; 3],
    show_axis_direction: bool,
) -> SceneNode3d {
    let mut axes_root = scene_root.add_group();
    let axis_width = 2.5;
    let cone_r = half_size * 0.025;
    let cone_h = half_size * 0.08;

    if show_axes[0] {
        let x_color = Color::new(246.0 / 255.0, 54.0 / 255.0, 82.0 / 255.0, 1.0);
        let mut x_pos = Vec::new();
        let mut x_ind = Vec::new();
        x_pos.push(Vec3::new(-half_size, 0.0, 0.0));
        x_pos.push(Vec3::new(half_size, 0.0, 0.0));
        x_pos.push(Vec3::new(-half_size, 0.0, 0.0));
        x_ind.push([0, 1, 2]);
        let mut x_node = axes_root.add_trimesh(x_pos, x_ind, Vec3::new(1.0, 1.0, 1.0), false);
        x_node.set_surface_rendering_activation(false);
        x_node.set_lines_color(Some(x_color));
        x_node.set_lines_width(axis_width, false);

        if show_axis_direction {
            let rot_x = Quat::from_axis_angle(Vec3::Z, -std::f32::consts::FRAC_PI_2);
            let mut cone = axes_root.add_cone(cone_r, cone_h);
            cone.set_color(x_color);
            cone.set_pose(Pose3::from_parts(Vec3::new(half_size, 0.0, 0.0), rot_x));
        }
    }

    if show_axes[1] {
        let y_color = Color::new(126.0 / 255.0, 194.0 / 255.0, 18.0 / 255.0, 1.0);
        let mut y_pos = Vec::new();
        let mut y_ind = Vec::new();
        y_pos.push(Vec3::new(0.0, -half_size, 0.0));
        y_pos.push(Vec3::new(0.0, half_size, 0.0));
        y_pos.push(Vec3::new(0.0, -half_size, 0.0));
        y_ind.push([0, 1, 2]);
        let mut y_node = axes_root.add_trimesh(y_pos, y_ind, Vec3::new(1.0, 1.0, 1.0), false);
        y_node.set_surface_rendering_activation(false);
        y_node.set_lines_color(Some(y_color));
        y_node.set_lines_width(axis_width, false);

        if show_axis_direction {
            let mut cone = axes_root.add_cone(cone_r, cone_h);
            cone.set_color(y_color);
            cone.set_pose(Pose3::from_parts(Vec3::new(0.0, half_size, 0.0), Quat::IDENTITY));
        }
    }

    if show_axes[2] {
        let z_color = Color::new(47.0 / 255.0, 131.0 / 255.0, 227.0 / 255.0, 1.0);
        let mut z_pos = Vec::new();
        let mut z_ind = Vec::new();
        z_pos.push(Vec3::new(0.0, 0.0, -half_size));
        z_pos.push(Vec3::new(0.0, 0.0, half_size));
        z_pos.push(Vec3::new(0.0, 0.0, -half_size));
        z_ind.push([0, 1, 2]);
        let mut z_node = axes_root.add_trimesh(z_pos, z_ind, Vec3::new(1.0, 1.0, 1.0), false);
        z_node.set_surface_rendering_activation(false);
        z_node.set_lines_color(Some(z_color));
        z_node.set_lines_width(axis_width, false);

        if show_axis_direction {
            let rot_z = Quat::from_axis_angle(Vec3::X, std::f32::consts::FRAC_PI_2);
            let mut cone = axes_root.add_cone(cone_r, cone_h);
            cone.set_color(z_color);
            cone.set_pose(Pose3::from_parts(Vec3::new(0.0, 0.0, half_size), rot_z));
        }
    }

    axes_root
}

