use cgmath::prelude::*;
use futures::executor::block_on;
use wgpu::util::DeviceExt;
use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

pub mod camera;
pub mod chunk;
pub mod event;
pub mod input;
pub mod mesh;
pub mod player;
pub mod texture;

pub use camera::{Camera, Projection};
pub use event::{ProcEvent, UserEvent};
pub use input::InputManager;
pub use player::Player;

pub use chunk::Chunk;

#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
struct CameraUniform {
    model: [[f32; 4]; 4],
}

const VERTICES: &[mesh::Vertex] = &[
    mesh::Vertex {
        position: [-0.0868241, 0.49240386, 0.0],
        tex_coords: [0.4131759, 0.00759614],
        color: [1.0, 1.0, 1.0],
    },
    mesh::Vertex {
        position: [-0.49513406, 0.06958647, 0.0],
        tex_coords: [0.0048659444, 0.43041354],
        color: [1.0, 1.0, 1.0],
    },
    mesh::Vertex {
        position: [-0.21918549, -0.44939706, 0.0],
        tex_coords: [0.28081453, 0.949397],
        color: [1.0, 1.0, 1.0],
    },
    mesh::Vertex {
        position: [0.35966998, -0.3473291, 0.0],
        tex_coords: [0.85967, 0.84732914],
        color: [1.0, 1.0, 1.0],
    },
    mesh::Vertex {
        position: [0.44147372, 0.2347359, 0.0],
        tex_coords: [0.9414737, 0.2652641],
        color: [1.0, 1.0, 1.0],
    },
];

const INDICES: &[[u32; 3]] = &[[0, 1, 4], [1, 2, 4], [2, 3, 4]];

struct State {
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,

    render_pipeline: wgpu::RenderPipeline,

    mesh: mesh::Mesh,

    diffuse_bind_group: wgpu::BindGroup,
    _diffuse_texture: texture::Texture,

    depth_texture: texture::Texture,

    player: Player,
    chunk00: Chunk,

    projection: Projection,
    camera_uniform: CameraUniform,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,

    input: InputManager,
}

impl State {
    async fn new(window: &Window) -> Self {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(wgpu::Backends::all());
        let surface = unsafe { instance.create_surface(window) };
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    features: wgpu::Features::empty(),
                    limits: wgpu::Limits::default(),
                    label: None,
                },
                None,
            )
            .await
            .unwrap();

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface.get_preferred_format(&adapter).unwrap(),
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
        };
        surface.configure(&device, &config);

        let diffuse_texture = texture::Texture::from_bytes(
            &device,
            &queue,
            include_bytes!("../res/images/tree.png"),
            Some("Happy Tree Texture"),
        )
        .unwrap();
        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
                        ty: wgpu::BindingType::Sampler {
                            comparison: false,
                            filtering: true,
                        },
                        count: None,
                    },
                ],
                label: Some("Texture Bind Group Layout"),
            });
        let diffuse_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&diffuse_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&diffuse_texture.sampler),
                },
            ],
            label: Some("Happy Tree Bind Group"),
        });

        let depth_texture =
            texture::Texture::create_depth_texture(&device, &config, Some("Depth Texture"));

        let shader = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        let mesh = mesh::Mesh::new(
            Some("Mesh".to_owned()),
            VERTICES.to_vec(),
            INDICES.to_vec(),
            vec![
                mesh::Instance {
                    position: cgmath::Point3::new(-0.7, 0.0, -1.5),
                    rotation: cgmath::Quaternion::from_angle_y(cgmath::Deg(60.0)),
                },
                mesh::Instance {
                    position: cgmath::Point3::new(0.0, 0.0, -2.0),
                    rotation: cgmath::Quaternion::from_angle_y(cgmath::Deg(0.0)),
                },
                mesh::Instance {
                    position: cgmath::Point3::new(0.7, 0.0, -1.5),
                    rotation: cgmath::Quaternion::from_angle_y(cgmath::Deg(-60.0)),
                },
            ],
        );
        // let instances = vec![
        //     Instance {
        //         position: cgmath::Point3::new(-0.7, 0.0, -1.5),
        //         rotation: cgmath::Quaternion::from_angle_y(cgmath::Deg(60.0)),
        //     },
        //     Instance {
        //         position: cgmath::Point3::new(0.0, 0.0, -2.0),
        //         rotation: cgmath::Quaternion::from_angle_y(cgmath::Deg(0.0)),
        //     },
        //     Instance {
        //         position: cgmath::Point3::new(0.7, 0.0, -1.5),
        //         rotation: cgmath::Quaternion::from_angle_y(cgmath::Deg(-60.0)),
        //     },
        // ];
        // let instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        //     label: Some("Instance Buffer"),
        //     contents: bytemuck::cast_slice(
        //         &instances.iter().map(Instance::to_raw).collect::<Vec<_>>(),
        //     ),
        //     usage: wgpu::BufferUsages::VERTEX,
        // });
        // let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        //     label: Some("Vertex Buffer"),
        //     contents: bytemuck::cast_slice(VERTICES),
        //     usage: wgpu::BufferUsages::VERTEX,
        // });
        // let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        //     label: Some("Index Buffer"),
        //     contents: bytemuck::cast_slice(INDICES),
        //     usage: wgpu::BufferUsages::INDEX,
        // });
        // let num_indices = 3 * INDICES.len() as u32;

        let player = Player::new(4.0, 3.5, 0.01);
        let projection = Projection::new(
            size.width,
            size.height,
            cgmath::Deg(60.0).into(),
            0.125,
            1024.0,
        );

        let camera_uniform = CameraUniform {
            model: (projection.calc_matrix() * player.camera().calc_matrix()).into(),
        };
        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&[camera_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Camera Bind Group Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });
        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Camera Bind Group"),
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&texture_bind_group_layout, &camera_bind_group_layout],
                push_constant_ranges: &[],
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[mesh::Vertex::desc(), mesh::InstanceRaw::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                }],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                clamp_depth: false,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: texture::Texture::DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
        });

        Self {
            surface,
            device,
            queue,
            config,
            size,
            render_pipeline,

            // vertex_buffer,
            // index_buffer,
            // num_indices,
            // instances,
            // instance_buffer,
            mesh,

            diffuse_bind_group,
            _diffuse_texture: diffuse_texture,

            depth_texture,

            player,
            chunk00: Chunk::floor(0, 0, 0),

            projection,
            camera_uniform,
            camera_buffer,
            camera_bind_group,

            input: InputManager::new(),
        }
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if 0 < new_size.width && 0 < new_size.height {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);

            self.projection.resize(new_size.width, new_size.height);
            self.camera_uniform.model =
                (self.projection.calc_matrix() * self.player.camera().calc_matrix()).into();
            self.queue.write_buffer(
                &self.camera_buffer,
                0,
                bytemuck::cast_slice(&[self.camera_uniform]),
            );

            self.depth_texture = texture::Texture::create_depth_texture(
                &self.device,
                &self.config,
                Some("Depth Texture"),
            );
        }
    }

    fn input(&mut self, event: &ProcEvent) -> bool {
        self.input.input(event);

        match event {
            ProcEvent::Window(event) => match event {
                _ => false,
            },
            ProcEvent::User(event) => match event {
                _ => false,
            },
        }
    }

    fn update(&mut self) {
        self.player.update(&self.input);

        self.camera_uniform.model =
            (self.projection.calc_matrix() * self.player.camera().calc_matrix()).into();
        self.queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&[self.camera_uniform]),
        );

        self.input.update();
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.0,
                        g: 0.0,
                        b: 0.0,
                        a: 1.0,
                    }),
                    store: true,
                },
            }],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.depth_texture.view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: true,
                }),
                stencil_ops: None,
            }),
        });

        render_pass.set_pipeline(&self.render_pipeline);

        render_pass.set_bind_group(0, &self.diffuse_bind_group, &[]);
        render_pass.set_bind_group(1, &self.camera_bind_group, &[]);

        self.mesh.draw(&mut self.device, &mut render_pass);
        self.chunk00.draw(
            &mut self.device,
            &mut render_pass,
            chunk::Faces::ALL,
            chunk::faced_data![None; reeval 6],
            // &[vec![
            //     chunk::BlockTypeData {
            //         tex_coords0: [[0.0, 1.0]; 6],
            //         tex_coords1: [[1.0, 0.0]; 6],
            //         is_transparent: true,
            //     },
            //     chunk::BlockTypeData {
            //         tex_coords0: [[0.0, 1.0]; 6],
            //         tex_coords1: [[1.0, 0.0]; 6],
            //         is_transparent: false,
            //     },
            // ]],
        );
        // render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
        // render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        // render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);

        // render_pass.draw_indexed(0..self.num_indices, 0, 0..self.instances.len() as _);

        drop(render_pass);

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}

fn main() {
    env_logger::init();

    let event_loop = EventLoop::<UserEvent>::with_user_event();
    let window = WindowBuilder::new().build(&event_loop).unwrap();
    window.set_cursor_grab(true).unwrap();
    window.set_cursor_visible(false);

    let mut state = block_on(State::new(&window));

    event_loop.run(move |event, _, control_flow| match event {
        Event::WindowEvent {
            ref event,
            window_id,
        } if window_id == window.id() => {
            if !state.input(&ProcEvent::Window(event)) {
                match event {
                    WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                    WindowEvent::Resized(new_size) => {
                        state.resize(*new_size);
                    }
                    WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                        state.resize(**new_inner_size);
                    }
                    _ => {}
                }
            }
        }
        Event::RedrawRequested(_) => {
            state.update();
            window
                .set_cursor_position(winit::dpi::PhysicalPosition::new(
                    state.size.width / 2,
                    state.size.height / 2,
                ))
                .unwrap();
            state
                .input
                .set_cursor_position(winit::dpi::PhysicalPosition::new(
                    state.size.width as f64 / 2.0,
                    state.size.height as f64 / 2.0,
                ));
            match state.render() {
                Ok(()) => {}
                Err(wgpu::SurfaceError::Lost) => state.resize(state.size),
                Err(wgpu::SurfaceError::OutOfMemory) => *control_flow = ControlFlow::Exit,
                Err(e) => eprintln!("{:?}", e),
            }
        }
        Event::MainEventsCleared => {
            window.request_redraw();
        }
        Event::UserEvent(event) => {
            if !state.input(&ProcEvent::User(&event)) {
                match event {}
            }
        }
        _ => {}
    });
}
