use cgmath::prelude::*;
use cgmath::{perspective, Matrix4, Point3, Rad, Vector3};

#[derive(Debug, Clone)]
pub struct Camera {
    pub pos: Point3<f32>,
    pub yaw: Rad<f32>,
    pub pitch: Rad<f32>,
}

pub struct Projection {
    aspect: f32,
    fovy: Rad<f32>,
    znear: f32,
    zfar: f32,
}

#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: Matrix4<f32> = Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.0,
    0.0, 0.0, 0.5, 1.0,
);

impl Camera {
    #[inline]
    pub fn new(pos: Point3<f32>, yaw: Rad<f32>, pitch: Rad<f32>) -> Self {
        Self { pos, yaw, pitch }
    }

    #[inline]
    pub fn forward(&self) -> Vector3<f32> {
        let (sin_yaw, cos_yaw) = self.yaw.sin_cos();
        let (sin_pitch, cos_pitch) = self.pitch.sin_cos();

        // Vector3::new(sy * cx, sx, cy * cx)
        Vector3::new(sin_yaw * cos_pitch, sin_pitch, cos_yaw * cos_pitch)
    }

    /// `forward()` without the `y` component.
    #[inline]
    pub fn forward_xz(&self) -> Vector3<f32> {
        let (sin_yaw, cos_yaw) = self.yaw.sin_cos();

        Vector3::new(sin_yaw, 0.0, cos_yaw)
    }

    #[inline]
    pub fn right_xz(&self) -> Vector3<f32> {
        let (sin_yaw, cos_yaw) = self.yaw.sin_cos();

        Vector3::new(-cos_yaw, 0.0, sin_yaw)
    }

    #[inline]
    pub fn calc_matrix(&self) -> Matrix4<f32> {
        Matrix4::look_to_rh(self.pos, self.forward(), cgmath::Vector3::unit_y())
    }
}

impl Projection {
    pub fn new(width: u32, height: u32, fovy: Rad<f32>, znear: f32, zfar: f32) -> Self {
        let mut slf = Self {
            aspect: 1.0,
            fovy,
            znear,
            zfar,
        };
        slf.resize(width, height);

        slf
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.aspect = width as f32 / height as f32;
    }

    pub fn calc_matrix(&self) -> Matrix4<f32> {
        OPENGL_TO_WGPU_MATRIX * perspective(self.fovy, self.aspect, self.znear, self.zfar)
    }
}
