#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod loader;
mod config;

use rfd::FileDialog;
use std::path::PathBuf;
use loader::{LoaderRegistry, MeshData};
use config::Config;

fn main() {
    let arg_path: Option<PathBuf> = std::env::args().nth(1).map(PathBuf::from);
    let initial_loading = arg_path.is_some();

    let cfg: Config = match Config::load_or_create() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to load/create config: {}. Using defaults.", e);
            Config::default()
        }
    };

    let mut registry = LoaderRegistry::new();
    registry.register(Box::new(loader::stl::StlLoader {}));
    registry.register(Box::new(loader::threemf::ThreemfLoader {}));
    registry.register(Box::new(loader::obj::ObjLoader {}));
    registry.register(Box::new(loader::gltf::GltfLoader {}));
    let registry = std::sync::Arc::new(registry);

    use std::sync::mpsc::channel;

    let (tx, rx) = channel::<Result<loader::MeshData, String>>();

    if let Some(p) = arg_path {
        let txc = tx.clone();
        let reg = registry.clone();
        std::thread::spawn(move || {
            match reg.load_path(&p) {
                Ok(m) => { let _ = txc.send(Ok(m)); }
                Err(e) => {
                    eprintln!("Failed to load mesh {}: {}", p.display(), e);
                    let _ = txc.send(Err(format!("Could not open '{}'. The format may be unsupported or the file is corrupted.", p.display())));
                }
            }
        });
    }

    start_viewer(rx, tx, registry, cfg, initial_loading);
}

fn start_viewer(
    rx: std::sync::mpsc::Receiver<Result<MeshData, String>>,
    tx: std::sync::mpsc::Sender<Result<MeshData, String>>,
    registry: std::sync::Arc<loader::LoaderRegistry>,
    cfg: Config,
    initial_loading: bool,
) {
    render::run(rx, tx, registry, cfg, initial_loading)
}

mod render {
    use super::*;
    use kiss3d::window::Window;
    use kiss3d::camera::OrbitCamera3d;
    use kiss3d::glamx::{Pose3, Quat, Vec3};
    use pollster;

    fn build_grid(
        scene_root: &mut kiss3d::scene::SceneNode3d,
        half_size: f32,
        divisions: u32,
        color: kiss3d::color::Color,
        show_axes: [bool; 3],
    ) -> kiss3d::scene::SceneNode3d {
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
            positions.push(Vec3::new(x,  half_size, 0.0));
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
            positions.push(Vec3::new( half_size, y, 0.0));
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

    fn build_axes(
        scene_root: &mut kiss3d::scene::SceneNode3d,
        half_size: f32,
        show_axes: [bool; 3],
        show_axis_direction: bool,
    ) -> kiss3d::scene::SceneNode3d {
        let mut axes_root = scene_root.add_group();
        let axis_width = 2.5;
        let cone_r = half_size * 0.025;
        let cone_h = half_size * 0.08;

        if show_axes[0] {
            let x_color = kiss3d::color::Color::new(246.0 / 255.0, 54.0 / 255.0, 82.0 / 255.0, 1.0);
            let mut x_pos = Vec::new();
            let mut x_ind = Vec::new();
            x_pos.push(Vec3::new(-half_size, 0.0, 0.0));
            x_pos.push(Vec3::new( half_size, 0.0, 0.0));
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
            let y_color = kiss3d::color::Color::new(126.0 / 255.0, 194.0 / 255.0, 18.0 / 255.0, 1.0);
            let mut y_pos = Vec::new();
            let mut y_ind = Vec::new();
            y_pos.push(Vec3::new(0.0, -half_size, 0.0));
            y_pos.push(Vec3::new(0.0,  half_size, 0.0));
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
            let z_color = kiss3d::color::Color::new(47.0 / 255.0, 131.0 / 255.0, 227.0 / 255.0, 1.0);
            let mut z_pos = Vec::new();
            let mut z_ind = Vec::new();
            z_pos.push(Vec3::new(0.0, 0.0, -half_size));
            z_pos.push(Vec3::new(0.0, 0.0,  half_size));
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

    fn trigger_file_dialog(
        tx: std::sync::mpsc::Sender<Result<MeshData, String>>,
        registry: std::sync::Arc<loader::LoaderRegistry>,
        camera: &mut FixedCenterCamera,
    ) {
        if let Some(p) = FileDialog::new()
            .add_filter("3D Models (*.stl, *.3mf, *.obj, *.gltf, *.glb)", &["stl", "3mf", "obj", "gltf", "glb"])
            .pick_file()
        {
            let txc = tx.clone();
            let reg = registry.clone();
            std::thread::spawn(move || {
                match reg.load_path(&p) {
                    Ok(m) => { let _ = txc.send(Ok(m)); }
                    Err(e) => {
                        eprintln!("Failed to load mesh {}: {}", p.display(), e);
                        let _ = txc.send(Err(format!("Could not open '{}'. The format may be unsupported or the file may be corrupted.", p.display())));
                    }
                }
            });
            if let Some(obj) = camera.object_mut() {
                obj.set_local_scale(0.0, 0.0, 0.0);
            }
            camera.set_loading(true);
        }
    }

    struct FixedCenterCamera {
        inner: OrbitCamera3d,
        center: Vec3,
        dist_step: f32,
        object: Option<kiss3d::scene::SceneNode3d>,
        light_node: kiss3d::scene::SceneNode3d,
        last_cursor: Option<(f32, f32)>,
        dragging: bool,
        last_click: Option<std::time::Instant>,
        loader: std::sync::Arc<loader::LoaderRegistry>,
        tx: std::sync::mpsc::Sender<Result<MeshData, String>>,
        loading: bool,
        should_close: bool,
        smooth_orbit: bool,
        vel_yaw: f32,
        vel_pitch: f32,
        pub pending_menu_pos: Option<(f32, f32)>,
        pub menu_open: bool,
    }

    impl FixedCenterCamera {
        fn new(
            inner: OrbitCamera3d,
            center: Vec3,
            dist_step: f32,
            light_node: kiss3d::scene::SceneNode3d,
            tx: std::sync::mpsc::Sender<Result<MeshData, String>>,
            loader: std::sync::Arc<loader::LoaderRegistry>,
            smooth_orbit: bool,
        ) -> Self {
            Self {
                inner,
                center,
                dist_step,
                object: None,
                light_node,
                last_cursor: None,
                dragging: false,
                last_click: None,
                loader,
                tx,
                loading: false,
                should_close: false,
                smooth_orbit,
                vel_yaw: 0.0,
                vel_pitch: 0.0,
                pending_menu_pos: None,
                menu_open: false,
            }
        }

        fn set_object(&mut self, obj: kiss3d::scene::SceneNode3d) {
            if let Some(mut old) = self.object.take() {
                old.set_local_scale(0.0, 0.0, 0.0);
                old.set_visible(false);
            }
            self.object = Some(obj);
        }

        fn object_mut(&mut self) -> Option<&mut kiss3d::scene::SceneNode3d> {
            self.object.as_mut()
        }

        fn set_loading(&mut self, v: bool) {
            self.loading = v;
            if v {
                if let Some(obj) = &mut self.object {
                    obj.set_local_scale(0.0, 0.0, 0.0);
                }
            }
        }

        fn is_loading(&self) -> bool {
            self.loading
        }

        fn reset_view(&mut self, eye: Vec3, center: Vec3) {
            self.inner.look_at(eye, center);
            self.inner.set_at(center);
            self.vel_yaw = 0.0;
            self.vel_pitch = 0.0;
        }
    }

    use kiss3d::camera::Camera3d;
    use kiss3d::window::Canvas;
    use kiss3d::event::WindowEvent;

    impl Camera3d for FixedCenterCamera {
        fn clip_planes(&self) -> (f32, f32) { self.inner.clip_planes() }
        fn view_transform(&self) -> kiss3d::glamx::Pose3 { self.inner.view_transform() }
        fn eye(&self) -> Vec3 { self.inner.eye() }

        fn handle_event(&mut self, canvas: &Canvas, event: &WindowEvent) {
            use kiss3d::event::WindowEvent::*;
            use kiss3d::event::{MouseButton, Action};

            match event {
                Scroll(_, off, _) => {
                    let offf = *off as f32;
                    let new_dist = (self.inner.dist() * self.dist_step.powf(offf)).clamp(self.inner.min_dist(), self.inner.max_dist());
                    self.inner.set_dist(new_dist);
                    self.inner.set_at(self.center);
                }
                WindowEvent::FramebufferSize(_w, _h) => {
                    self.inner.handle_event(canvas, event);
                }
                CursorPos(x, y, _) => {
                    let x = *x as f32;
                    let y = *y as f32;
                    if self.dragging {
                        if let Some((lx, ly)) = self.last_cursor {
                            let dx = x - lx;
                            let dy = y - ly;

                            let new_yaw = self.inner.yaw() + dx * 0.01;
                            let new_pitch = (self.inner.pitch() - dy * 0.01)
                                .clamp(0.01, std::f32::consts::PI - 0.01);
                            self.inner.set_yaw(new_yaw);
                            self.inner.set_pitch(new_pitch);
                            self.inner.set_at(self.center);

                            if self.smooth_orbit {
                                self.vel_yaw = self.vel_yaw * 0.4 + dx * 0.006;
                                self.vel_pitch = self.vel_pitch * 0.4 - dy * 0.006;
                            }
                        }
                        self.last_cursor = Some((x, y));
                    } else {
                        self.last_cursor = Some((x, y));
                    }
                }
                MouseButton(btn, act, _) => {
                    if *btn == MouseButton::Button1 {
                        if *act == Action::Press {
                            if self.menu_open {
                                self.menu_open = false;
                                return;
                            }
                            self.vel_yaw = 0.0;
                            self.vel_pitch = 0.0;
                            let now = std::time::Instant::now();
                            let mut double = false;
                            if let Some(last) = self.last_click {
                                if now.duration_since(last) <= std::time::Duration::from_millis(300) {
                                    double = true;
                                }
                            }

                            if double {
                                self.last_click = None;
                                trigger_file_dialog(self.tx.clone(), self.loader.clone(), self);
                            } else {
                                self.last_click = Some(now);
                                self.dragging = true;
                            }
                        } else {
                            self.dragging = false;
                            if !self.dragging {
                                self.last_cursor = None;
                            }
                        }
                    } else if *btn == MouseButton::Button2 && *act == Action::Press {
                        if let Some((x, y)) = self.last_cursor {
                            let scale = canvas.scale_factor() as f32;
                            self.pending_menu_pos = Some((x / scale, y / scale));
                        }
                    }
                }
                Key(k, act, _) => {
                    use kiss3d::event::Key as K;
                    if *act == Action::Press && *k == K::Escape {
                        if self.menu_open {
                            self.menu_open = false;
                        } else {
                            self.should_close = true;
                        }
                    } else {
                        self.inner.handle_event(canvas, event);
                    }
                }
                _ => {
                    self.inner.handle_event(canvas, event);
                }
            }
        }

        fn view_transform_pair(&self, pass: usize) -> (kiss3d::glamx::Pose3, kiss3d::glamx::Mat4) {
            self.inner.view_transform_pair(pass)
        }

        fn render_layers(&self) -> u32 { self.inner.render_layers() }
        fn transformation(&self) -> kiss3d::glamx::Mat4 { self.inner.transformation() }
        fn inverse_transformation(&self) -> kiss3d::glamx::Mat4 { self.inner.inverse_transformation() }
        fn update(&mut self, canvas: &Canvas) {
            if self.smooth_orbit {
                if self.dragging {
                    self.vel_yaw *= 0.85;
                    self.vel_pitch *= 0.85;
                } else if self.vel_yaw.abs() > 1e-5 || self.vel_pitch.abs() > 1e-5 {
                    let new_yaw = self.inner.yaw() + self.vel_yaw;
                    let unconstrained_pitch = self.inner.pitch() + self.vel_pitch;
                    let new_pitch = unconstrained_pitch.clamp(0.01, std::f32::consts::PI - 0.01);
                    if (unconstrained_pitch - new_pitch).abs() > 1e-5 {
                        self.vel_pitch = 0.0;
                    }
                    self.inner.set_yaw(new_yaw);
                    self.inner.set_pitch(new_pitch);
                    self.inner.set_at(self.center);

                    self.vel_yaw *= 0.92;
                    self.vel_pitch *= 0.92;
                } else {
                    self.vel_yaw = 0.0;
                    self.vel_pitch = 0.0;
                }
            }
            self.inner.update(canvas);
            let eye = self.inner.eye();
            self.inner.look_at(eye, self.center);
            self.light_node.set_position(eye);
        }
    }

    pub fn run(
        rx: std::sync::mpsc::Receiver<Result<MeshData, String>>,
        tx: std::sync::mpsc::Sender<Result<MeshData, String>>,
        registry: std::sync::Arc<loader::LoaderRegistry>,
        mut cfg: Config,
        initial_loading: bool,
    ) {
        use std::sync::mpsc::TryRecvError;

        let mut window = if initial_loading {
            pollster::block_on(Window::new("Mesh - Loading..."))
        } else {
            pollster::block_on(Window::new("Mesh - Double-click or Right-click for Menu"))
        };
        window.set_background_color(kiss3d::color::Color::new(
            cfg.background[0] as f32 / 255.0,
            cfg.background[1] as f32 / 255.0,
            cfg.background[2] as f32 / 255.0,
            1.0,
        ));

        let mut scene_root = kiss3d::scene::SceneNode3d::empty();

        let mut grid_node: Option<kiss3d::scene::SceneNode3d> = if cfg.show_grid {
            let grid_color = kiss3d::color::Color::new(
                cfg.grid_color[0] as f32 / 255.0,
                cfg.grid_color[1] as f32 / 255.0,
                cfg.grid_color[2] as f32 / 255.0,
                1.0,
            );
            Some(build_grid(&mut scene_root, cfg.grid_size, cfg.grid_divisions, grid_color, cfg.show_axes))
        } else {
            None
        };

        let mut axes_node: Option<kiss3d::scene::SceneNode3d> = Some(build_axes(
            &mut scene_root,
            cfg.grid_size,
            cfg.show_axes,
            cfg.show_axis_direction,
        ));

        let scaled_light_radius = cfg.object_scale * 5.0;

        let node_color = kiss3d::color::Color::new(
            cfg.object_color[0] as f32 / 255.0,
            cfg.object_color[1] as f32 / 255.0,
            cfg.object_color[2] as f32 / 255.0,
            1.0,
        );

        let center: Vec3 = Vec3::new(0.0, 0.0, 0.0);

        let mut placeholder = scene_root.add_cube(0.5, 0.5, 0.5);
        placeholder.set_color(node_color);

        let eye = Vec3::new(cfg.camera_eye[0], cfg.camera_eye[1], cfg.camera_eye[2]);
        let mut base_camera = OrbitCamera3d::new(eye, center);
        base_camera.rebind_drag_button(None);
        base_camera.rebind_reset_key(None);
        base_camera.set_min_dist(cfg.scroll_min);
        base_camera.set_max_dist(cfg.scroll_max);
        base_camera.set_up_axis(Vec3::new(0.0, 0.0, 1.0));

        let light_node = {
            let light = kiss3d::light::Light::point(scaled_light_radius * 2.0);
            scene_root.add_light(light)
        };

        let dist_step_value: f32 = 1.0 + (cfg.scroll_speed * if cfg.invert_scroll { 1.0 } else { -1.0 });
        base_camera.set_dist_step(dist_step_value);

        let mut camera = FixedCenterCamera::new(base_camera, center, dist_step_value, light_node, tx.clone(), registry.clone(), cfg.smooth_orbit);
        if initial_loading {
            placeholder.set_local_scale(0.0, 0.0, 0.0);
        }
        camera.set_object(placeholder);
        camera.set_loading(initial_loading);

        let mut last_was_loading = false;
        let mut base_scale_factor: f32 = 1.0;
        let mut menu_pos = egui::pos2(120.0, 120.0);

        while pollster::block_on(window.render_3d(&mut scene_root, &mut camera)) {
            if camera.should_close {
                break;
            }
            let now_loading = camera.is_loading();
            if now_loading != last_was_loading {
                if now_loading {
                    window.set_title("Mesh - Loading...");
                } else {
                    window.set_title("Mesh - Double-click or Right-click for Menu");
                }
                last_was_loading = now_loading;
            }

            if let Some((mx, my)) = camera.pending_menu_pos.take() {
                menu_pos = egui::pos2(mx, my);
                camera.menu_open = true;
            }

            let mut open_file_dialog = false;
            let mut reset_camera_requested = false;
            let mut reset_defaults_requested = false;
            let mut close_app_requested = false;
            let mut config_changed = false;
            let mut bg_changed = false;
            let mut obj_color_changed = false;
            let mut scale_changed = false;
            let mut grid_rebuild = false;
            let mut axes_rebuild = false;
            let mut controls_changed = false;

            window.draw_ui(|ctx| {
                if camera.menu_open {
                    let area_resp = egui::Area::new(egui::Id::new("context_menu"))
                        .order(egui::Order::Foreground)
                        .fixed_pos(menu_pos)
                        .constrain(true)
                        .default_width(ctx.global_style().spacing.menu_width)
                        .show(ctx, |ui| {
                            egui::Frame::menu(&ctx.global_style())
                                .shadow(egui::Shadow::NONE)
                                .show(ui, |ui| {
                                    ui.spacing_mut().slider_width = 80.0;

                                    if ui.add(egui::Button::new("Load 3D Model").frame(false)).clicked() {
                                        open_file_dialog = true;
                                        camera.menu_open = false;
                                    }

                                    ui.separator();

                                    ui.collapsing("Grid", |ui| {
                                        if ui.checkbox(&mut cfg.show_grid, "Show Grid").changed() {
                                            config_changed = true;
                                            grid_rebuild = true;
                                        }
                                        if cfg.show_grid {
                                            if ui.add(egui::Slider::new(&mut cfg.grid_size, 0.5..=20.0).text("Size")).changed() {
                                                config_changed = true;
                                                grid_rebuild = true;
                                            }
                                            if ui.add(egui::Slider::new(&mut cfg.grid_divisions, 2..=50).text("Divisions")).changed() {
                                                config_changed = true;
                                                grid_rebuild = true;
                                            }
                                            ui.horizontal(|ui| {
                                                ui.label("Grid Color:");
                                                if ui.color_edit_button_srgb(&mut cfg.grid_color).changed() {
                                                    config_changed = true;
                                                    grid_rebuild = true;
                                                }
                                            });
                                        }
                                    });

                                    ui.collapsing("Coordinate Axes", |ui| {
                                        ui.label("Axes Visibility:");
                                        ui.horizontal(|ui| {
                                            if ui.checkbox(&mut cfg.show_axes[0], "X (Red)").changed() {
                                                config_changed = true;
                                                axes_rebuild = true;
                                            }
                                            if ui.checkbox(&mut cfg.show_axes[1], "Y (Green)").changed() {
                                                config_changed = true;
                                                axes_rebuild = true;
                                            }
                                            if ui.checkbox(&mut cfg.show_axes[2], "Z (Blue)").changed() {
                                                config_changed = true;
                                                axes_rebuild = true;
                                            }
                                        });
                                        if ui.checkbox(&mut cfg.show_axis_direction, "Axis Direction Arrows").changed() {
                                            config_changed = true;
                                            axes_rebuild = true;
                                        }
                                    });

                                    ui.collapsing("Colors", |ui| {
                                        ui.horizontal(|ui| {
                                            ui.label("Background:");
                                            if ui.color_edit_button_srgb(&mut cfg.background).changed() {
                                                config_changed = true;
                                                bg_changed = true;
                                            }
                                        });
                                        ui.horizontal(|ui| {
                                            ui.label("Object:");
                                            if ui.color_edit_button_srgb(&mut cfg.object_color).changed() {
                                                config_changed = true;
                                                obj_color_changed = true;
                                            }
                                        });
                                    });

                                    ui.collapsing("Controls & Scale", |ui| {
                                        if ui.add(egui::Slider::new(&mut cfg.object_scale, 0.1..=10.0).text("Scale")).changed() {
                                            config_changed = true;
                                            scale_changed = true;
                                        }
                                        if ui.checkbox(&mut cfg.smooth_orbit, "Smooth Orbit").changed() {
                                            config_changed = true;
                                            controls_changed = true;
                                        }
                                        if ui.checkbox(&mut cfg.invert_scroll, "Invert Zoom").changed() {
                                            config_changed = true;
                                            controls_changed = true;
                                        }
                                        if ui.add(egui::Slider::new(&mut cfg.scroll_speed, 0.001..=0.05).text("Zoom Speed")).changed() {
                                            config_changed = true;
                                            controls_changed = true;
                                        }
                                    });

                                    ui.separator();

                                    if ui.add(egui::Button::new("Reset Camera").frame(false)).clicked() {
                                        reset_camera_requested = true;
                                        camera.menu_open = false;
                                    }

                                    ui.separator();

                                    if ui.add(egui::Button::new("Reset Defaults").frame(false)).clicked() {
                                        reset_defaults_requested = true;
                                    }

                                    if ui.add(egui::Button::new("Exit").frame(false)).clicked() {
                                        close_app_requested = true;
                                    }
                                });
                        });

                    if ctx.input(|i| i.pointer.button_pressed(egui::PointerButton::Primary)) {
                        if let Some(pos) = ctx.input(|i| i.pointer.interact_pos()) {
                            if !area_resp.response.rect.contains(pos) {
                                camera.menu_open = false;
                            }
                        }
                    }
                }
            });

            if open_file_dialog {
                trigger_file_dialog(tx.clone(), registry.clone(), &mut camera);
            }

            if reset_camera_requested {
                camera.reset_view(Vec3::new(cfg.camera_eye[0], cfg.camera_eye[1], cfg.camera_eye[2]), center);
            }

            if reset_defaults_requested {
                cfg = Config::default();
                let _ = cfg.save();
                bg_changed = true;
                obj_color_changed = true;
                scale_changed = true;
                grid_rebuild = true;
                axes_rebuild = true;
                controls_changed = true;
            }

            if close_app_requested {
                camera.should_close = true;
            }

            if config_changed {
                if let Err(e) = cfg.save() {
                    eprintln!("Failed to save config: {}", e);
                }
            }

            if bg_changed {
                window.set_background_color(kiss3d::color::Color::new(
                    cfg.background[0] as f32 / 255.0,
                    cfg.background[1] as f32 / 255.0,
                    cfg.background[2] as f32 / 255.0,
                    1.0,
                ));
            }

            if obj_color_changed {
                let c = kiss3d::color::Color::new(
                    cfg.object_color[0] as f32 / 255.0,
                    cfg.object_color[1] as f32 / 255.0,
                    cfg.object_color[2] as f32 / 255.0,
                    1.0,
                );
                if let Some(obj) = camera.object_mut() {
                    obj.set_color(c);
                }
            }

            if scale_changed {
                let actual_scale = cfg.object_scale * base_scale_factor;
                if let Some(obj) = camera.object_mut() {
                    obj.set_local_scale(actual_scale, actual_scale, actual_scale);
                }
            }

            if grid_rebuild || axes_rebuild {
                if let Some(mut old) = grid_node.take() {
                    old.set_visible(false);
                }
                if let Some(mut old) = axes_node.take() {
                    old.set_visible(false);
                }
                if cfg.show_grid {
                    let grid_color = kiss3d::color::Color::new(
                        cfg.grid_color[0] as f32 / 255.0,
                        cfg.grid_color[1] as f32 / 255.0,
                        cfg.grid_color[2] as f32 / 255.0,
                        1.0,
                    );
                    grid_node = Some(build_grid(&mut scene_root, cfg.grid_size, cfg.grid_divisions, grid_color, cfg.show_axes));
                }
                axes_node = Some(build_axes(&mut scene_root, cfg.grid_size, cfg.show_axes, cfg.show_axis_direction));
            }

            if controls_changed {
                camera.smooth_orbit = cfg.smooth_orbit;
                let dist_step_value = 1.0 + (cfg.scroll_speed * if cfg.invert_scroll { 1.0 } else { -1.0 });
                camera.dist_step = dist_step_value;
                camera.inner.set_dist_step(dist_step_value);
            }

            match rx.try_recv() {
                Ok(Ok(mesh)) => {
                    let mut min = Vec3::new(f32::INFINITY, f32::INFINITY, f32::INFINITY);
                    let mut max = Vec3::new(f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY);
                    for p in &mesh.positions {
                        min.x = min.x.min(p[0]);
                        min.y = min.y.min(p[1]);
                        min.z = min.z.min(p[2]);
                        max.x = max.x.max(p[0]);
                        max.y = max.y.max(p[1]);
                        max.z = max.z.max(p[2]);
                    }

                    let center_offset = (min + max) / 2.0;

                    let verts_glam: Vec<Vec3> = mesh
                        .positions
                        .iter()
                        .map(|p| Vec3::new(p[0], p[1], p[2]) - center_offset)
                        .collect();

                    let tris: Vec<[u32; 3]> = mesh.indices.clone();

                    let mut node = scene_root.add_trimesh(verts_glam, tris, Vec3::new(1.0, 1.0, 1.0), false);
                    let current_color = kiss3d::color::Color::new(
                        cfg.object_color[0] as f32 / 255.0,
                        cfg.object_color[1] as f32 / 255.0,
                        cfg.object_color[2] as f32 / 255.0,
                        1.0,
                    );
                    node.set_color(current_color);

                    let size = (max - min).abs();
                    let max_dim = size.x.max(size.y).max(size.z).max(1e-6);
                    base_scale_factor = 1.0 / max_dim;
                    let scale_factor = cfg.object_scale * base_scale_factor;
                    node.set_local_scale(scale_factor, scale_factor, scale_factor);

                    camera.set_object(node);
                    camera.set_loading(false);
                }
                Ok(Err(err)) => {
                    eprintln!("{}", err);
                    camera.set_loading(false);
                    window.set_title("Mesh - Double-click or Right-click for Menu");
                }
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => {}
            }
        }
    }
}
