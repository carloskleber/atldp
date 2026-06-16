//! Orbit camera (3D) and pan/zoom camera (2D). Pure math — no event handling.

use glam::{Mat4, Vec3};

/// Spherical-coordinate orbit camera for the 3D viewport.
///
/// The camera looks at `target`; the eye position is computed from `pitch`,
/// `yaw`, and `distance`. Interaction state (mouse drag, scroll) lives in
/// `atldp-app`, not here.
pub struct OrbitCamera {
    pub target: Vec3,
    pub distance: f32,
    /// Elevation angle, radians. Clamped to (-π/2 + ε, π/2 - ε) by the app.
    pub pitch: f32,
    /// Azimuth angle, radians.
    pub yaw: f32,
    pub fov_y: f32,
    pub near: f32,
    pub far: f32,
}

impl OrbitCamera {
    pub fn new() -> Self {
        Self {
            target: Vec3::ZERO,
            distance: 500.0,
            pitch: 0.35,
            yaw: 0.6,
            fov_y: 45.0_f32.to_radians(),
            near: 1.0,
            far: 50_000.0,
        }
    }

    pub fn eye(&self) -> Vec3 {
        let (sp, cp) = self.pitch.sin_cos();
        let (sy, cy) = self.yaw.sin_cos();
        self.target + Vec3::new(cp * sy, sp, cp * cy) * self.distance
    }

    pub fn view_matrix(&self) -> Mat4 {
        Mat4::look_at_rh(self.eye(), self.target, Vec3::Y)
    }

    pub fn proj_matrix(&self, aspect: f32) -> Mat4 {
        Mat4::perspective_rh(self.fov_y, aspect, self.near, self.far)
    }

    pub fn view_proj(&self, aspect: f32) -> Mat4 {
        self.proj_matrix(aspect) * self.view_matrix()
    }
}

impl Default for OrbitCamera {
    fn default() -> Self {
        Self::new()
    }
}

/// Orthographic 2D camera for the plan/profile viewport.
pub struct Camera2D {
    /// World-space center of the view (metres).
    pub center: [f32; 2],
    /// Pixels per metre (zoom level).
    pub pixels_per_metre: f32,
}

impl Camera2D {
    pub fn new() -> Self {
        Self {
            center: [150.0, 0.0],
            pixels_per_metre: 1.5,
        }
    }

    /// World → screen, given `viewport_size` in pixels. Y is flipped (up = screen-up).
    pub fn world_to_screen(&self, world: [f32; 2], viewport: [f32; 2]) -> [f32; 2] {
        let s = self.pixels_per_metre;
        [
            viewport[0] * 0.5 + (world[0] - self.center[0]) * s,
            viewport[1] * 0.5 - (world[1] - self.center[1]) * s,
        ]
    }
}

impl Default for Camera2D {
    fn default() -> Self {
        Self::new()
    }
}
