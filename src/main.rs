use std::{iter, sync::Arc, time::Instant};

use anyhow::Result;
use wgpu::util::DeviceExt;
use winit::{
    application::ApplicationHandler,
    event::*,
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowId},
};

use camera::{Camera, CameraController, CameraUniform};
use model::VertexBuffer;
use texture::Texture;

mod camera;
mod model;
mod texture;

macro_rules! inline_const {
    (($expr:expr) as $ty:ty) => {{
        const C: $ty = $expr;
        C
    }};
    ($block:block as $ty:ty) => {
        inline_const!(($block) as $ty)
    };
    ([$($tt:tt)*] as $ty:ty) => {
        inline_const!(([$($tt)*]) as $ty)
    };
    (&[$($tt:tt)*] as $ty:ty) => {
        inline_const!((&[$($tt)*]) as $ty)
    };
    ($lit:literal as $ty:ty) => {
        inline_const!(($lit) as $ty)
    };
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct LightUniform {
    dir: glam::Vec3,
    _pad1: u32,
    color: glam::Vec3,
    _pad2: u32,
}

struct GraphicsState {
    size: winit::dpi::PhysicalSize<u32>,
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    render_pipeline: wgpu::RenderPipeline,
    light_render_pipeline: wgpu::RenderPipeline,

    depth_texture: Texture,

    camera: Camera,
    camera_uniform: CameraUniform,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,

    light_uniform: LightUniform,
    light_buffer: wgpu::Buffer,
    light_bind_group: wgpu::BindGroup,

    model: model::Model,
    instance_buffer: wgpu::Buffer,
    num_instances: u32,
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
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
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

        let material_layout = model::Material::create_bind_group_layout(&device);

        let light_uniform = LightUniform {
            dir: glam::Quat::from_rotation_x(20f32.to_radians()) * glam::Vec3::NEG_Y,
            color: glam::vec3(1.0, 1.0, 1.0),
            _pad1: 0,
            _pad2: 0,
        };
        let light_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Light Buffer"),
            contents: bytemuck::cast_slice(&[light_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let light_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Light Bind Group Layout"),
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
        let light_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Light Bind Group"),
            layout: &light_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: light_buffer.as_entire_binding(),
            }],
        });

        let depth_texture = Texture::create_depth_texture(&device, &config, "Depth Texture");

        let create_render_pipeline = |label, layout, buffers, shader| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: "vs_main",
                    buffers,
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
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: Texture::DEPTH_FORMAT,
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
                multiview: None,
            })
        };

        let render_pipeline = create_render_pipeline(
            "Render Pipeline",
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[
                    &material_layout,
                    &camera_bind_group_layout,
                    &light_bind_group_layout,
                ],
                push_constant_ranges: &[],
            }),
            inline_const!(
                &[model::Vertex::DESC, model::Instance::DESC] as &[wgpu::VertexBufferLayout]
            ),
            device.create_shader_module(wgpu::include_wgsl!("shader.wgsl")),
        );
        let light_render_pipeline = create_render_pipeline(
            "Light Render Pipeline",
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Light Render Pipeline Layout"),
                bind_group_layouts: &[&camera_bind_group_layout, &light_bind_group_layout],
                push_constant_ranges: &[],
            }),
            inline_const!(
                &[model::Vertex::DESC, model::Instance::DESC] as &[wgpu::VertexBufferLayout]
            ),
            device.create_shader_module(wgpu::include_wgsl!("light.wgsl")),
        );

        let model = model::Model::load("res/models/monkey.obj", &device, &queue, &material_layout)
            .await
            .unwrap();

        const NUM_INSTANCES_PER_ROW: u32 = 10;
        const SPACE_BETWEEN: f32 = 3.0;
        let instances = (0..NUM_INSTANCES_PER_ROW)
            .flat_map(|z| {
                (0..NUM_INSTANCES_PER_ROW).map(move |x| {
                    let x = SPACE_BETWEEN * (x as f32 - NUM_INSTANCES_PER_ROW as f32 / 2.0);
                    let z = SPACE_BETWEEN * (z as f32 - NUM_INSTANCES_PER_ROW as f32 / 2.0);

                    let position = glam::Vec3::new(x, -2.0, z);

                    let rotation = match position.try_normalize() {
                        Some(dir) => glam::Quat::from_axis_angle(dir, 45f32.to_radians()),
                        None => glam::Quat::IDENTITY,
                    };

                    model::Instance {
                        position,
                        rotation,
                        scale: glam::Vec3::ONE,
                    }
                })
            })
            .map(VertexBuffer::into_raw)
            .collect::<Vec<_>>();
        let instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Instance Buffer"),
            contents: bytemuck::cast_slice(&instances),
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
            light_render_pipeline,

            depth_texture,

            camera,
            camera_uniform,
            camera_buffer,
            camera_bind_group,

            light_uniform,
            light_buffer,
            light_bind_group,

            model,
            instance_buffer,
            num_instances: instances.len() as _,
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

        self.depth_texture =
            Texture::create_depth_texture(&self.device, &self.config, "Depth Texture");

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
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.depth_texture.view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            occlusion_query_set: None,
            timestamp_writes: None,
        });
        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_bind_group(1, &self.camera_bind_group, &[]);
        render_pass.set_bind_group(2, &self.light_bind_group, &[]);

        render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
        self.model.draw(&mut render_pass, .., 0..self.num_instances);

        render_pass.set_pipeline(&self.light_render_pipeline);
        render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
        render_pass.set_bind_group(1, &self.light_bind_group, &[]);

        self.model.meshes[0].draw(&mut render_pass, 0..1);

        drop(render_pass);

        self.queue.submit(iter::once(encoder.finish()));

        output.present();

        Ok(())
    }
}

struct App {
    rt: tokio::runtime::Runtime,
    graphics: Option<GraphicsState>,
    camera_controller: CameraController,

    last_render_time: Instant,
}

impl App {
    fn new() -> Self {
        Self {
            rt: tokio::runtime::Builder::new_current_thread()
                .build()
                .unwrap(),
            graphics: None,
            camera_controller: CameraController::new(12., 20.),
            last_render_time: Instant::now(),
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
            None => self.graphics = Some(self.rt.block_on(GraphicsState::new(window))),
        }
    }

    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device_id: DeviceId,
        event: DeviceEvent,
    ) {
        if self.camera_controller.device_event(&event) {
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
                let delta_time = self.last_render_time.elapsed().as_secs_f32();
                self.last_render_time = Instant::now();

                self.camera_controller
                    .update_camera(delta_time, &mut graphics.camera);
                graphics.update_camera_uniform();

                // graphics.light_uniform.dir =
                //     glam::Quat::from_rotation_y(1f32.to_radians()) * graphics.light_uniform.dir;
                graphics.queue.write_buffer(
                    &graphics.light_buffer,
                    0,
                    bytemuck::cast_slice(&[graphics.light_uniform]),
                );

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
