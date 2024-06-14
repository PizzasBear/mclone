use winit::{
    event::*,
    keyboard::{KeyCode, PhysicalKey},
};

#[derive(Debug)]
pub struct Camera {
    pub pos: glam::Vec3,
    pub rot: glam::Vec2,
    pub aspect: f32,
    pub fovy: f32,
    pub znear: f32,
    pub zfar: f32,
}

impl Camera {
    fn build_view_projection_matrix(&self) -> glam::Mat4 {
        let (sin_x, cos_x) = self.rot.x.to_radians().sin_cos();
        let (sin_y, cos_y) = self.rot.y.to_radians().sin_cos();
        let dir = glam::vec3(cos_x * sin_y, sin_x, cos_x * cos_y);
        let view = glam::Mat4::look_to_rh(self.pos, -dir, glam::Vec3::Y);
        let proj = glam::Mat4::perspective_rh(self.fovy, self.aspect, self.znear, self.zfar);
        proj * view
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    pub view_proj: glam::Mat4,
    pub view_pos: glam::Vec3,
    pub _pad1: u32,
}

impl CameraUniform {
    pub fn new() -> Self {
        Self {
            view_proj: glam::Mat4::IDENTITY,
            view_pos: glam::Vec3::ZERO,
            _pad1: 0,
        }
    }

    pub fn update_view_proj(&mut self, camera: &Camera) {
        self.view_proj = camera.build_view_projection_matrix();
        self.view_pos = camera.pos;
    }
}

pub struct CameraController {
    speed: f32,
    sensitivity: f32,
    vel: glam::Vec3,
    im_vel: glam::Vec3,

    ang_vel: glam::Vec2,
}

impl CameraController {
    pub fn new(speed: f32, sensitivity: f32) -> Self {
        Self {
            speed,
            sensitivity,
            vel: glam::Vec3::ZERO,
            im_vel: glam::Vec3::ZERO,

            ang_vel: glam::Vec2::ZERO,
        }
    }

    pub fn device_event(&mut self, event: &DeviceEvent) -> bool {
        match event {
            DeviceEvent::MouseMotion { delta } => {
                self.ang_vel.x += delta.1 as f32 * self.sensitivity;
                self.ang_vel.y -= delta.0 as f32 * self.sensitivity;

                true
            }
            _ => false,
        }
    }

    pub fn window_event(&mut self, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key,
                        state: key_state,
                        repeat: false,
                        ..
                    },
                ..
            } => match physical_key {
                PhysicalKey::Code(KeyCode::KeyW | KeyCode::ArrowUp) => {
                    self.im_vel.z -= (1 - 2 * *key_state as i8) as f32;
                    true
                }
                PhysicalKey::Code(KeyCode::KeyS | KeyCode::ArrowDown) => {
                    self.im_vel.z += (1 - 2 * *key_state as i8) as f32;
                    true
                }
                PhysicalKey::Code(KeyCode::KeyA | KeyCode::ArrowLeft) => {
                    self.im_vel.x -= (1 - 2 * *key_state as i8) as f32;
                    true
                }
                PhysicalKey::Code(KeyCode::KeyD | KeyCode::ArrowRight) => {
                    self.im_vel.x += (1 - 2 * *key_state as i8) as f32;
                    true
                }
                PhysicalKey::Code(KeyCode::Space) => {
                    self.im_vel.y += (1 - 2 * *key_state as i8) as f32;
                    true
                }
                PhysicalKey::Code(KeyCode::ShiftLeft) => {
                    self.im_vel.y -= (1 - 2 * *key_state as i8) as f32;
                    true
                }
                _ => false,
            },
            _ => false,
        }
    }

    pub fn update_camera(&mut self, delta_time: f32, camera: &mut Camera) {
        let dvel = self.im_vel - self.vel;
        let dvel_len = dvel.length();
        if 0.1 < dvel_len {
            self.vel += (delta_time * 10.0 / dvel_len).min(0.96875) * dvel;
        } else {
            self.vel = self.im_vel;
        }
        let rot_vel = glam::Quat::from_rotation_y(camera.rot.y.to_radians()) * self.vel;
        camera.pos += delta_time * self.speed * rot_vel;

        camera.rot += self.ang_vel / 100.0;
        self.ang_vel = glam::Vec2::ZERO;

        camera.rot.y %= 360.0;
        camera.rot.x = camera.rot.x.clamp(-89.0, 89.0);
    }
}
