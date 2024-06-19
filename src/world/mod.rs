use ahash::HashMap;
use anyhow::*;
use wgpu::util::DeviceExt;
use winit::event::*;

use crate::{camera::Camera, texture::Texture};

mod chunk;

pub use chunk::{BlockFace, Chunk};

use chunk::ChunkInstance;

#[derive(Debug, Copy, Clone)]
pub struct BlockTexture {
    pub pos: glam::Vec2,
    pub size: glam::Vec2,

    pub color: image::Rgba<u8>,
}

impl BlockTexture {
    pub fn new(pos: glam::Vec2, size: glam::Vec2) -> Self {
        Self {
            pos,
            size,
            color: [0; 4].into(),
        }
    }

    pub fn with_color(mut self, color: image::Rgba<u8>) -> Self {
        self.color = color;
        self
    }

    pub fn get(&self, coords: glam::Vec2) -> glam::Vec2 {
        self.pos + coords * self.size
    }
}

#[derive(Debug, Copy, Clone)]
pub enum BlockMeshType {
    Transparent,
    SameSided(BlockTexture),
    Surrounded {
        top: BlockTexture,
        bottom: BlockTexture,
        sides: BlockTexture,
    },
    Directional {
        right: BlockTexture,
        left: BlockTexture,
        top: BlockTexture,
        bottom: BlockTexture,
        front: BlockTexture,
        back: BlockTexture,
    },
}

#[derive(Debug)]
pub struct BlockData {
    pub name: String,
    pub mesh_type: BlockMeshType,
}

impl BlockData {
    fn is_transparent(&self) -> bool {
        matches!(self.mesh_type, BlockMeshType::Transparent)
    }
}

#[derive(Debug)]
pub struct BlockRegistry {
    pub texture: Texture,
    pub bind_group: wgpu::BindGroup,
    pub blocks: Vec<BlockData>,
    pub block_map: HashMap<String, u32>,
}

impl BlockRegistry {
    pub fn create_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Block Registry Bind Group Layout"),
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
}

pub struct World {
    pub registry: BlockRegistry,
    pub loaded_chunks: Vec<Chunk>,
    pub instance_buffer: Option<wgpu::Buffer>,
    pub render_pipeline: wgpu::RenderPipeline,
}

impl World {
    pub async fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        config: &wgpu::SurfaceConfiguration,
        camera_bind_group_layout: &wgpu::BindGroupLayout,
        light_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> Result<Self> {
        let size = glam::vec2(16., 16.) / 1024.0;
        let blocks = vec![
            BlockData {
                name: "air".to_owned(),
                mesh_type: BlockMeshType::Transparent,
            },
            BlockData {
                name: "cobblestone".to_owned(),
                mesh_type: BlockMeshType::SameSided(BlockTexture::new(
                    glam::vec2(624.0, 272.0) / 1024.0,
                    size,
                )),
            },
            BlockData {
                name: "dirt".to_owned(),
                mesh_type: BlockMeshType::SameSided(BlockTexture::new(
                    glam::vec2(768.0, 304.0) / 1024.0,
                    size,
                )),
            },
            BlockData {
                name: "grass".to_owned(),
                mesh_type: BlockMeshType::Surrounded {
                    top: BlockTexture::new(glam::vec2(880.0, 320.0) / 1024.0, size)
                        .with_color(0x97c667_ffu32.to_be_bytes().into()),
                    bottom: BlockTexture::new(glam::vec2(768.0, 304.0) / 1024.0, size),
                    sides: BlockTexture::new(glam::vec2(832.0, 320.0) / 1024.0, size),
                },
            },
            BlockData {
                name: "furnace".to_owned(),
                mesh_type: BlockMeshType::Directional {
                    right: BlockTexture::new(glam::vec2(656.0, 320.0) / 1024.0, size),
                    left: BlockTexture::new(glam::vec2(656.0, 320.0) / 1024.0, size),
                    top: BlockTexture::new(glam::vec2(672.0, 320.0) / 1024.0, size),
                    bottom: BlockTexture::new(glam::vec2(672.0, 320.0) / 1024.0, size),
                    front: BlockTexture::new(glam::vec2(624.0, 320.0) / 1024.0, size),
                    back: BlockTexture::new(glam::vec2(656.0, 320.0) / 1024.0, size),
                },
            },
            BlockData {
                name: "observer".to_owned(),
                mesh_type: BlockMeshType::Directional {
                    right: BlockTexture::new(glam::vec2(816.0, 368.0) / 1024.0, size),
                    left: BlockTexture::new(glam::vec2(816.0, 368.0) / 1024.0, size),
                    top: BlockTexture::new(glam::vec2(832.0, 368.0) / 1024.0, size),
                    bottom: BlockTexture::new(glam::vec2(832.0, 368.0) / 1024.0, size),
                    front: BlockTexture::new(glam::vec2(800.0, 368.0) / 1024.0, size),
                    back: BlockTexture::new(glam::vec2(768.0, 368.0) / 1024.0, size),
                },
            },
        ];
        let texture = Texture::load(
            device,
            queue,
            "res/images/minecraft_textures_block_atlas.png",
            false,
            "Block Atlas",
        )
        .await?;
        let registry_bind_group_layout = BlockRegistry::create_bind_group_layout(device);
        let registry = BlockRegistry {
            bind_group: device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &registry_bind_group_layout,
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
                label: Some("Block Bind Group"),
            }),
            texture,
            block_map: (blocks.iter().enumerate())
                .map(|(i, b)| (b.name.clone(), i as _))
                .collect(),
            blocks,
        };
        let loaded_chunks = vec![Chunk::generate(glam::ivec3(0, -1, 0))];
        Ok(Self {
            registry,
            loaded_chunks,
            instance_buffer: None,
            render_pipeline: Chunk::create_render_pipeline(
                &device,
                &config,
                &registry_bind_group_layout,
                &camera_bind_group_layout,
                &light_bind_group_layout,
            ),
        })
    }

    pub fn raycast(
        &self,
        origin: glam::Vec3,
        direction: glam::Vec3,
        max_distance: f32,
    ) -> Option<(usize, usize, BlockFace)> {
        let mut pos = origin;
        let mut ipos = pos.floor().as_ivec3();

        fn dist_to_whole(pos: f32, dir: f32) -> f32 {
            if dir == 0.0 {
                f32::INFINITY
            } else if 0.0 < dir {
                (pos.floor() + 1.0 - pos) / dir + f32::EPSILON
            } else {
                (pos.ceil() - 1.0 - pos) / dir + f32::EPSILON
            }
        }

        loop {
            // tracing::info!("Raycast: pos={pos}  ipos={ipos}  dir={direction}");
            let dx = dist_to_whole(pos.x, direction.x);
            let dy = dist_to_whole(pos.y, direction.y);
            let dz = dist_to_whole(pos.z, direction.z);

            let block_face;
            if dx < dy && dx < dz {
                // tracing::info!("Raycast[dx]:  dx={dx}  dy={dy}  dz={dz}");
                pos += direction * dx;
                ipos.x += direction.x.signum() as i32;
                block_face = match 0.0 < direction.x {
                    true => BlockFace::Left,
                    false => BlockFace::Right,
                };
            } else if dy < dz {
                // tracing::info!("Raycast[dy]:  dx={dx}  dy={dy}  dz={dz}");
                pos += direction * dy;
                ipos.y += direction.y.signum() as i32;
                block_face = match 0.0 < direction.y {
                    true => BlockFace::Bottom,
                    false => BlockFace::Top,
                };
            } else {
                // tracing::info!("Raycast[dz]:  dx={dx}  dy={dy}  dz={dz}");
                pos += direction * dz;
                ipos.z += direction.z.signum() as i32;
                block_face = match 0.0 < direction.z {
                    true => BlockFace::Front,
                    false => BlockFace::Back,
                };
            }

            if max_distance * max_distance < origin.distance_squared(pos) {
                // tracing::info!("Raycast[max distance reached at]:  origin={origin} pos={pos}");
                break None;
            }

            let chunk_pos = ipos.div_euclid(32 * glam::IVec3::ONE);
            let Some((chunk_i, chunk)) = self
                .loaded_chunks
                .iter()
                .enumerate()
                .find(|(_, c)| c.pos == chunk_pos)
            else {
                continue;
            };
            let block_i = Chunk::block_pos_to_idx((ipos - 32 * chunk_pos).as_uvec3());
            let block = &chunk.blocks[block_i];

            if block.id != 0 {
                // tracing::info!(
                //     "Raycast[hit block]:  pos={pos}  ipos={ipos}  chunk_pos={chunk_pos}"
                // );
                break Some((chunk_i, block_i, block_face));
            }
        }
    }

    pub fn draw<'a>(
        &'a mut self,
        device: &wgpu::Device,
        render_pass: &mut wgpu::RenderPass<'a>,
        camera_bind_group: &'a wgpu::BindGroup,
        light_bind_group: &'a wgpu::BindGroup,
    ) {
        let instance_buffer = self.instance_buffer.get_or_insert_with(|| {
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Chunk Instance Buffer"),
                contents: bytemuck::cast_slice(&[ChunkInstance {
                    offset: glam::vec3(0.0, -32.0, 0.0),
                }]),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            })
        });

        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_bind_group(0, &self.registry.bind_group, &[]);
        render_pass.set_bind_group(1, camera_bind_group, &[]);
        render_pass.set_bind_group(2, light_bind_group, &[]);
        render_pass.set_vertex_buffer(1, instance_buffer.slice(..));
        for chunk in &mut self.loaded_chunks {
            if chunk.vertex_buffer.is_none() {
                chunk.gen_mesh(device, &self.registry);
            }
            render_pass.set_vertex_buffer(0, chunk.vertex_buffer.as_ref().unwrap().slice(..));
            let index_buffer = chunk.index_buffer.as_ref().unwrap();
            render_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            render_pass.draw_indexed(0..(6 * chunk.vertices.len()) as _, 0, 0..1);
        }
    }

    pub fn window_event(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        cam: &Camera,
        event: &WindowEvent,
    ) -> bool {
        match event {
            WindowEvent::MouseInput {
                button: MouseButton::Right,
                state: ElementState::Pressed,
                ..
            } => {
                // tracing::info!("Button event: {:?}", event);
                if let Some((chunk_i, block_i, face)) = self.raycast(cam.pos, cam.dir(), 6.0) {
                    let Some(block_i) = block_i.checked_add_signed(face.ioffset()) else {
                        return true;
                    };
                    self.loaded_chunks[chunk_i].place_block(
                        device,
                        queue,
                        &self.registry,
                        block_i,
                        1,
                        face,
                    );
                }
                true
            }
            WindowEvent::MouseInput {
                button: MouseButton::Left,
                state: ElementState::Pressed,
                ..
            } => {
                // tracing::info!("Button event: {:?}", event);
                if let Some((chunk_i, block_i, face)) = self.raycast(cam.pos, cam.dir(), 6.0) {
                    self.loaded_chunks[chunk_i].place_block(
                        device,
                        queue,
                        &self.registry,
                        block_i,
                        0,
                        face,
                    );
                }
                true

                // tracing::info!(
                //     "Raycast hit: chunk={chunk_i}:{}  block={block_i}:{}  face={:?}",
                //     chunk.pos,
                //     Chunk::block_idx_to_pos(block_i),
                //     face,
                // );
                // tracing::info!("Raycast Block Data: {block:?}");
                // let Some(mesh_idx) = block.face(face) else {
                //     tracing::info!("Raycast: Block face doesn't exists: {face:?}");
                //     return false;
                // };
                // println!("Mesh index: {}", mesh_idx);

                // let grass = &self.registry.blocks[self.registry.block_map["grass"] as usize];
                // let grass_top = match grass.mesh_type {
                //     BlockMeshType::Surrounded { top, .. } => top,
                //     _ => unimplemented!(),
                // };

                // let vertices = block.gen_face(
                //     &self.registry,
                //     Chunk::block_idx_to_pos(block_i).as_vec3(),
                //     face,
                // );
                // let bottom_tex_coords = (vertices.iter().map(|v| v.tex_coords))
                //     .min_by_key(|c| (c.element_sum() * 1024.0) as u32)
                //     .unwrap();
                // let top_tex_coords = (vertices.iter().map(|v| v.tex_coords))
                //     .max_by_key(|c| (c.element_sum() * 1024.0) as u32)
                //     .unwrap();
                // let size = top_tex_coords - bottom_tex_coords;
                // let vertices = vertices.map(|v| chunk::Vertex {
                //     tex_coords: grass_top.get((v.tex_coords - bottom_tex_coords) / size),
                //     color: 0xff7777ff_u32.to_be_bytes(),
                //     ..v
                // });

                // queue.write_buffer(
                //     &chunk.vertex_buffer.as_ref().unwrap(),
                //     (mem::size_of_val(&vertices) * mesh_idx) as _,
                //     bytemuck::cast_slice(&vertices),
                // );
            }
            _ => false,
        }
    }
}
