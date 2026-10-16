use crate::camera::FixedCenterCamera;
use crate::config::Config;
use crate::dialog;
use crate::loader::{self, LoaderRegistry, MeshData};
use crate::render::{SceneRenderer, Uniforms};
use crate::scene::model;
use crate::ui::menu::{render_context_menu, UiResponse};
use crate::ui::overlay;
use glam::Vec3;
use std::sync::mpsc::{Receiver, Sender, TryRecvError};
use std::sync::Arc;

pub struct App {
    renderer: Option<SceneRenderer>,
    wgpu_render_state: Option<egui_wgpu::RenderState>,
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
    current_mesh_data: Option<MeshData>,
    current_model_stats: Option<overlay::ModelStats>,
    show_about: bool,
}

impl App {
    pub fn new(
        cc: &eframe::CreationContext,
        rx: Receiver<Result<MeshData, String>>,
        tx: Sender<Result<MeshData, String>>,
        registry: Arc<LoaderRegistry>,
        cfg: Config,
        initial_loading: bool,
    ) -> Self {
        crate::i18n::setup_cjk_fonts(&cc.egui_ctx);

        let wgpu_render_state = cc.wgpu_render_state.clone();
        let renderer = if let Some(ref state) = wgpu_render_state {
            let mut r = SceneRenderer::new(state.device.clone(), state.queue.clone());
            let grid_color = [
                cfg.grid_color[0] as f32 / 255.0,
                cfg.grid_color[1] as f32 / 255.0,
                cfg.grid_color[2] as f32 / 255.0,
                1.0,
            ];
            r.update_grid(cfg.grid_size, cfg.grid_divisions, grid_color, cfg.show_axes);
            r.update_axes(cfg.grid_size, cfg.show_axes, cfg.show_axis_direction);

            let initial_z_offset = model::compute_model_z_offset(
                cfg.model_position,
                0.5,
                cfg.object_scale,
            );
            r.update_dummy_box(cfg.object_scale, initial_z_offset);
            Some(r)
        } else {
            None
        };

        let base_scale_factor = 1.0;
        let current_model_size = Vec3::new(0.5, 0.5, 0.5);

        let initial_z_offset = model::compute_model_z_offset(
            cfg.model_position,
            current_model_size.z,
            cfg.object_scale * base_scale_factor,
        );
        let center = Vec3::new(0.0, 0.0, initial_z_offset);
        let eye = center + Vec3::new(cfg.camera_eye[0], cfg.camera_eye[1], cfg.camera_eye[2]);

        let dist_step_value = 1.0 + (cfg.scroll_speed * if cfg.invert_scroll { 1.0 } else { -1.0 });

        let mut camera = FixedCenterCamera::new(
            eye,
            center,
            dist_step_value,
            cfg.scroll_min,
            cfg.scroll_max,
            cfg.smooth_orbit,
        );
        camera.set_loading(initial_loading);

        Self {
            renderer,
            wgpu_render_state,
            camera,
            cfg,
            rx,
            tx,
            registry,
            base_scale_factor,
            current_model_size,
            last_was_loading: initial_loading,
            menu_pos: egui::pos2(120.0, 120.0),
            has_model: false,
            current_mesh_data: None,
            current_model_stats: None,
            show_about: false,
        }
    }

    fn trigger_file_dialog(&mut self) {
        self.camera.dialog_open = true;
        let picked = dialog::trigger_file_dialog(self.registry.clone(), self.tx.clone());
        self.camera.dialog_open = false;
        self.camera.last_dialog_close = Some(std::time::Instant::now());

        if picked {
            self.camera.set_loading(true);
        }
    }

    fn unload_model(&mut self) {
        self.has_model = false;
        self.current_mesh_data = None;
        self.current_model_stats = None;
        self.base_scale_factor = 1.0;
        self.current_model_size = Vec3::new(0.5, 0.5, 0.5);

        if let Some(ref mut r) = self.renderer {
            r.clear_mesh();
            let actual_scale = self.cfg.object_scale * self.base_scale_factor;
            let z_offset = model::compute_model_z_offset(
                self.cfg.model_position,
                self.current_model_size.z,
                actual_scale,
            );
            r.update_dummy_box(actual_scale, z_offset);
            self.camera.set_center(Vec3::new(0.0, 0.0, z_offset));
        }
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
        self.rebuild_grid_and_axes();
        self.apply_controls_settings();
        self.update_model_transform();
    }

    fn update_model_transform(&mut self) {
        let actual_scale = self.cfg.object_scale * self.base_scale_factor;
        let z_offset = model::compute_model_z_offset(
            self.cfg.model_position,
            self.current_model_size.z,
            actual_scale,
        );
        let new_center = Vec3::new(0.0, 0.0, z_offset);
        self.camera.set_center(new_center);

        if self.has_model {
            if let (Some(mesh), Some(r)) = (&self.current_mesh_data, &mut self.renderer) {
                let (min, max) = model::compute_bounds(&mesh.positions);
                let center_offset = (min + max) / 2.0;
                r.upload_mesh(mesh, actual_scale, center_offset, new_center);
            }
        } else if let Some(ref mut r) = self.renderer {
            r.update_dummy_box(actual_scale, z_offset);
        }
    }

    fn rebuild_grid_and_axes(&mut self) {
        if let Some(ref mut r) = self.renderer {
            let grid_color = [
                self.cfg.grid_color[0] as f32 / 255.0,
                self.cfg.grid_color[1] as f32 / 255.0,
                self.cfg.grid_color[2] as f32 / 255.0,
                1.0,
            ];
            r.update_grid(
                self.cfg.grid_size,
                self.cfg.grid_divisions,
                grid_color,
                self.cfg.show_axes,
            );
            r.update_axes(
                self.cfg.grid_size,
                self.cfg.show_axes,
                self.cfg.show_axis_direction,
            );
        }
    }

    fn apply_controls_settings(&mut self) {
        self.camera.smooth_orbit = self.cfg.smooth_orbit;
        let dist_step_value = 1.0
            + (self.cfg.scroll_speed * if self.cfg.invert_scroll { 1.0 } else { -1.0 });
        self.camera.dist_step = dist_step_value;
    }

    fn rebuild_model_node(&mut self) {
        let Some(ref mesh) = self.current_mesh_data else { return };

        let (min, max) = model::compute_bounds(&mesh.positions);
        let center_offset = (min + max) / 2.0;

        let size = (max - min).abs();
        self.current_model_size = size;
        let max_dim = size.x.max(size.y).max(size.z).max(1e-6);
        self.base_scale_factor = 1.0 / max_dim;
        let scale_factor = self.cfg.object_scale * self.base_scale_factor;

        let z_offset = model::compute_model_z_offset(
            self.cfg.model_position,
            self.current_model_size.z,
            scale_factor,
        );
        let model_center = Vec3::new(0.0, 0.0, z_offset);

        if let Some(ref mut r) = self.renderer {
            r.upload_mesh(mesh, scale_factor, center_offset, model_center);
        }

        self.camera.set_center(model_center);
        self.has_model = true;
    }

    fn process_incoming_meshes(&mut self) {
        match self.rx.try_recv() {
            Ok(Ok(mesh)) => {
                self.current_model_stats = Some(compute_stats(&mesh));
                self.current_mesh_data = Some(mesh);
                self.rebuild_model_node();
                self.camera.set_loading(false);
            }
            Ok(Err(err)) => {
                eprintln!("{}", err);
                self.camera.set_loading(false);
            }
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => {}
        }
    }

    fn render_ui(&mut self, ctx: &egui::Context) -> UiResponse {
        let mut ui_resp = UiResponse::default();
        let menu_open = self.camera.menu_open;
        let menu_pos = self.menu_pos;
        let cfg = &mut self.cfg;
        let is_loading = self.camera.is_loading();
        let has_model = self.has_model;
        let lang = cfg.language;

        self.show_about = self.camera.about_open;

        if is_loading {
            overlay::render_loading_overlay(ctx, lang);
        } else if !has_model && overlay::render_empty_overlay(ctx, lang) {
            ui_resp.open_file_dialog = true;
            ui_resp.close_menu_requested = true;
        }

        if cfg.show_dimensions && has_model && !is_loading {
            overlay::render_details_overlay(
                ctx,
                lang,
                self.current_model_size,
                self.current_model_stats,
            );
        }

        if menu_open {
            let menu_resp = render_context_menu(ctx, menu_pos, cfg, has_model);
            let open_dialog = ui_resp.open_file_dialog || menu_resp.open_file_dialog;
            let close_menu = ui_resp.close_menu_requested || menu_resp.close_menu_requested;
            let open_about = ui_resp.open_about_requested || menu_resp.open_about_requested;
            ui_resp = menu_resp;
            ui_resp.open_file_dialog = open_dialog;
            ui_resp.close_menu_requested = close_menu;
            ui_resp.open_about_requested = open_about;
        }

        overlay::render_about_dialog(ctx, &mut self.show_about, cfg.language);
        self.camera.about_open = self.show_about;

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

        if resp.open_about_requested {
            self.show_about = true;
            self.camera.about_open = true;
        }

        if resp.config_changed {
            if let Err(e) = self.cfg.save() {
                eprintln!("Failed to save config: {}", e);
            }
        }

        if resp.scale_changed || resp.position_changed || resp.dummy_box_changed {
            self.update_model_transform();
        }

        if resp.grid_rebuild || resp.axes_rebuild {
            self.rebuild_grid_and_axes();
        }

        if resp.controls_changed {
            self.apply_controls_settings();
        }
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();

        if self.camera.should_close {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }

        let now_loading = self.camera.is_loading();
        if now_loading != self.last_was_loading {
            let lang = self.cfg.language;
            let title = if now_loading {
                lang.t(crate::i18n::TextKey::WindowLoadingTitle).to_string()
            } else {
                lang.t(crate::i18n::TextKey::WindowTitle).to_string()
            };
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title));
            self.last_was_loading = now_loading;
        }

        // Handle Drag & Drop of supported 3D models natively via eframe
        let mut dropped_path = None;
        ctx.input(|i| {
            for file in &i.raw.dropped_files {
                if let Some(ref path) = file.path {
                    if loader::is_supported_file(path) {
                        dropped_path = Some(path.clone());
                        break;
                    }
                }
            }
        });

        if let Some(path) = dropped_path {
            self.camera.set_loading(true);
            dialog::load_file_async(&path, self.registry.clone(), self.tx.clone());
        }

        // Camera updates & animation repaint
        self.camera.update();
        if self.camera.smooth_orbit
            && (self.camera.vel_yaw.abs() > 1e-5 || self.camera.vel_pitch.abs() > 1e-5)
        {
            ctx.request_repaint();
        }
        if self.camera.is_loading() {
            ctx.request_repaint();
        }

        // Check pending requests from camera
        if let Some((mx, my)) = self.camera.pending_menu_pos.take() {
            self.menu_pos = egui::pos2(mx, my);
            self.camera.menu_open = true;
        }
        if self.camera.open_file_requested {
            self.camera.open_file_requested = false;
            self.trigger_file_dialog();
        }

        // Process incoming loaded meshes
        self.process_incoming_meshes();

        // 3D Viewport in CentralPanel
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show_inside(ui, |ui| {
                let available_size = ui.available_size();
                let (rect, response) =
                    ui.allocate_exact_size(available_size, egui::Sense::click_and_drag());

                // Mouse interaction for camera orbit
                if response.dragged_by(egui::PointerButton::Primary) {
                    let delta = response.drag_delta();
                    self.camera.orbit_horizontal(-delta.x * 0.01);
                    self.camera.orbit_vertical(-delta.y * 0.01);
                }

                // Double-click to open file dialog
                if response.double_clicked_by(egui::PointerButton::Primary) {
                    self.trigger_file_dialog();
                }

                // Right click opens context menu
                if response.secondary_clicked() {
                    if let Some(pos) = response.interact_pointer_pos() {
                        self.menu_pos = pos;
                        self.camera.menu_open = true;
                    }
                } else if response.clicked() && self.camera.menu_open {
                    self.camera.menu_open = false;
                }

                // Scroll wheel for camera zoom
                let mut scroll_y = 0.0;
                ctx.input(|i| {
                    for event in &i.raw.events {
                        if let egui::Event::MouseWheel { delta, .. } = event {
                            scroll_y += delta.y;
                        }
                    }
                    if scroll_y == 0.0 && i.smooth_scroll_delta.y != 0.0 {
                        scroll_y = i.smooth_scroll_delta.y / 20.0;
                    }
                });

                if scroll_y.abs() > 1e-4 {
                    let hover_in_rect = ctx
                        .input(|i| i.pointer.hover_pos())
                        .map(|p| rect.contains(p))
                        .unwrap_or(false);
                    if hover_in_rect {
                        let sign = if self.cfg.invert_scroll { -1.0 } else { 1.0 };
                        let speed = self.cfg.scroll_speed * 20.0;
                        self.camera.zoom_delta(scroll_y.signum() * (scroll_y.abs().min(5.0)) * speed * sign);
                    }
                }

                // Keyboard shortcuts
                ctx.input(|i| {
                    if i.key_pressed(egui::Key::Escape) {
                        if self.camera.menu_open {
                            self.camera.menu_open = false;
                        } else if self.show_about {
                            self.show_about = false;
                            self.camera.about_open = false;
                        } else {
                            self.camera.should_close = true;
                        }
                    }
                    if i.key_pressed(egui::Key::Plus) || i.key_pressed(egui::Key::Equals) {
                        self.camera.zoom(true);
                    }
                    if i.key_pressed(egui::Key::Minus) {
                        self.camera.zoom(false);
                    }
                    if i.key_pressed(egui::Key::ArrowLeft) {
                        self.camera.orbit_horizontal(0.05);
                    }
                    if i.key_pressed(egui::Key::ArrowRight) {
                        self.camera.orbit_horizontal(-0.05);
                    }
                    if i.key_pressed(egui::Key::ArrowUp) {
                        self.camera.orbit_vertical(-0.05);
                    }
                    if i.key_pressed(egui::Key::ArrowDown) {
                        self.camera.orbit_vertical(0.05);
                    }
                });

                // Offscreen WGPU rendering
                let pixels_per_point = ctx.pixels_per_point();
                let vp_width = (rect.width() * pixels_per_point).round() as u32;
                let vp_height = (rect.height() * pixels_per_point).round() as u32;

                if vp_width > 0 && vp_height > 0 {
                    if let (Some(wgpu_state), Some(renderer)) =
                        (&self.wgpu_render_state, &mut self.renderer)
                    {
                        let mut egui_renderer = wgpu_state.renderer.write();
                        renderer.resize(vp_width, vp_height, &mut egui_renderer);

                        let aspect = vp_width as f32 / vp_height as f32;
                        let eye = self.camera.eye();
                        let uniforms = Uniforms {
                            view_proj: self.camera.view_projection(aspect).to_cols_array_2d(),
                            camera_pos: [eye.x, eye.y, eye.z],
                            _pad0: 0.0,
                            light_dir: [0.5, 0.5, 1.0],
                            _pad1: 0.0,
                            object_color: [
                                self.cfg.object_color[0] as f32 / 255.0,
                                self.cfg.object_color[1] as f32 / 255.0,
                                self.cfg.object_color[2] as f32 / 255.0,
                                1.0,
                            ],
                            material_mode: match self.cfg.material_mode {
                                crate::config::MaterialMode::Material => 0,
                                crate::config::MaterialMode::Solid => 1,
                                crate::config::MaterialMode::Wireframe => 2,
                            },
                            has_texture: 0,
                            _pad2: [0, 0],
                        };

                        let bg_color = [
                            self.cfg.background[0] as f32 / 255.0,
                            self.cfg.background[1] as f32 / 255.0,
                            self.cfg.background[2] as f32 / 255.0,
                            1.0,
                        ];

                        let show_dummy =
                            !self.has_model && self.cfg.show_dummy_box && !self.camera.is_loading();
                        let show_grid = self.cfg.show_grid;
                        let show_axes = self.cfg.show_axes[0]
                            || self.cfg.show_axes[1]
                            || self.cfg.show_axes[2];

                        renderer.render(&uniforms, bg_color, show_dummy, show_grid, show_axes);

                        if let Some(tex_id) = renderer.egui_texture_id {
                            ui.put(
                                rect,
                                egui::Image::from_texture(egui::load::SizedTexture::new(
                                    tex_id,
                                    rect.size(),
                                )),
                            );
                        }
                    }
                }
            });

        // Overlays & Context Menu
        let ui_resp = self.render_ui(&ctx);
        self.apply_ui_response(ui_resp);
    }
}

fn compute_stats(mesh: &MeshData) -> overlay::ModelStats {
    let vertices = mesh.positions.len();
    if !mesh.indices.is_empty() {
        let faces = mesh.indices.len();
        let mut edges = std::collections::HashSet::with_capacity(faces * 3 / 2);
        for tri in &mesh.indices {
            let (a, b, c) = (tri[0], tri[1], tri[2]);
            if a != b {
                let e = if a < b { (a, b) } else { (b, a) };
                edges.insert(e);
            }
            if b != c {
                let e = if b < c { (b, c) } else { (c, b) };
                edges.insert(e);
            }
            if c != a {
                let e = if c < a { (c, a) } else { (a, c) };
                edges.insert(e);
            }
        }
        overlay::ModelStats {
            vertices,
            lines: edges.len(),
            faces,
        }
    } else {
        let count = mesh.positions.len();
        let faces = count / 3;
        let mut edges = std::collections::HashSet::with_capacity(faces * 3 / 2);
        for i in (0..count).step_by(3) {
            if i + 2 < count {
                let (a, b, c) = (i as u32, (i + 1) as u32, (i + 2) as u32);
                edges.insert((a, b));
                edges.insert((b, c));
                edges.insert((c, a));
            }
        }
        overlay::ModelStats {
            vertices,
            lines: edges.len(),
            faces,
        }
    }
}
