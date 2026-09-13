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
    ) {
        let n = divisions + 1;
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

        let _ = n;

        let mut grid_node = scene_root.add_trimesh(
            positions,
            indices,
            Vec3::new(1.0, 1.0, 1.0),
            false,
        );
        grid_node.set_surface_rendering_activation(false);
        grid_node.set_lines_color(Some(color));
        grid_node.set_lines_width(1.0, false);
    }

    fn build_axes(
        scene_root: &mut kiss3d::scene::SceneNode3d,
        half_size: f32,
        show_axes: [bool; 3],
        show_axis_direction: bool,
    ) {
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
            let mut x_node = scene_root.add_trimesh(x_pos, x_ind, Vec3::new(1.0, 1.0, 1.0), false);
            x_node.set_surface_rendering_activation(false);
            x_node.set_lines_color(Some(x_color));
            x_node.set_lines_width(axis_width, false);

            if show_axis_direction {
                let rot_x = Quat::from_axis_angle(Vec3::Z, -std::f32::consts::FRAC_PI_2);
                let mut cone = scene_root.add_cone(cone_r, cone_h);
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
            let mut y_node = scene_root.add_trimesh(y_pos, y_ind, Vec3::new(1.0, 1.0, 1.0), false);
            y_node.set_surface_rendering_activation(false);
            y_node.set_lines_color(Some(y_color));
            y_node.set_lines_width(axis_width, false);

            if show_axis_direction {
                let mut cone = scene_root.add_cone(cone_r, cone_h);
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
            let mut z_node = scene_root.add_trimesh(z_pos, z_ind, Vec3::new(1.0, 1.0, 1.0), false);
            z_node.set_surface_rendering_activation(false);
            z_node.set_lines_color(Some(z_color));
            z_node.set_lines_width(axis_width, false);

            if show_axis_direction {
                let rot_z = Quat::from_axis_angle(Vec3::X, std::f32::consts::FRAC_PI_2);
                let mut cone = scene_root.add_cone(cone_r, cone_h);
                cone.set_color(z_color);
                cone.set_pose(Pose3::from_parts(Vec3::new(0.0, 0.0, half_size), rot_z));
            }
        }
    }

    pub fn run(
        rx: std::sync::mpsc::Receiver<Result<MeshData, String>>,
        tx: std::sync::mpsc::Sender<Result<MeshData, String>>,
        registry: std::sync::Arc<loader::LoaderRegistry>,
        cfg: Config,
        initial_loading: bool,
    ) {

        use std::sync::mpsc::TryRecvError;

        let mut window = if initial_loading {
            pollster::block_on(Window::new("Mesh - Loading..."))
        } else {
            pollster::block_on(Window::new("Mesh - Double-click to open"))
        };
        window.set_background_color(kiss3d::color::Color::new(
            cfg.background[0] as f32 / 255.0,
            cfg.background[1] as f32 / 255.0,
            cfg.background[2] as f32 / 255.0,
            1.0,
        ));

        let mut scene_root = kiss3d::scene::SceneNode3d::empty();

        if cfg.show_grid {
            let grid_color = kiss3d::color::Color::new(
                cfg.grid_color[0] as f32 / 255.0,
                cfg.grid_color[1] as f32 / 255.0,
                cfg.grid_color[2] as f32 / 255.0,
                1.0,
            );
            build_grid(&mut scene_root, cfg.grid_size, cfg.grid_divisions, grid_color, cfg.show_axes);
        }

        build_axes(&mut scene_root, cfg.grid_size, cfg.show_axes, cfg.show_axis_direction);

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
        }

        impl FixedCenterCamera {
            fn new(
                inner: OrbitCamera3d,
                center: Vec3,
                dist_step: f32,
                light_node: kiss3d::scene::SceneNode3d,
                tx: std::sync::mpsc::Sender<Result<MeshData, String>>,
                loader: std::sync::Arc<loader::LoaderRegistry>,
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
                }
            }

            fn set_object(&mut self, obj: kiss3d::scene::SceneNode3d) {
                if let Some(mut old) = self.object.take() {
                    old.set_local_scale(0.0, 0.0, 0.0);
                }
                self.object = Some(obj);
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
                            }
                            self.last_cursor = Some((x, y));
                        } else {
                            self.last_cursor = Some((x, y));
                        }
                    }
                    MouseButton(btn, act, _) => {
                        if *btn == MouseButton::Button1 {
                            use std::time::{Instant, Duration};
                            if *act == Action::Press {
                                let now = Instant::now();
                                let mut double = false;
                                if let Some(last) = self.last_click {
                                    if now.duration_since(last) <= Duration::from_millis(300) {
                                        double = true;
                                    }
                                }

                                if double {
                                    self.last_click = None;
                                    if let Some(p) = FileDialog::new()
                                        .add_filter("3D Models", &["stl", "3mf", "obj", "gltf", "glb"]) 
                                        .pick_file()
                                    {
                                        let txc = self.tx.clone();
                                        let reg = self.loader.clone();
                                        std::thread::spawn(move || {
                                            match reg.load_path(&p) {
                                                Ok(m) => { let _ = txc.send(Ok(m)); }
                                                Err(e) => {
                                                    eprintln!("Failed to load mesh {}: {}", p.display(), e);
                                                    let _ = txc.send(Err(format!("Could not open '{}'. The format may be unsupported or the file may be corrupted.", p.display())));
                                                }
                                            }
                                        });
                                        if let Some(obj) = &mut self.object {
                                            obj.set_local_scale(0.0, 0.0, 0.0);
                                        }
                                        self.loading = true;
                                    }
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
                        }
                    }
                    Key(k, act, _) => {
                        use kiss3d::event::Key as K;
                        if *act == Action::Press && *k == K::Escape {
                            self.should_close = true;
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
                self.inner.update(canvas);
                let eye = self.inner.eye();
                self.inner.look_at(eye, self.center);
                self.light_node.set_position(eye);
            }
        }

        let mut camera = FixedCenterCamera::new(base_camera, center, dist_step_value, light_node, tx.clone(), registry.clone());
        if initial_loading {
            placeholder.set_local_scale(0.0, 0.0, 0.0);
        }
        camera.set_object(placeholder);
        camera.set_loading(initial_loading);

        

        let mut last_was_loading = false;

        while pollster::block_on(window.render_3d(&mut scene_root, &mut camera)) {
            if camera.should_close {
                break;
            }
            let now_loading = camera.is_loading();
            if now_loading != last_was_loading {
                if now_loading {
                    window.set_title("Mesh - Loading...");
                } else {
                    window.set_title("Mesh");
                }
                last_was_loading = now_loading;
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
                    node.set_color(node_color);

                    let size = (max - min).abs();
                    let max_dim = size.x.max(size.y).max(size.z).max(1e-6);
                    let scale_factor = cfg.object_scale / max_dim;
                    node.set_local_scale(scale_factor, scale_factor, scale_factor);

                    camera.set_object(node);
                    camera.set_loading(false);
                }
                Ok(Err(err)) => {
                    eprintln!("{}", err);
                    camera.set_loading(false);
                    window.set_title("Mesh - Double-click to open");
                }
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => {}
            }
        }
    }
}
