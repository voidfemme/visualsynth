use bytemuck;

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct MousePosition {
    pub x: f32,
    pub y: f32,
}
