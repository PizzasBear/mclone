use ahash::HashMap;
use anyhow::*;
use wgpu::util::DeviceExt;

use crate::texture::Texture;

mod chunk;

use chunk::{Chunk, ChunkInstance};

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

pub struct BlockData {
    pub name: String,
    pub mesh_type: BlockMeshType,
}

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
        let loaded_chunks = vec![Chunk::generate(glam::IVec3::ZERO)];
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
                    offset: glam::vec3(-16.0, -20.0, -16.0),
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
            render_pass.draw_indexed(0..(index_buffer.size() / 4) as _, 0, 0..1);
        }
    }
}
