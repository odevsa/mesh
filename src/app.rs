use crate::camera::FixedCenterCamera;
use crate::config::Config;
use crate::dialog;
use crate::loader::{LoaderRegistry, MeshData};
use crate::scene::{axes, grid, model};
use crate::ui::menu::{render_context_menu, UiResponse};
use crate::ui::overlay;
use kiss3d::camera::OrbitCamera3d;
use kiss3d::color::Color;
use kiss3d::glamx::Vec3;
use kiss3d::scene::SceneNode3d;
use kiss3d::window::Window;
use std::sync::mpsc::{Receiver, Sender, TryRecvError};
use std::sync::Arc;

pub struct App {
    window: Window,
    scene_root: SceneNode3d,
    grid_node: Option<SceneNode3d>,
    axes_node: Option<SceneNode3d>,
    camera: FixedCenterCamera,
    cfg: Config,
    rx: Receiver<Result<MeshData, String>>,
    tx: Sender<Result<MeshData, String>>,
    registry: Arc<LoaderRegistry>,
    base_scale_factor: f32,
    current_model_size: Vec3,
    last_was_loading: bool,
    menu_pos: egui::Pos2,
    has_model: bool,
}

impl App {
    pub fn new(
        rx: Receiver<Result<MeshData, String>>,
        tx: Sender<Result<MeshData, String>>,
        registry: Arc<LoaderRegistry>,
        cfg: Config,
        initial_loading: bool,
    ) -> Self {
        let setup = kiss3d::window::CanvasSetup {
            vsync: true,
            samples: kiss3d::window::NumSamples::One,
            ..Default::default()
        };
        let mut window = if initial_loading {
            pollster::block_on(Window::new_with_setup("Mesh - Loading...", 800, 600, setup.clone()))
        } else {
            pollster::block_on(Window::new_with_setup("Mesh", 800, 600, setup))
        };

        window.set_background_color(Color::new(
            cfg.background[0] as f32 / 255.0,
            cfg.background[1] as f32 / 255.0,
            cfg.background[2] as f32 / 255.0,
            1.0,
        ));

        let mut scene_root = SceneNode3d::empty();

        let grid_node = if cfg.show_grid {
            let grid_color = Color::new(
                cfg.grid_color[0] as f32 / 255.0,
                cfg.grid_color[1] as f32 / 255.0,
                cfg.grid_color[2] as f32 / 255.0,
                1.0,
            );
            Some(grid::build_grid(
                &mut scene_root,
                cfg.grid_size,
                cfg.grid_divisions,
                grid_color,
                cfg.show_axes,
            ))
        } else {
            None
        };

        let axes_node = Some(axes::build_axes(
            &mut scene_root,
            cfg.grid_size,
            cfg.show_axes,
            cfg.show_axis_direction,
        ));

        let scaled_light_radius = cfg.object_scale * 5.0;
        let node_color = Color::new(
            cfg.object_color[0] as f32 / 255.0,
            cfg.object_color[1] as f32 / 255.0,
            cfg.object_color[2] as f32 / 255.0,
            1.0,
        );

        let base_scale_factor = 1.0;
        let current_model_size = Vec3::new(0.5, 0.5, 0.5);

        let initial_z_offset = model::compute_model_z_offset(
            cfg.model_position,
            current_model_size.z,
            cfg.object_scale * base_scale_factor,
        );
        let center = Vec3::new(0.0, 0.0, initial_z_offset);

        let mut placeholder = scene_root.add_cube(0.5, 0.5, 0.5);
        placeholder.set_color(node_color);
        placeholder.set_position(center);

        let eye = center + Vec3::new(cfg.camera_eye[0], cfg.camera_eye[1], cfg.camera_eye[2]);
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

        let dist_step_value = 1.0 + (cfg.scroll_speed * if cfg.invert_scroll { 1.0 } else { -1.0 });
        base_camera.set_dist_step(dist_step_value);

        let mut camera = FixedCenterCamera::new(
            base_camera,
            center,
            dist_step_value,
            light_node,
            cfg.smooth_orbit,
        );

        if initial_loading {
            placeholder.set_local_scale(0.0, 0.0, 0.0);
        }
        if !cfg.show_dummy_box {
            placeholder.set_visible(false);
        }
        camera.set_object(placeholder);
        camera.set_loading(initial_loading);

        Self {
            window,
            scene_root,
            grid_node,
            axes_node,
            camera,
            cfg,
            rx,
            tx,
            registry,
            base_scale_factor,
            current_model_size,
            last_was_loading: false,
            menu_pos: egui::pos2(120.0, 120.0),
            has_model: false,
        }
    }

    pub fn run(mut self) {
        while pollster::block_on(self.window.render_3d(&mut self.scene_root, &mut self.camera)) {
            if self.camera.should_close {
                break;
            }

            self.update_window_title();
            self.update_menu_position();
            self.handle_camera_requests();

            let ui_resp = self.render_ui();
            self.apply_ui_response(ui_resp);

            if self.camera.should_close {
                break;
            }

            self.process_incoming_meshes();
        }
        if let Err(e) = self.cfg.save() {
            eprintln!("Failed to save config: {}", e);
        }
        self.window.hide();
        drop(self);
    }

    fn update_window_title(&mut self) {
        let now_loading = self.camera.is_loading();
        if now_loading != self.last_was_loading {
            if now_loading {
                self.window.set_title("Mesh - Loading...");
            } else {
                self.window.set_title("Mesh");
            }
            self.last_was_loading = now_loading;
        }
    }

    fn update_menu_position(&mut self) {
        if let Some((mx, my)) = self.camera.pending_menu_pos.take() {
            self.menu_pos = egui::pos2(mx, my);
            self.camera.menu_open = true;
        }
    }

    fn handle_camera_requests(&mut self) {
        if self.camera.open_file_requested {
            self.camera.open_file_requested = false;
            self.trigger_file_dialog();
        }
    }

    fn trigger_file_dialog(&mut self) {
        self.camera.dialog_open = true;
        let picked = dialog::trigger_file_dialog(self.registry.clone(), self.tx.clone());
        self.camera.dialog_open = false;
        self.camera.last_dialog_close = Some(std::time::Instant::now());

        if picked {
            if let Some(obj) = self.camera.object_mut() {
                obj.set_local_scale(0.0, 0.0, 0.0);
            }
            self.camera.set_loading(true);
        }
    }

    fn render_ui(&mut self) -> UiResponse {
        let mut ui_resp = UiResponse::default();
        let menu_open = self.camera.menu_open;
        let menu_pos = self.menu_pos;
        let cfg = &mut self.cfg;
        let is_loading = self.camera.is_loading();
        let has_model = self.has_model;

        self.window.draw_ui(|ctx| {
            if is_loading {
                overlay::render_loading_overlay(ctx);
            } else if !has_model && overlay::render_empty_overlay(ctx) {
                ui_resp.open_file_dialog = true;
                ui_resp.close_menu_requested = true;
            }

            if menu_open {
                let menu_resp = render_context_menu(ctx, menu_pos, cfg, has_model);
                let open_dialog = ui_resp.open_file_dialog || menu_resp.open_file_dialog;
                let close_menu = ui_resp.close_menu_requested || menu_resp.close_menu_requested;
                ui_resp = menu_resp;
                ui_resp.open_file_dialog = open_dialog;
                ui_resp.close_menu_requested = close_menu;
            }
        });

        if ui_resp.close_menu_requested {
            self.camera.menu_open = false;
        }

        ui_resp
    }

    fn apply_ui_response(&mut self, resp: UiResponse) {
        if resp.open_file_dialog {
            self.trigger_file_dialog();
        }

        if resp.unload_model_requested {
            self.unload_model();
        }

        if resp.reset_camera_requested {
            self.reset_camera();
        }

        if resp.reset_defaults_requested {
            self.reset_defaults();
        }

        if resp.close_app_requested {
            self.camera.should_close = true;
        }

        if resp.config_changed {
            if let Err(e) = self.cfg.save() {
                eprintln!("Failed to save config: {}", e);
            }
        }

        if resp.bg_changed {
            self.apply_background_color();
        }

        if resp.obj_color_changed {
            self.apply_object_color();
        }

        if resp.scale_changed || resp.position_changed {
            self.update_model_transform(resp.scale_changed);
        }

        if resp.grid_rebuild || resp.axes_rebuild {
            self.rebuild_grid_and_axes();
        }

        if resp.controls_changed {
            self.apply_controls_settings();
        }

        if resp.dummy_box_changed && !self.has_model {
            let is_loading = self.camera.is_loading();
            if let Some(obj) = self.camera.object_mut() {
                if self.cfg.show_dummy_box {
                    let actual_scale = self.cfg.object_scale * self.base_scale_factor;
                    obj.set_local_scale(actual_scale, actual_scale, actual_scale);
                    obj.set_visible(!is_loading);
                } else {
                    obj.set_visible(false);
                }
            }
        }
    }

    fn unload_model(&mut self) {
        self.has_model = false;
        self.base_scale_factor = 1.0;
        self.current_model_size = Vec3::new(0.5, 0.5, 0.5);

        let actual_scale = self.cfg.object_scale * self.base_scale_factor;
        let z_offset = model::compute_model_z_offset(
            self.cfg.model_position,
            self.current_model_size.z,
            actual_scale,
        );
        let center = Vec3::new(0.0, 0.0, z_offset);

        let node_color = Color::new(
            self.cfg.object_color[0] as f32 / 255.0,
            self.cfg.object_color[1] as f32 / 255.0,
            self.cfg.object_color[2] as f32 / 255.0,
            1.0,
        );

        let mut placeholder = self.scene_root.add_cube(0.5, 0.5, 0.5);
        placeholder.set_color(node_color);
        placeholder.set_position(center);
        placeholder.set_local_scale(actual_scale, actual_scale, actual_scale);
        if !self.cfg.show_dummy_box {
            placeholder.set_visible(false);
        }

        self.camera.set_object(placeholder);
        self.camera.set_center(center);
        self.window.set_title("Mesh");
    }

    fn reset_camera(&mut self) {
        let eye_offset = Vec3::new(
            self.cfg.camera_eye[0],
            self.cfg.camera_eye[1],
            self.cfg.camera_eye[2],
        );
        let current_center = self.camera.center();
        self.camera.reset_view(current_center + eye_offset, current_center);
    }

    fn reset_defaults(&mut self) {
        self.cfg = Config::default();
        let _ = self.cfg.save();
        self.apply_background_color();
        self.apply_object_color();
        self.update_model_transform(true);
        self.rebuild_grid_and_axes();
        self.apply_controls_settings();
        if !self.has_model {
            let is_loading = self.camera.is_loading();
            if let Some(obj) = self.camera.object_mut() {
                if self.cfg.show_dummy_box {
                    let actual_scale = self.cfg.object_scale * self.base_scale_factor;
                    obj.set_local_scale(actual_scale, actual_scale, actual_scale);
                    obj.set_visible(!is_loading);
                } else {
                    obj.set_visible(false);
                }
            }
        }
    }

    fn apply_background_color(&mut self) {
        self.window.set_background_color(Color::new(
            self.cfg.background[0] as f32 / 255.0,
            self.cfg.background[1] as f32 / 255.0,
            self.cfg.background[2] as f32 / 255.0,
            1.0,
        ));
    }

    fn apply_object_color(&mut self) {
        let c = Color::new(
            self.cfg.object_color[0] as f32 / 255.0,
            self.cfg.object_color[1] as f32 / 255.0,
            self.cfg.object_color[2] as f32 / 255.0,
            1.0,
        );
        if let Some(obj) = self.camera.object_mut() {
            obj.set_color(c);
        }
    }

    fn update_model_transform(&mut self, scale_changed: bool) {
        let actual_scale = self.cfg.object_scale * self.base_scale_factor;
        let z_offset = model::compute_model_z_offset(
            self.cfg.model_position,
            self.current_model_size.z,
            actual_scale,
        );
        let new_center = Vec3::new(0.0, 0.0, z_offset);
        if let Some(obj) = self.camera.object_mut() {
            if scale_changed {
                obj.set_local_scale(actual_scale, actual_scale, actual_scale);
            }
            obj.set_position(new_center);
        }
        self.camera.set_center(new_center);
    }

    fn rebuild_grid_and_axes(&mut self) {
        if let Some(mut old) = self.grid_node.take() {
            old.set_visible(false);
        }
        if let Some(mut old) = self.axes_node.take() {
            old.set_visible(false);
        }

        if self.cfg.show_grid {
            let grid_color = Color::new(
                self.cfg.grid_color[0] as f32 / 255.0,
                self.cfg.grid_color[1] as f32 / 255.0,
                self.cfg.grid_color[2] as f32 / 255.0,
                1.0,
            );
            self.grid_node = Some(grid::build_grid(
                &mut self.scene_root,
                self.cfg.grid_size,
                self.cfg.grid_divisions,
                grid_color,
                self.cfg.show_axes,
            ));
        }

        self.axes_node = Some(axes::build_axes(
            &mut self.scene_root,
            self.cfg.grid_size,
            self.cfg.show_axes,
            self.cfg.show_axis_direction,
        ));
    }

    fn apply_controls_settings(&mut self) {
        self.camera.smooth_orbit = self.cfg.smooth_orbit;
        let dist_step_value = 1.0
            + (self.cfg.scroll_speed * if self.cfg.invert_scroll { 1.0 } else { -1.0 });
        self.camera.dist_step = dist_step_value;
        self.camera.inner.set_dist_step(dist_step_value);
    }

    fn process_incoming_meshes(&mut self) {
        match self.rx.try_recv() {
            Ok(Ok(mesh)) => {
                let (min, max) = model::compute_bounds(&mesh.positions);
                let center_offset = (min + max) / 2.0;
                let verts_glam = model::center_vertices(&mesh.positions, center_offset);
                let tris = mesh.indices.clone();

                let mut node = self
                    .scene_root
                    .add_trimesh(verts_glam, tris, Vec3::new(1.0, 1.0, 1.0), false);

                let current_color = Color::new(
                    self.cfg.object_color[0] as f32 / 255.0,
                    self.cfg.object_color[1] as f32 / 255.0,
                    self.cfg.object_color[2] as f32 / 255.0,
                    1.0,
                );
                node.set_color(current_color);

                let size = (max - min).abs();
                self.current_model_size = size;
                let max_dim = size.x.max(size.y).max(size.z).max(1e-6);
                self.base_scale_factor = 1.0 / max_dim;
                let scale_factor = self.cfg.object_scale * self.base_scale_factor;
                node.set_local_scale(scale_factor, scale_factor, scale_factor);

                let z_offset = model::compute_model_z_offset(
                    self.cfg.model_position,
                    self.current_model_size.z,
                    scale_factor,
                );
                let model_center = Vec3::new(0.0, 0.0, z_offset);
                node.set_position(model_center);

                self.camera.set_object(node);
                self.camera.set_loading(false);
                self.camera.set_center(model_center);
                self.has_model = true;
            }
            Ok(Err(err)) => {
                eprintln!("{}", err);
                self.camera.set_loading(false);
                self.window.set_title("Mesh");
                if !self.has_model {
                    if let Some(obj) = self.camera.object_mut() {
                        if self.cfg.show_dummy_box {
                            let actual_scale = self.cfg.object_scale * self.base_scale_factor;
                            obj.set_local_scale(actual_scale, actual_scale, actual_scale);
                            obj.set_visible(true);
                        } else {
                            obj.set_visible(false);
                        }
                    }
                }
            }
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => {}
        }
    }
}

