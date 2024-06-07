use std::{iter, mem, sync::Arc};

use anyhow::Result;
use camera::{Camera, CameraController, CameraUniform};
use futures::executor::block_on;
use texture::Texture;
use wgpu::util::DeviceExt;
use winit::{
    application::ApplicationHandler,
    event::*,
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowId},
};

mod camera;
mod texture;

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct InstanceRaw {
    model: glam::Mat4,
}

#[derive(Copy, Clone)]
struct Instance {
    scale: glam::Vec3,
    position: glam::Vec3,
    rotation: glam::Quat,
}

impl Instance {
    fn to_raw(&self) -> InstanceRaw {
        InstanceRaw {
            model: glam::Mat4::from_scale_rotation_translation(
                self.scale,
                self.rotation,
                self.position,
            ),
        }
    }

    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        const ATTRS: &[wgpu::VertexAttribute] = &wgpu::vertex_attr_array![
            5 => Float32x4,
            6 => Float32x4,
            7 => Float32x4,
            8 => Float32x4,
        ];
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<InstanceRaw>() as _,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: ATTRS,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: glam::Vec3,
    tex_coords: glam::Vec2,
}

impl Vertex {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        const ATTRS: &[wgpu::VertexAttribute] = &wgpu::vertex_attr_array![
            0 => Float32x3,
            1 => Float32x2
        ];
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Vertex>() as _,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: ATTRS,
        }
    }
}

const VERTICES: &[Vertex] = &[
    Vertex {
        position: glam::vec3(-0.0868241, 0.49240386, 0.0),
        tex_coords: glam::vec2(0.4131759, 0.00759614),
    }, // A
    Vertex {
        position: glam::vec3(-0.49513406, 0.06958647, 0.0),
        tex_coords: glam::vec2(0.0048659444, 0.43041354),
    }, // B
    Vertex {
        position: glam::vec3(-0.21918549, -0.44939706, 0.0),
        tex_coords: glam::vec2(0.28081453, 0.949397),
    }, // C
    Vertex {
        position: glam::vec3(0.35966998, -0.3473291, 0.0),
        tex_coords: glam::vec2(0.85967, 0.84732914),
    }, // D
    Vertex {
        position: glam::vec3(0.44147372, 0.2347359, 0.0),
        tex_coords: glam::vec2(0.9414737, 0.2652641),
    }, // E
];

const INDICES: &[u16] = &[0, 1, 4, 1, 2, 4, 2, 3, 4];

struct GraphicsState {
    size: winit::dpi::PhysicalSize<u32>,
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    render_pipeline: wgpu::RenderPipeline,

    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    num_indices: u32,

    instances: Vec<Instance>,
    instance_buffer: wgpu::Buffer,

    diffuse_bind_group: wgpu::BindGroup,
    #[allow(dead_code)]
    diffuse_texture: Texture,

    camera: Camera,
    camera_uniform: CameraUniform,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
}

impl GraphicsState {
    pub async fn new(window: Arc<Window>) -> Self {
        let size = window.inner_size();
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });

        let surface = instance.create_surface(window.clone()).unwrap();

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
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    label: None,
                },
                None,
            )
            .await
            .unwrap();

        let surface_caps = surface.get_capabilities(&adapter);

        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        let diffuse_texture = Texture::from_bytes(
            &device,
            &queue,
            include_bytes!("../res/images/tree.png"),
            "Diffuse",
        )
        .unwrap();

        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Texture Bind Group Layout"),
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
            });
        let diffuse_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Texture Bind Group"),
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
        });

        let camera = Camera {
            // position the camera 1 unit up and 2 units back
            // +z is out of the screen
            pos: glam::vec3(0.0, 0.0, 2.0),
            rot: glam::vec2(0.0, 0.0),
            aspect: config.width as f32 / config.height as f32,
            fovy: 45.0,
            znear: 0.1,
            zfar: 100.0,
        };
        let mut camera_uniform = CameraUniform::new();
        camera_uniform.update_view_proj(&camera);
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

        let shader = device.create_shader_module(wgpu::include_wgsl!("shader.wgsl"));
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
                buffers: &[Vertex::desc(), Instance::desc()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let num_indices = INDICES.len() as u32;
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });

        const NUM_INSTANCES_PER_ROW: u32 = 10;
        const INSTANCE_DISPLACEMENT: glam::Vec3 = glam::vec3(
            NUM_INSTANCES_PER_ROW as f32 * 0.5,
            0.0,
            NUM_INSTANCES_PER_ROW as f32 * 0.5,
        );

        let instances = (0..NUM_INSTANCES_PER_ROW)
            .flat_map(|z| {
                (0..NUM_INSTANCES_PER_ROW).map(move |x| {
                    let position = glam::vec3(x as _, 0.0, z as _) - INSTANCE_DISPLACEMENT;

                    let rotation = if position == glam::Vec3::ZERO {
                        // this is needed so an object at (0, 0, 0) won't get scaled to zero
                        // as Quaternions can affect scale if they're not created correctly
                        glam::Quat::IDENTITY
                    } else {
                        glam::Quat::from_axis_angle(position.normalize(), 45f32.to_radians())
                    };

                    Instance {
                        scale: glam::Vec3::ONE,
                        position,
                        rotation,
                    }
                })
            })
            .collect::<Vec<_>>();
        let instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Instance Buffer"),
            contents: bytemuck::cast_slice(
                &instances.iter().map(Instance::to_raw).collect::<Vec<_>>(),
            ),
            usage: wgpu::BufferUsages::VERTEX,
        });

        Self {
            size,
            window,
            surface,
            device,
            queue,
            config,
            render_pipeline,

            vertex_buffer,
            index_buffer,
            num_indices,

            instances,
            instance_buffer,

            diffuse_bind_group,
            diffuse_texture,

            camera,
            camera_uniform,
            camera_buffer,
            camera_bind_group,
        }
    }

    pub fn update_window(&mut self, window: Arc<Window>) {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });
        self.window = window;
        self.surface = instance.create_surface(self.window.clone()).unwrap();
        self.resize(self.window.inner_size());
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width == 0 || new_size.height == 0 {
            tracing::warn!("Ignoring resize event with 0 width or height");
            return;
        }

        self.size = new_size;
        self.config.width = new_size.width;
        self.config.height = new_size.height;
        self.surface.configure(&self.device, &self.config);

        self.camera.aspect = new_size.width as f32 / new_size.height as f32;
        self.update_camera_uniform();
    }

    pub fn update_camera_uniform(&mut self) {
        self.camera_uniform.update_view_proj(&self.camera);
        self.queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&[self.camera_uniform]),
        );
    }

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
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
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.1,
                        g: 0.2,
                        b: 0.3,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
        });
        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_bind_group(0, &self.diffuse_bind_group, &[]);
        render_pass.set_bind_group(1, &self.camera_bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        render_pass.draw_indexed(0..self.num_indices, 0, 0..self.instances.len() as _);
        drop(render_pass);

        self.queue.submit(iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}

struct App {
    graphics: Option<GraphicsState>,
    camera_controller: CameraController,
}

impl App {
    fn new() -> Self {
        Self {
            graphics: None,
            camera_controller: CameraController::new(0.03, 0.4),
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = Arc::new(
            event_loop
                .create_window(Window::default_attributes().with_title("mclone"))
                .unwrap(),
        );
        window
            .set_cursor_grab(winit::window::CursorGrabMode::Locked)
            .unwrap();
        window.set_cursor_visible(false);

        match &mut self.graphics {
            Some(graphics) => graphics.update_window(window.clone()),
            None => self.graphics = Some(block_on(GraphicsState::new(window))),
        }
    }

    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device_id: DeviceId,
        event: DeviceEvent,
    ) {
        let Some(graphics) = &mut self.graphics else {
            tracing::warn!("Ignoring device event without graphics state");
            return;
        };
        if self
            .camera_controller
            .device_event(&event, &mut graphics.camera)
        {
            return;
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let Some(graphics) = &mut self.graphics else {
            tracing::warn!("Ignoring window event without graphics state");
            return;
        };
        if graphics.window.id() != window_id {
            return;
        }
        if self.camera_controller.window_event(&event) {
            return;
        }
        match event {
            WindowEvent::CloseRequested
            | WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(KeyCode::Escape),
                        state: ElementState::Pressed,
                        ..
                    },
                ..
            } => {
                event_loop.exit();
            }
            WindowEvent::Resized(size) => {
                graphics.resize(size);
            }
            WindowEvent::ScaleFactorChanged { .. } => {
                graphics.resize(graphics.window.inner_size());
            }
            WindowEvent::RedrawRequested => {
                use std::sync::atomic::AtomicU64;
                static FRAME_COUNTER: AtomicU64 = AtomicU64::new(0);

                self.camera_controller.update_camera(&mut graphics.camera);
                graphics.update_camera_uniform();

                let frame = FRAME_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                if frame % 60 == 0 {
                    tracing::info!(
                        "#{frame}: camera.pos={:?} camera.rot={:?}",
                        graphics.camera.pos,
                        graphics.camera.rot,
                    );
                }

                match graphics.render() {
                    Ok(()) => {}
                    Err(wgpu::SurfaceError::Lost) => graphics.resize(graphics.size),
                    Err(wgpu::SurfaceError::OutOfMemory) => event_loop.exit(),
                    Err(e) => tracing::warn!("Encountered surface error: {e}"),
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        let Some(graphics) = &mut self.graphics else {
            return;
        };
        graphics.window.request_redraw();
    }
}

fn main() -> Result<()> {
    tracing_subscriber::fmt().init();

    let event_loop = EventLoop::new()?;
    event_loop.run_app(&mut App::new())?;

    Ok(())
}
