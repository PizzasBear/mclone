use crate::ProcEvent;
use cgmath::prelude::*;
use std::collections::HashSet;
use std::time::{Duration, Instant};
use winit::event::*;

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub enum Key {
    VirtualKeyCode(VirtualKeyCode),
    MouseButton(MouseButton),
}

pub struct InputManager {
    pressed_keys: HashSet<Key>,
    pressed_down_keys: HashSet<Key>,
    pressed_up_keys: HashSet<Key>,

    modifiers: ModifiersState,

    cursor_position: winit::dpi::PhysicalPosition<f64>,
    delta_cursor_position: cgmath::Vector2<f64>,

    last_update: Instant,
    delta_time: Duration,
}

impl InputManager {
    #[inline]
    pub fn new() -> Self {
        Self {
            pressed_keys: HashSet::new(),
            pressed_down_keys: HashSet::new(),
            pressed_up_keys: HashSet::new(),

            modifiers: ModifiersState::empty(),

            cursor_position: winit::dpi::PhysicalPosition::new(0.0, 0.0),
            delta_cursor_position: Zero::zero(),

            last_update: Instant::now(),
            delta_time: Duration::from_millis(20),
        }
    }

    #[inline]
    pub fn update(&mut self) {
        self.pressed_down_keys.clear();
        self.pressed_up_keys.clear();
        self.delta_cursor_position = Zero::zero();

        self.delta_time = self.last_update.elapsed();
        self.last_update = Instant::now();
    }

    #[inline]
    pub fn key_pressed(&self, key: &Key) -> bool {
        self.pressed_keys.contains(key)
    }

    #[inline]
    pub fn key_down(&self, key: &Key) -> bool {
        self.pressed_down_keys.contains(key)
    }

    #[inline]
    pub fn key_up(&self, key: &Key) -> bool {
        self.pressed_up_keys.contains(key)
    }

    #[inline]
    pub fn modifiers(&self) -> ModifiersState {
        self.modifiers
    }

    #[inline]
    pub fn cursor_position(&self) -> winit::dpi::PhysicalPosition<f64> {
        self.cursor_position
    }

    #[inline]
    pub fn set_cursor_position(&mut self, cursor_position: winit::dpi::PhysicalPosition<f64>) {
        self.cursor_position = cursor_position;
    }

    #[inline]
    pub fn delta_cursor_position(&self) -> cgmath::Vector2<f64> {
        self.delta_cursor_position
    }

    #[inline]
    pub fn delta_time(&self) -> Duration {
        self.delta_time
    }

    pub fn input(&mut self, event: &ProcEvent) {
        match *event {
            ProcEvent::Window(event) => match *event {
                WindowEvent::KeyboardInput {
                    input:
                        KeyboardInput {
                            state,
                            virtual_keycode: Some(virtual_keycode),
                            ..
                        },
                    ..
                } => match state {
                    ElementState::Pressed => {
                        self.pressed_keys
                            .insert(Key::VirtualKeyCode(virtual_keycode));
                        self.pressed_down_keys
                            .insert(Key::VirtualKeyCode(virtual_keycode));
                    }
                    ElementState::Released => {
                        self.pressed_keys
                            .remove(&Key::VirtualKeyCode(virtual_keycode));
                        self.pressed_up_keys
                            .insert(Key::VirtualKeyCode(virtual_keycode));
                    }
                },
                WindowEvent::ModifiersChanged(modifiers) => {
                    self.modifiers = modifiers;
                }
                WindowEvent::CursorMoved { position, .. } => {
                    self.delta_cursor_position = cgmath::Vector2::new(
                        position.x - self.cursor_position.x,
                        position.y - self.cursor_position.y,
                    );
                    self.cursor_position = position;
                }
                WindowEvent::MouseInput { state, button, .. } => match state {
                    ElementState::Pressed => {
                        self.pressed_keys.insert(Key::MouseButton(button));
                        self.pressed_down_keys.insert(Key::MouseButton(button));
                    }
                    ElementState::Released => {
                        self.pressed_keys.remove(&Key::MouseButton(button));
                        self.pressed_up_keys.insert(Key::MouseButton(button));
                    }
                },
                _ => {}
            },
            ProcEvent::User(event) => match event {
                _ => {}
            },
        }
        // UserEvent::MouseMotion {
        //     delta_x,
        //     delta_y,
        //     x,
        //     y,
        // } => {
        //     todo!();
        // }
    }
}
