mod loader;
mod config;

use rfd::FileDialog;
use std::path::PathBuf;
use loader::{LoaderRegistry, MeshData};
use config::Config;

fn main() {
    let arg_path: Option<PathBuf> = std::env::args().nth(1).map(PathBuf::from);

    let cfg: Config = match Config::load_or_create() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to load/create config: {}. Using defaults.", e);
            Config::default()
        }
    };

    let mut registry = LoaderRegistry::new();
    registry.register(Box::new(loader::stl::StlLoader {}));
    registry.register(Box::new(loader::obj::ObjLoader {}));
    registry.register(Box::new(loader::gltf::GltfLoader {}));

    let (mesh, load_error) = if let Some(p) = arg_path {
        match registry.load_path(&p) {
            Ok(m) => (Some(m), None),
            Err(e) => {
                eprintln!("Failed to load mesh {}: {}", p.display(), e);
                let friendly = format!("Could not open '{}'. The format may be unsupported or the file is corrupted. Showing default cube.", p.display());
                (None, Some(friendly))
            }
        }
    } else {
        let file = FileDialog::new()
            .add_filter("3D Models", &["stl", "obj", "gltf", "glb"])
            .pick_file();

        if let Some(p) = file {
            match registry.load_path(&p) {
                Ok(m) => (Some(m), None),
                Err(e) => {
                    eprintln!("Failed to load mesh {}: {}", p.display(), e);
                    let friendly = format!("Could not open '{}'. The format may be unsupported or the file is corrupted. Showing default cube.", p.display());
                    (None, Some(friendly))
                }
            }
        } else {
            (None, Some("No file provided. Showing default cube.".into()))
        }
    };

    start_viewer(mesh, cfg, load_error);
}

fn start_viewer(mesh: Option<MeshData>, cfg: Config, load_error: Option<String>) {
    render::run(mesh, cfg, load_error)
}

mod render {
    use super::*;
    use kiss3d::window::Window;
    use kiss3d::camera::OrbitCamera3d;
    use kiss3d::glamx::Vec3;
    use pollster;

    pub fn run(mesh: Option<MeshData>, cfg: Config, load_error: Option<String>) {
        
        let mut window = pollster::block_on(Window::new("Mesh View"));
        window.set_background_color(kiss3d::color::Color::new(
            cfg.background[0] as f32 / 255.0,
            cfg.background[1] as f32 / 255.0,
            cfg.background[2] as f32 / 255.0,
            1.0,
        ));

        let mut scene_root = kiss3d::scene::SceneNode3d::empty();
        
        let scaled_light_position = cfg.object_scale * 2.0;
        let scaled_light_radius = cfg.object_scale * 5.0;
        let light_top_back = kiss3d::light::Light::point(scaled_light_radius / 2.0);
        let light_bottom_back = kiss3d::light::Light::point(scaled_light_radius / 2.0);
        let light_left = kiss3d::light::Light::point(scaled_light_radius);
        let light_right = kiss3d::light::Light::point(scaled_light_radius);
        scene_root.add_light(light_top_back).set_position(Vec3::new(0.0, scaled_light_position/2.0, -scaled_light_position));
        scene_root.add_light(light_bottom_back).set_position(Vec3::new(0.0, -scaled_light_position/2.0, -scaled_light_position));
        scene_root.add_light(light_left).set_position(Vec3::new(-scaled_light_position, scaled_light_position, scaled_light_position));
        scene_root.add_light(light_right).set_position(Vec3::new(scaled_light_position, scaled_light_position, scaled_light_position));

        let node_color = kiss3d::color::Color::new(
            cfg.object_color[0] as f32 / 255.0,
            cfg.object_color[1] as f32 / 255.0,
            cfg.object_color[2] as f32 / 255.0,
            1.0,
        );

        let center: Vec3;

        if let Some(mesh) = mesh {
            let verts_glam: Vec<Vec3> = mesh
                .positions
                .iter()
                .map(|p| Vec3::new(p[0], p[1], p[2]))
                .collect();

            let tris: Vec<[u32; 3]> = mesh.indices.clone();

            let mut node = scene_root.add_trimesh(verts_glam, tris, Vec3::new(1.0, 1.0, 1.0), false);

            node.set_color(node_color);

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

            let size = (max - min).abs();
            let max_dim = size.x.max(size.y).max(size.z).max(1e-6);
            let scale_factor = cfg.object_scale / max_dim;
            let scaled_size = Vec3::clone(&size) * scale_factor;
            center = Vec3::new(0.0, scaled_size.y / 2.0, scaled_size.z / 2.0);

            node.set_local_scale(scale_factor, scale_factor, scale_factor);
        } else {
            let mut cube = scene_root.add_cube(0.5, 0.5, 0.5);
            cube.set_color(node_color);
            center = Vec3::new(0.0, 0.0, 0.0);
        }

        let eye = Vec3::new(cfg.camera_eye[0], cfg.camera_eye[1], cfg.camera_eye[2]);
        let mut base_camera = OrbitCamera3d::new(eye, center);
        base_camera.rebind_drag_button(None);
        base_camera.rebind_reset_key(None);

        let dist_step_value: f32 = 1.0 + (cfg.scroll_speed * if cfg.invert_scroll { 1.0 } else { -1.0 });
        base_camera.set_dist_step(dist_step_value);

        struct FixedCenterCamera {
            inner: OrbitCamera3d,
            center: Vec3,
            dist_step: f32,
        }

        impl FixedCenterCamera {
            fn new(inner: OrbitCamera3d, center: Vec3, dist_step: f32) -> Self {
                Self { inner, center, dist_step }
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
                match event {
                    Scroll(_, off, _) => {
                        let offf = *off as f32;
                        let new_dist = (self.inner.dist() * self.dist_step.powf(offf)).clamp(self.inner.min_dist(), self.inner.max_dist());
                        self.inner.set_dist(new_dist);
                        self.inner.set_at(self.center);
                    }
                    _ => {
                        self.inner.handle_event(canvas, event);
                        self.inner.set_at(self.center);
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
            }
        }

        let mut camera = FixedCenterCamera::new(base_camera, center, dist_step_value);

        if let Some(err) = load_error {
            eprintln!("{}", err);
            println!("Error: {}", err);
        }

        while pollster::block_on(window.render_3d(&mut scene_root, &mut camera)) {}
    }
}
