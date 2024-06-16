use std::mem;

use wgpu::util::DeviceExt;
use winit::dpi::PhysicalSize;

use crate::{
    graphics::{self, VertexBuffer},
    texture,
};

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: glam::Vec2,
    pub tex_coords: glam::Vec2,
    pub color: [u8; 4],
}

impl VertexBuffer for Vertex {
    type Raw = Self;
    fn to_raw(&self) -> Self {
        *self
    }
    const DESC: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: mem::size_of::<Vertex>() as _,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &wgpu::vertex_attr_array![
            0 => Float32x2, // position
            1 => Float32x2, // tex_coords
            2 => Unorm8x4, // color
        ],
    };
}

#[repr(C, packed(4))]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct InstanceRaw {
    pub model: glam::Mat2,
    pub position: glam::Vec3,
}

#[derive(Debug, Copy, Clone)]
pub struct Instance {
    pub position: glam::Vec3,
    pub scale: glam::Vec2,
    pub angle: f32,
}

impl VertexBuffer for Instance {
    type Raw = InstanceRaw;
    fn to_raw(&self) -> InstanceRaw {
        InstanceRaw {
            model: glam::Mat2::from_scale_angle(self.scale, self.angle),
            position: self.position,
        }
    }
    const DESC: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: mem::size_of::<InstanceRaw>() as _,
        step_mode: wgpu::VertexStepMode::Instance,
        attributes: &wgpu::vertex_attr_array![
            3 => Float32x2, // model1
            4 => Float32x2, // model2
            5 => Float32x3, // position
        ],
    };
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GuiUniform {
    pub resolution: glam::Vec2,
    pub _pad1: u64,
}

pub struct Gui {
    pub square_vertices: wgpu::Buffer,
    pub square_indices: wgpu::Buffer,

    pub bind_group_layout: wgpu::BindGroupLayout,
    pub render_pipeline: wgpu::RenderPipeline,

    pub uniform: GuiUniform,
    pub uniform_buffer: wgpu::Buffer,
    pub uniform_bind_group: wgpu::BindGroup,
}

impl Gui {
    pub fn new(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        resolution: PhysicalSize<u32>,
    ) -> Self {
        let color = [255; 4];
        let square_vertices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Square Buffer"),
            contents: bytemuck::cast_slice(&[
                Vertex {
                    position: glam::Vec2::new(-1.0, -1.0),
                    tex_coords: glam::Vec2::new(0.0, 0.0),
                    color,
                },
                Vertex {
                    position: glam::Vec2::new(1.0, -1.0),
                    tex_coords: glam::Vec2::new(1.0, 0.0),
                    color,
                },
                Vertex {
                    position: glam::Vec2::new(1.0, 1.0),
                    tex_coords: glam::Vec2::new(1.0, 1.0),
                    color,
                },
                Vertex {
                    position: glam::Vec2::new(-1.0, 1.0),
                    tex_coords: glam::Vec2::new(0.0, 1.0),
                    color,
                },
            ]),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let square_indices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Square Indices"),
            contents: bytemuck::cast_slice(&[0u16, 1, 2, 0, 2, 3]),
            usage: wgpu::BufferUsages::INDEX,
        });

        let bind_group_layout = Self::create_bind_group_layout(device);

        let uniform = GuiUniform {
            resolution: glam::vec2(resolution.width as _, resolution.height as _),
            _pad1: 0,
        };
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("GUI Uniform Buffer"),
            contents: bytemuck::cast_slice(&[uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let uniform_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("GUI Uniform Bind Group Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });
        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("GUI Uniform Bind Group"),
            layout: &uniform_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        Self {
            square_vertices,
            square_indices,
            render_pipeline: Self::create_render_pipeline(
                device,
                config,
                &bind_group_layout,
                &uniform_bind_group_layout,
            ),
            bind_group_layout,

            uniform,
            uniform_buffer,
            uniform_bind_group,
        }
    }

    pub fn create_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("GUI Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        })
    }

    pub fn create_render_pipeline(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        gui_bind_group_layout: &wgpu::BindGroupLayout,
        gui_uniform_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> wgpu::RenderPipeline {
        graphics::create_render_pipeline(
            device,
            config,
            "GUI Render Pipeline",
            &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("GUI Render Pipeline Layout"),
                bind_group_layouts: &[gui_bind_group_layout, gui_uniform_bind_group_layout],
                push_constant_ranges: &[],
            }),
            &[Vertex::DESC, Instance::DESC],
            &device.create_shader_module(wgpu::include_wgsl!("gui.wgsl")),
        )
    }

    pub fn resize(&mut self, queue: &wgpu::Queue, resolution: PhysicalSize<u32>) {
        self.uniform.resolution = glam::vec2(resolution.width as _, resolution.height as _);
        queue.write_buffer(
            &self.uniform_buffer,
            0,
            bytemuck::cast_slice(&[self.uniform]),
        );
    }

    pub fn draw_sprite<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_bind_group(1, &self.uniform_bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.square_vertices.slice(..));
        render_pass.set_index_buffer(self.square_indices.slice(..), wgpu::IndexFormat::Uint16);
        render_pass.draw_indexed(0..6, 0, 0..1);
    }
}

pub struct Sprite {
    pub instance: Instance,
    pub instance_buffer: wgpu::Buffer,

    pub texture: texture::Texture,
    pub bind_group: wgpu::BindGroup,
}

impl Sprite {
    pub fn new(
        device: &wgpu::Device,
        gui: &Gui,
        texture: texture::Texture,
        instance: Instance,
    ) -> Self {
        let instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Sprite Instance Buffer"),
            contents: bytemuck::cast_slice(&[instance.to_raw()]),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });
        Self {
            instance,
            instance_buffer,

            bind_group: device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Sprite Bind Group"),
                layout: &gui.bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&texture.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&texture.sampler),
                    },
                ],
            }),
            texture,
        }
    }

    pub fn update_instance(&mut self, queue: &wgpu::Queue) {
        queue.write_buffer(
            &self.instance_buffer,
            0,
            bytemuck::cast_slice(&[self.instance.to_raw()]),
        );
    }

    pub fn draw<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>, gui: &'a Gui) {
        render_pass.set_bind_group(0, &self.bind_group, &[]);
        render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
        gui.draw_sprite(render_pass);
    }
}
