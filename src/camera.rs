use glam::{Mat4, Vec3};
use std::time::Instant;

pub struct FixedCenterCamera {
    pub center: Vec3,
    pub dist: f32,
    pub yaw: f32,
    pub pitch: f32,
    pub min_dist: f32,
    pub max_dist: f32,
    pub dist_step: f32,
    pub smooth_orbit: bool,
    pub vel_yaw: f32,
    pub vel_pitch: f32,

    pub loading: bool,
    pub should_close: bool,
    pub menu_open: bool,
    pub about_open: bool,
    pub dialog_open: bool,
    pub last_dialog_close: Option<Instant>,
    pub open_file_requested: bool,
    pub pending_menu_pos: Option<(f32, f32)>,
}

impl FixedCenterCamera {
    pub fn new(
        eye: Vec3,
        center: Vec3,
        dist_step: f32,
        min_dist: f32,
        max_dist: f32,
        smooth_orbit: bool,
    ) -> Self {
        let diff = eye - center;
        let dist = diff.length().clamp(min_dist, max_dist);
        let pitch = (diff.z / dist.max(1e-4)).clamp(-1.0, 1.0).acos().clamp(0.01, std::f32::consts::PI - 0.01);
        let yaw = diff.y.atan2(diff.x);

        Self {
            center,
            dist,
            yaw,
            pitch,
            min_dist,
            max_dist,
            dist_step,
            smooth_orbit,
            vel_yaw: 0.0,
            vel_pitch: 0.0,
            loading: false,
            should_close: false,
            menu_open: false,
            about_open: false,
            dialog_open: false,
            last_dialog_close: None,
            open_file_requested: false,
            pending_menu_pos: None,
        }
    }

    pub fn set_loading(&mut self, v: bool) {
        self.loading = v;
    }

    pub fn is_loading(&self) -> bool {
        self.loading
    }

    pub fn center(&self) -> Vec3 {
        self.center
    }

    pub fn set_center(&mut self, new_center: Vec3) {
        self.center = new_center;
    }

    pub fn reset_view(&mut self, eye: Vec3, center: Vec3) {
        let diff = eye - center;
        self.dist = diff.length().clamp(self.min_dist, self.max_dist);
        self.center = center;
        self.pitch = (diff.z / self.dist.max(1e-4)).clamp(-1.0, 1.0).acos().clamp(0.01, std::f32::consts::PI - 0.01);
        self.yaw = diff.y.atan2(diff.x);
        self.vel_yaw = 0.0;
        self.vel_pitch = 0.0;
    }

    pub fn eye(&self) -> Vec3 {
        let p = self.pitch.clamp(0.01, std::f32::consts::PI - 0.01);
        let x = self.center.x + self.dist * p.sin() * self.yaw.cos();
        let y = self.center.y + self.dist * p.sin() * self.yaw.sin();
        let z = self.center.z + self.dist * p.cos();
        Vec3::new(x, y, z)
    }

    pub fn view_matrix(&self) -> Mat4 {
        Mat4::look_at_rh(self.eye(), self.center, Vec3::Z)
    }

    pub fn projection_matrix(&self, aspect: f32) -> Mat4 {
        Mat4::perspective_rh(
            std::f32::consts::FRAC_PI_4,
            aspect.max(0.001),
            0.01,
            2000.0,
        )
    }

    pub fn view_projection(&self, aspect: f32) -> Mat4 {
        self.projection_matrix(aspect) * self.view_matrix()
    }

    pub fn orbit_vertical(&mut self, delta: f32) {
        self.pitch = (self.pitch + delta).clamp(0.01, std::f32::consts::PI - 0.01);
        if self.smooth_orbit {
            self.vel_pitch = delta * 0.3;
        }
    }

    pub fn orbit_horizontal(&mut self, delta: f32) {
        self.yaw += delta;
        if self.smooth_orbit {
            self.vel_yaw = delta * 0.3;
        }
    }

    pub fn zoom(&mut self, zoom_in: bool) {
        let mut factor = self.dist_step.powf(10.0);
        if factor > 1.0 {
            factor = 1.0 / factor;
        }
        self.dist = if zoom_in {
            self.dist * factor
        } else {
            self.dist / factor
        }
        .clamp(self.min_dist, self.max_dist);
    }

    pub fn zoom_delta(&mut self, scroll_delta: f32) {
        let factor = 1.08f32.powf(scroll_delta);
        self.dist = (self.dist / factor).clamp(self.min_dist, self.max_dist);
    }

    pub fn update(&mut self) {
        if self.smooth_orbit {
            if self.vel_yaw.abs() > 1e-5 || self.vel_pitch.abs() > 1e-5 {
                self.yaw += self.vel_yaw;
                self.pitch = (self.pitch + self.vel_pitch).clamp(0.01, std::f32::consts::PI - 0.01);
                self.vel_yaw *= 0.92;
                self.vel_pitch *= 0.92;
            } else {
                self.vel_yaw = 0.0;
                self.vel_pitch = 0.0;
            }
        }
    }
}
