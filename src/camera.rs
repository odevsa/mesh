use crate::config::Projection;
use glam::{Mat4, Vec3};
use std::time::Instant;

/// Step angle for keyboard orbit navigation (15 degrees = PI / 12 radians).
pub const ORBIT_STEP: f32 = std::f32::consts::PI / 12.0;

pub struct FixedCenterCamera {
    pub center: Vec3,
    pub initial_center: Vec3,
    pub dist: f32,
    pub yaw: f32,
    pub pitch: f32,
    pub min_dist: f32,
    pub max_dist: f32,
    pub dist_step: f32,
    pub smooth_orbit: bool,
    pub vel_yaw: f32,
    pub vel_pitch: f32,
    pub projection: Projection,

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
            initial_center: center,
            dist,
            yaw,
            pitch,
            min_dist,
            max_dist,
            dist_step,
            smooth_orbit,
            vel_yaw: 0.0,
            vel_pitch: 0.0,
            projection: Projection::Perspective,
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

    #[allow(dead_code)]
    pub fn center(&self) -> Vec3 {
        self.center
    }

    pub fn set_center(&mut self, new_center: Vec3) {
        self.initial_center = new_center;
        self.center = new_center;
    }

    pub fn reset_view(&mut self, eye: Vec3, center: Vec3) {
        let diff = eye - center;
        self.dist = diff.length().clamp(self.min_dist, self.max_dist);
        self.center = center;
        self.initial_center = center;
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
        match self.projection {
            Projection::Perspective => Mat4::perspective_rh(
                std::f32::consts::FRAC_PI_4,
                aspect.max(0.001),
                0.01,
                2000.0,
            ),
            Projection::Orthographic => {
                let half_h = self.dist * (std::f32::consts::FRAC_PI_8).tan();
                let half_w = half_h * aspect.max(0.001);
                Mat4::orthographic_rh(
                    -half_w,
                    half_w,
                    -half_h,
                    half_h,
                    -1000.0,
                    1000.0,
                )
            }
        }
    }

    pub fn toggle_projection(&mut self) -> Projection {
        self.projection = match self.projection {
            Projection::Perspective => Projection::Orthographic,
            Projection::Orthographic => Projection::Perspective,
        };
        self.projection
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

    pub fn orbit_vertical_instant(&mut self, delta: f32) {
        self.pitch = (self.pitch + delta).clamp(0.01, std::f32::consts::PI - 0.01);
        self.vel_pitch = 0.0;
    }

    pub fn orbit_horizontal_instant(&mut self, delta: f32) {
        self.yaw += delta;
        self.vel_yaw = 0.0;
    }

    pub fn pan(&mut self, delta_x: f32, delta_y: f32, viewport_height: f32, max_pan: Vec3) {
        let vp_h = viewport_height.max(1.0);
        let fov_factor = 2.0 * (std::f32::consts::FRAC_PI_8).tan();
        let world_per_pixel = (self.dist * fov_factor) / vp_h;

        let eye = self.eye();
        let forward = (self.center - eye).normalize_or_zero();
        let right = forward.cross(Vec3::Z).normalize_or(Vec3::X);
        let up = right.cross(forward).normalize_or(Vec3::Z);

        let offset = (-right * delta_x + up * delta_y) * world_per_pixel;
        let mut new_center = self.center + offset;

        let max_x = max_pan.x.abs();
        let max_y = max_pan.y.abs();
        let max_z = max_pan.z.abs();

        new_center.x = new_center.x.clamp(self.initial_center.x - max_x, self.initial_center.x + max_x);
        new_center.y = new_center.y.clamp(self.initial_center.y - max_y, self.initial_center.y + max_y);
        new_center.z = new_center.z.clamp(self.initial_center.z - max_z, self.initial_center.z + max_z);

        self.center = new_center;
    }

    #[allow(dead_code)]
    pub fn reset_pan(&mut self) {
        self.center = self.initial_center;
    }

    pub fn set_view_top(&mut self) {
        self.center = self.initial_center;
        self.pitch = 0.01;
        self.yaw = -std::f32::consts::FRAC_PI_2;
        self.vel_yaw = 0.0;
        self.vel_pitch = 0.0;
    }

    pub fn set_view_bottom(&mut self) {
        self.center = self.initial_center;
        self.pitch = std::f32::consts::PI - 0.01;
        self.yaw = -std::f32::consts::FRAC_PI_2;
        self.vel_yaw = 0.0;
        self.vel_pitch = 0.0;
    }

    pub fn set_view_front(&mut self) {
        self.center = self.initial_center;
        self.pitch = std::f32::consts::FRAC_PI_2;
        self.yaw = -std::f32::consts::FRAC_PI_2;
        self.vel_yaw = 0.0;
        self.vel_pitch = 0.0;
    }

    pub fn set_view_back(&mut self) {
        self.center = self.initial_center;
        self.pitch = std::f32::consts::FRAC_PI_2;
        self.yaw = std::f32::consts::FRAC_PI_2;
        self.vel_yaw = 0.0;
        self.vel_pitch = 0.0;
    }

    pub fn set_view_right(&mut self) {
        self.center = self.initial_center;
        self.pitch = std::f32::consts::FRAC_PI_2;
        self.yaw = 0.0;
        self.vel_yaw = 0.0;
        self.vel_pitch = 0.0;
    }

    pub fn set_view_left(&mut self) {
        self.center = self.initial_center;
        self.pitch = std::f32::consts::FRAC_PI_2;
        self.yaw = std::f32::consts::PI;
        self.vel_yaw = 0.0;
        self.vel_pitch = 0.0;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_camera_pan_and_clamping() {
        let eye = Vec3::new(0.0, -2.0, 0.0);
        let center = Vec3::new(0.0, 0.0, 0.0);
        let mut camera = FixedCenterCamera::new(eye, center, 0.05, 0.1, 100.0, false);

        let max_pan = Vec3::new(1.0, 1.0, 1.0);
        // Pan horizontally (right)
        camera.pan(100.0, 0.0, 800.0, max_pan);
        assert!(camera.center.x != 0.0);
        assert!((camera.center.x - camera.initial_center.x).abs() <= max_pan.x);

        // Pan with large displacement beyond max_pan
        for _ in 0..100 {
            camera.pan(500.0, 0.0, 800.0, max_pan);
        }
        assert!((camera.center.x - camera.initial_center.x).abs() <= max_pan.x + 1e-5);
        assert!((camera.center.y - camera.initial_center.y).abs() <= max_pan.y + 1e-5);
        assert!((camera.center.z - camera.initial_center.z).abs() <= max_pan.z + 1e-5);

        // Reset pan
        camera.reset_pan();
        assert_eq!(camera.center, camera.initial_center);
    }

    #[test]
    fn test_camera_view_presets_reset_pan() {
        let eye = Vec3::new(0.0, -2.0, 0.0);
        let center = Vec3::new(0.0, 0.0, 0.5);
        let mut camera = FixedCenterCamera::new(eye, center, 0.05, 0.1, 100.0, false);

        // Apply some pan
        camera.pan(50.0, 50.0, 800.0, Vec3::splat(1.0));
        assert_ne!(camera.center, camera.initial_center);

        // Top view preset
        camera.set_view_top();
        assert_eq!(camera.center, camera.initial_center);
        assert!((camera.pitch - 0.01).abs() < 1e-4);

        // Pan again
        camera.pan(-50.0, 30.0, 800.0, Vec3::splat(1.0));
        assert_ne!(camera.center, camera.initial_center);

        // Bottom view preset
        camera.set_view_bottom();
        assert_eq!(camera.center, camera.initial_center);
        assert!((camera.pitch - (std::f32::consts::PI - 0.01)).abs() < 1e-4);

        // Front view preset
        camera.set_view_front();
        assert_eq!(camera.center, camera.initial_center);
        assert!((camera.pitch - std::f32::consts::FRAC_PI_2).abs() < 1e-4);
        assert!((camera.yaw - (-std::f32::consts::FRAC_PI_2)).abs() < 1e-4);

        // Back view preset
        camera.set_view_back();
        assert_eq!(camera.center, camera.initial_center);
        assert!((camera.pitch - std::f32::consts::FRAC_PI_2).abs() < 1e-4);
        assert!((camera.yaw - std::f32::consts::FRAC_PI_2).abs() < 1e-4);

        // Right view preset
        camera.set_view_right();
        assert_eq!(camera.center, camera.initial_center);
        assert!((camera.pitch - std::f32::consts::FRAC_PI_2).abs() < 1e-4);
        assert!(camera.yaw.abs() < 1e-4);

        // Left view preset
        camera.set_view_left();
        assert_eq!(camera.center, camera.initial_center);
        assert!((camera.pitch - std::f32::consts::FRAC_PI_2).abs() < 1e-4);
        assert!((camera.yaw - std::f32::consts::PI).abs() < 1e-4);
    }

    #[test]
    fn test_camera_toggle_projection() {
        let eye = Vec3::new(0.0, -2.0, 0.0);
        let center = Vec3::new(0.0, 0.0, 0.0);
        let mut camera = FixedCenterCamera::new(eye, center, 0.05, 0.1, 100.0, false);
        assert_eq!(camera.projection, Projection::Perspective);

        let p1 = camera.projection_matrix(1.0);
        assert_ne!(p1.w_axis.z, 0.0);

        let toggled = camera.toggle_projection();
        assert_eq!(toggled, Projection::Orthographic);
        assert_eq!(camera.projection, Projection::Orthographic);

        let p2 = camera.projection_matrix(1.0);
        assert_eq!(p2.w_axis.w, 1.0);
        assert_eq!(p2.z_axis.w, 0.0);

        let toggled_back = camera.toggle_projection();
        assert_eq!(toggled_back, Projection::Perspective);
        assert_eq!(camera.projection, Projection::Perspective);
    }

    #[test]
    fn test_camera_instant_orbit() {
        let eye = Vec3::new(0.0, -2.0, 0.0);
        let center = Vec3::new(0.0, 0.0, 0.0);
        let mut camera = FixedCenterCamera::new(eye, center, 0.05, 0.1, 100.0, true);

        // Smooth orbit sets velocities
        camera.orbit_horizontal(0.5);
        assert!(camera.vel_yaw != 0.0);

        // Instant orbit zeroes velocity and rotates by exact step
        let old_yaw = camera.yaw;
        camera.orbit_horizontal_instant(ORBIT_STEP);
        assert_eq!(camera.vel_yaw, 0.0);
        assert!((camera.yaw - (old_yaw + ORBIT_STEP)).abs() < 1e-5);

        // Instant vertical orbit
        let old_pitch = camera.pitch;
        camera.orbit_vertical_instant(-ORBIT_STEP);
        assert_eq!(camera.vel_pitch, 0.0);
        assert!((camera.pitch - (old_pitch - ORBIT_STEP)).abs() < 1e-5);
    }
}
