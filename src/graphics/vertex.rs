#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    position: [f32; 2],
}

impl Vertex {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x2,
                }
            ],
        }
    }
}

// Function to generate vertices for the waveform
pub fn generate_waveform_vertices(num_vertices: usize) -> Vec<Vertex> {
    let mut vertices = Vec::with_capacity(num_vertices);
    let x_step = 2.0 / (num_vertices as f32 - 1.0); // Step to cover [-1, 1] range
    
    for i in 0..num_vertices {
        let x = -1.0 + (i as f32 * x_step);
        // Placeholder Y value, you might want to adjust it based on your audio data
        let y = 0.0; 
        vertices.push(Vertex { position: [x, y] });
    }

    vertices
}
