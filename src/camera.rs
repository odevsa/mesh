use kiss3d::camera::{Camera3d, OrbitCamera3d};
use kiss3d::event::{Action, MouseButton, WindowEvent};
use kiss3d::glamx::{Mat4, Pose3, Vec3};
use kiss3d::scene::SceneNode3d;
use kiss3d::window::Canvas;

pub struct FixedCenterCamera {
    pub inner: OrbitCamera3d,
    pub center: Vec3,
    pub dist_step: f32,
    pub object: Option<SceneNode3d>,
    pub light_node: SceneNode3d,
    pub last_cursor: Option<(f32, f32)>,
    pub dragging: bool,
    pub last_click: Option<std::time::Instant>,
    pub loading: bool,
    pub should_close: bool,
    pub smooth_orbit: bool,
    pub vel_yaw: f32,
    pub vel_pitch: f32,
    pub pending_menu_pos: Option<(f32, f32)>,
    pub menu_open: bool,
    pub open_file_requested: bool,
}

impl FixedCenterCamera {
    pub fn new(
        inner: OrbitCamera3d,
        center: Vec3,
        dist_step: f32,
        light_node: SceneNode3d,
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
            loading: false,
            should_close: false,
            smooth_orbit,
            vel_yaw: 0.0,
            vel_pitch: 0.0,
            pending_menu_pos: None,
            menu_open: false,
            open_file_requested: false,
        }
    }

    pub fn set_object(&mut self, obj: SceneNode3d) {
        if let Some(mut old) = self.object.take() {
            old.set_local_scale(0.0, 0.0, 0.0);
            old.set_visible(false);
        }
        self.object = Some(obj);
    }

    pub fn object_mut(&mut self) -> Option<&mut SceneNode3d> {
        self.object.as_mut()
    }

    pub fn set_loading(&mut self, v: bool) {
        self.loading = v;
        if v {
            if let Some(obj) = &mut self.object {
                obj.set_local_scale(0.0, 0.0, 0.0);
            }
        }
    }

    pub fn is_loading(&self) -> bool {
        self.loading
    }

    pub fn center(&self) -> Vec3 {
        self.center
    }

    pub fn set_center(&mut self, new_center: Vec3) {
        let delta = new_center - self.center;
        self.center = new_center;
        let new_eye = self.inner.eye() + delta;
        self.inner.look_at(new_eye, self.center);
        self.inner.set_at(self.center);
    }

    pub fn reset_view(&mut self, eye: Vec3, center: Vec3) {
        self.center = center;
        self.inner.look_at(eye, center);
        self.inner.set_at(center);
        self.vel_yaw = 0.0;
        self.vel_pitch = 0.0;
        self.dragging = false;
        self.last_cursor = None;
    }
}

impl Camera3d for FixedCenterCamera {
    fn clip_planes(&self) -> (f32, f32) {
        self.inner.clip_planes()
    }

    fn view_transform(&self) -> Pose3 {
        self.inner.view_transform()
    }

    fn eye(&self) -> Vec3 {
        self.inner.eye()
    }

    fn handle_event(&mut self, canvas: &Canvas, event: &WindowEvent) {
        use kiss3d::event::Key;

        match event {
            WindowEvent::Scroll(_, off, _) => {
                let offf = *off as f32;
                let new_dist = (self.inner.dist() * self.dist_step.powf(offf))
                    .clamp(self.inner.min_dist(), self.inner.max_dist());
                self.inner.set_dist(new_dist);
                self.inner.set_at(self.center);
            }
            WindowEvent::FramebufferSize(_w, _h) => {
                self.inner.handle_event(canvas, event);
            }
            WindowEvent::CursorPos(x, y, _) => {
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
            WindowEvent::MouseButton(btn, act, _) => {
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
                            self.open_file_requested = true;
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
            WindowEvent::Key(k, act, _) => {
                if *act == Action::Press && *k == Key::Escape {
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

    fn view_transform_pair(&self, pass: usize) -> (Pose3, Mat4) {
        self.inner.view_transform_pair(pass)
    }

    fn render_layers(&self) -> u32 {
        self.inner.render_layers()
    }

    fn transformation(&self) -> Mat4 {
        self.inner.transformation()
    }

    fn inverse_transformation(&self) -> Mat4 {
        self.inner.inverse_transformation()
    }

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

