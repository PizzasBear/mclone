use crate::{input::Key, Camera, InputManager};
use cgmath::{prelude::*, Point3, Vector3};
use winit::event::VirtualKeyCode;

pub struct Player {
    pos: Point3<f32>,
    yaw: cgmath::Rad<f32>,
    pitch: cgmath::Rad<f32>,

    forward_speed: f32,
    speed: f32,

    mouse_sensitivity: f32,
}

impl Player {
    pub fn new(forward_speed: f32, speed: f32, mouse_sensitivity: f32) -> Self {
        Self {
            pos: Point3::origin(),
            yaw: cgmath::Rad(0.0),
            pitch: cgmath::Rad(0.0),

            forward_speed,
            speed,

            mouse_sensitivity,
        }
    }

    pub fn camera(&self) -> Camera {
        Camera::new(self.pos + Vector3::new(0.0, 1.8, 0.0), self.yaw, self.pitch)
    }

    pub fn update(&mut self, input: &InputManager) {
        let cam = self.camera();

        if input.key_pressed(&Key::VirtualKeyCode(VirtualKeyCode::W)) {
            self.pos += cam.forward_xz() * (self.forward_speed * input.delta_time().as_secs_f32());
        }
        if input.key_pressed(&Key::VirtualKeyCode(VirtualKeyCode::S)) {
            self.pos -= cam.forward_xz() * (self.speed * input.delta_time().as_secs_f32());
        }
        if input.key_pressed(&Key::VirtualKeyCode(VirtualKeyCode::D)) {
            self.pos += cam.right_xz() * (self.speed * input.delta_time().as_secs_f32());
        }
        if input.key_pressed(&Key::VirtualKeyCode(VirtualKeyCode::A)) {
            self.pos -= cam.right_xz() * (self.speed * input.delta_time().as_secs_f32());
        }
        if input.key_pressed(&Key::VirtualKeyCode(VirtualKeyCode::Space)) {
            self.pos += cgmath::Vector3::unit_y() * (self.speed * input.delta_time().as_secs_f32());
        }
        if input.modifiers().shift() {
            self.pos -= cgmath::Vector3::unit_y() * (self.speed * input.delta_time().as_secs_f32());
        }

        self.yaw -= cgmath::Rad(input.delta_cursor_position().x as f32 * self.mouse_sensitivity);
        self.yaw.0 = self.yaw.0 % (2.0 * std::f32::consts::PI);
        self.pitch -= cgmath::Rad(input.delta_cursor_position().y as f32 * self.mouse_sensitivity);
        self.pitch.0 = self.pitch.0.clamp(
            -std::f32::consts::FRAC_PI_2 * 0.98,
            std::f32::consts::FRAC_PI_2 * 0.98,
        );
    }
}
