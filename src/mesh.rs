use cgmath::prelude::*;
use std::mem::size_of;
use wgpu::util::DeviceExt;

#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct Vertex {
    pub position: [f32; 3],
    pub tex_coords: [f32; 2],
    pub color: [f32; 3],
}

impl Vertex {
    #[inline]
    pub const fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        const ATTRIBUTES: &[wgpu::VertexAttribute] =
            &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2, 2 => Float32x3];

        wgpu::VertexBufferLayout {
            array_stride: size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: ATTRIBUTES,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
#[repr(C)]
pub struct Instance {
    pub position: cgmath::Point3<f32>,
    pub rotation: cgmath::Quaternion<f32>,
}

#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct InstanceRaw {
    model: [[f32; 4]; 4],
}

impl Instance {
    pub fn to_raw(&self) -> InstanceRaw {
        InstanceRaw {
            model: (cgmath::Matrix4::from_translation(self.position.to_vec())
                * cgmath::Matrix4::from(self.rotation))
            .into(),
        }
    }
}

impl InstanceRaw {
    #[inline]
    pub const fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        const ATTRIBUTES: &[wgpu::VertexAttribute] = &wgpu::vertex_attr_array![5 => Float32x4, 6 =>Float32x4, 7 => Float32x4, 8 => Float32x4];

        wgpu::VertexBufferLayout {
            array_stride: size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: ATTRIBUTES,
        }
    }
}

pub struct Mesh {
    label: Option<String>,

    vertices: Vec<Vertex>,
    indices: Vec<[u32; 3]>,
    instances: Vec<Instance>,

    vertex_buffer: Option<wgpu::Buffer>,
    index_buffer: Option<wgpu::Buffer>,
    instance_buffer: Option<wgpu::Buffer>,
}

impl Mesh {
    pub fn new(
        label: Option<String>,
        vertices: Vec<Vertex>,
        indices: Vec<[u32; 3]>,
        instances: Vec<Instance>,
    ) -> Self {
        Self {
            label,

            vertices,
            indices,
            instances,

            vertex_buffer: None,
            index_buffer: None,
            instance_buffer: None,
        }
    }

    pub fn empty(label: Option<String>) -> Self {
        Self {
            label,

            vertices: vec![],
            indices: vec![],
            instances: vec![],

            vertex_buffer: None,
            index_buffer: None,
            instance_buffer: None,
        }
    }

    pub fn vertices(&self) -> &[Vertex] {
        &self.vertices
    }

    pub fn indices(&self) -> &[[u32; 3]] {
        &self.indices
    }

    pub fn instances(&self) -> &[Instance] {
        &self.instances
    }

    pub fn vertices_mut(&mut self) -> &mut Vec<Vertex> {
        self.vertex_buffer = None;
        &mut self.vertices
    }

    pub fn indices_mut(&mut self) -> &mut Vec<[u32; 3]> {
        self.index_buffer = None;
        &mut self.indices
    }

    pub fn instances_mut(&mut self) -> &mut Vec<Instance> {
        self.instance_buffer = None;
        &mut self.instances
    }

    pub fn draw<'a>(
        &'a mut self,
        device: &mut wgpu::Device,
        render_pass: &mut wgpu::RenderPass<'a>,
    ) {
        let vertex_buffer = self.vertex_buffer.get_or_insert_with(|| {
            println!("Created a Vertex Buffer");

            let label = self
                .label
                .as_ref()
                .map(|label| format!("{} / Vertex Buffer", label));
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: label.as_ref().map(String::as_str),
                contents: bytemuck::cast_slice(&self.vertices),
                usage: wgpu::BufferUsages::VERTEX,
            })
        });
        let index_buffer = self.index_buffer.get_or_insert_with(|| {
            println!("Created an Index Buffer");

            let label = self
                .label
                .as_ref()
                .map(|label| format!("{} / Index Buffer", label));
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: label.as_ref().map(String::as_str),
                contents: bytemuck::cast_slice(&self.indices),
                usage: wgpu::BufferUsages::INDEX,
            })
        });
        let instance_buffer = self.instance_buffer.get_or_insert_with(|| {
            println!("Created an Instance Buffer");

            let label = self
                .label
                .as_ref()
                .map(|label| format!("{} / Instance Buffer", label));
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: label.as_ref().map(String::as_str),
                contents: bytemuck::cast_slice(
                    &self
                        .instances
                        .iter()
                        .map(Instance::to_raw)
                        .collect::<Vec<_>>(),
                ),
                usage: wgpu::BufferUsages::VERTEX,
            })
        });

        render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
        render_pass.set_vertex_buffer(1, instance_buffer.slice(..));
        render_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);

        render_pass.draw_indexed(
            0..3 * self.indices.len() as u32,
            0,
            0..self.instances.len() as _,
        );
    }
}
