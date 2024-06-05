use winit::event::*;

pub enum UserEvent {}
// MouseMotion {
//     delta_x: f32,
//     delta_y: f32,
//     x: f32,
//     y: f32,
// },

pub enum ProcEvent<'a> {
    User(&'a UserEvent),
    Window(&'a WindowEvent<'a>),
}
