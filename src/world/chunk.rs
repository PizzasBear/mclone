use std::{iter, mem};

use rand::prelude::*;
use wgpu::util::DeviceExt;

use crate::graphics::{self, VertexBuffer};

use super::{BlockData, BlockMeshType, BlockRegistry};

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ChunkInstance {
    pub offset: glam::Vec3,
}

impl VertexBuffer for ChunkInstance {
    type Raw = Self;
    fn to_raw(&self) -> Self {
        *self
    }

    const DESC: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: mem::size_of::<Self::Raw>() as _,
        step_mode: wgpu::VertexStepMode::Instance,
        attributes: &wgpu::vertex_attr_array![
            5 => Float32x3,
        ],
    };
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: glam::Vec3,
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
            0 => Float32x3, // position
            1 => Float32x2, // tex_coords
            2 => Unorm8x4, // color
        ],
    };
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, bytemuck::Zeroable)]
#[repr(u8)]
pub enum BlockFace {
    Back,
    Front,
    Right,
    Left,
    Top,
    Bottom,
}

impl BlockFace {
    const unsafe fn from_u8_unchecked(value: u8) -> Self {
        debug_assert!(value < 6);
        mem::transmute(value)
    }

    pub fn iter(
    ) -> impl Iterator<Item = Self> + DoubleEndedIterator + ExactSizeIterator + iter::FusedIterator + Clone
    {
        (0..6u8).map(|i| unsafe { Self::from_u8_unchecked(i) })
    }

    pub fn ioffset(self) -> isize {
        match self {
            Self::Right => 1,
            Self::Left => -1,
            Self::Top => 1 << 5,
            Self::Bottom => -1 << 5,
            Self::Front => -1 << 10,
            Self::Back => 1 << 10,
        }
    }

    pub fn voffset(self) -> glam::IVec3 {
        match self {
            Self::Right => glam::IVec3::X,
            Self::Left => -glam::IVec3::X,
            Self::Top => glam::IVec3::Y,
            Self::Bottom => -glam::IVec3::Y,
            Self::Front => -glam::IVec3::Z,
            Self::Back => glam::IVec3::Z,
        }
    }
    pub fn try_from_dir(dir: glam::Vec3) -> Option<Self> {
        match dir.round().as_ivec3() {
            glam::IVec3 { x: 1, y: 0, z: 0 } => Some(Self::Right),
            glam::IVec3 { x: -1, y: 0, z: 0 } => Some(Self::Left),
            glam::IVec3 { x: 0, y: 1, z: 0 } => Some(Self::Top),
            glam::IVec3 { x: 0, y: -1, z: 0 } => Some(Self::Bottom),
            glam::IVec3 { x: 0, y: 0, z: 1 } => Some(Self::Back),
            glam::IVec3 { x: 0, y: 0, z: -1 } => Some(Self::Front),
            _ => None,
        }
    }

    pub fn is_edge(self, i: usize) -> bool {
        match self {
            Self::Right => i & 31 == 31,
            Self::Left => i & 31 == 0,
            Self::Top => i >> 5 & 31 == 31,
            Self::Bottom => i >> 5 & 31 == 0,
            Self::Front => i >> 10 == 0,
            Self::Back => i >> 10 == 31,
        }
    }

    pub const fn flip(self) -> Self {
        const _: () = {
            if !matches!(BlockFace::Front.flip(), BlockFace::Back) {
                panic!("BlockFace::Front.flip() != BlockFace::Back");
            } else if !matches!(BlockFace::Back.flip(), BlockFace::Front) {
                panic!("BlockFace::Back.flip() != BlockFace::Front");
            } else if !matches!(BlockFace::Top.flip(), BlockFace::Bottom) {
                panic!("BlockFace::Top.flip() != BlockFace::Bottom");
            } else if !matches!(BlockFace::Bottom.flip(), BlockFace::Top) {
                panic!("BlockFace::Bottom.flip() != BlockFace::Top");
            } else if !matches!(BlockFace::Right.flip(), BlockFace::Left) {
                panic!("BlockFace::Right.flip() != BlockFace::Left");
            } else if !matches!(BlockFace::Left.flip(), BlockFace::Right) {
                panic!("BlockFace::Left.flip() != BlockFace::Right");
            }
        };

        unsafe { Self::from_u8_unchecked(self as u8 ^ 1) }
    }

    pub fn on(self, dir: Self) -> Self {
        match dir {
            Self::Front => self,
            Self::Back => match self {
                Self::Top | Self::Bottom => self,
                _ => self.flip(),
            },
            _ => match self {
                Self::Front => dir.flip(),
                Self::Back => dir,
                _ if self == dir.flip() => Self::Back,
                _ if self == dir => Self::Front,
                _ => self,
            },
        }
    }
}

#[derive(Debug)]
pub struct ChunkBlock {
    pub id: u32,
    pub faces: [u16; 6],
    pub faces_bit16: u8,
    pub dir: BlockFace,
    pub data: Option<Box<[u8]>>,
}

#[derive(Debug)]
pub struct Chunk {
    pub pos: glam::IVec3,
    pub blocks: Vec<ChunkBlock>,
    pub vertices: Vec<[Vertex; 4]>,
    pub vertex_buffer: Option<wgpu::Buffer>,
    pub index_buffer: Option<wgpu::Buffer>,
}

impl ChunkBlock {
    pub fn face(&self, face: BlockFace) -> Option<usize> {
        let face = face as usize;
        let value = ((self.faces_bit16 as usize >> face & 1) << 16) + self.faces[face] as usize;
        (value + 1 < 1 << 17).then_some(value)
    }
    pub fn set_face(&mut self, face: BlockFace, value: Option<usize>) {
        let assigned_face = face;
        let assigned_value = value;
        debug_assert!(!value.is_some_and(|v| 1 << 17 - 1 <= v));
        let value = value.unwrap_or((1 << 17) - 1);

        let face = face as usize;
        self.faces_bit16 &= !(1 << face);
        self.faces_bit16 |= ((value >> 16) << face) as u8;
        self.faces[face] = value as u16;

        debug_assert_eq!(self.face(assigned_face), assigned_value);
    }

    pub fn data<'a>(&self, reg: &'a BlockRegistry) -> &'a BlockData {
        &reg.blocks[self.id as usize]
    }

    pub fn gen_face(&self, reg: &BlockRegistry, pos: glam::Vec3, face: BlockFace) -> [Vertex; 4] {
        let data = self.data(reg);

        let face_on_block = face.on(self.dir);
        let texture = match data.mesh_type {
            BlockMeshType::Transparent => panic!("Transparent blocks should not be rendered"),
            BlockMeshType::SameSided(coords) => coords,
            BlockMeshType::Surrounded { top, bottom, sides } => match face {
                BlockFace::Top => top,
                BlockFace::Bottom => bottom,
                _ => sides,
            },
            BlockMeshType::Directional {
                right,
                left,
                top,
                bottom,
                front,
                back,
            } => match face_on_block {
                BlockFace::Right => right,
                BlockFace::Left => left,
                BlockFace::Front => front,
                BlockFace::Back => back,
                BlockFace::Top => top,
                BlockFace::Bottom => bottom,
            },
        };

        let mut vertices = [(0, 0), (1, 0), (1, 1), (0, 1)].map(|(i, j): (i8, i8)| {
            use BlockFace as BF;
            let axis = (face as u8 + 1 & 1) as _;
            let local_pos = match (face, self.dir) {
                (BF::Right | BF::Left, BF::Top) => glam::vec3(axis, i as _, (1 - j) as _),
                (BF::Right | BF::Left, BF::Bottom) => glam::vec3(axis, (1 - i) as _, j as _),
                (BF::Right | BF::Left, _) => glam::vec3(axis, (1 - j) as _, (1 - i) as _),

                (BF::Front | BF::Back, BF::Left) => glam::vec3(i as _, (1 - j) as _, axis),
                (BF::Front | BF::Back, BF::Top) => glam::vec3((1 - i) as _, j as _, axis),
                (BF::Front | BF::Back, BF::Bottom) => glam::vec3(i as _, (1 - j) as _, axis),
                (BF::Front | BF::Back, _) => glam::vec3(i as _, (1 - j) as _, axis),

                (BF::Top | BF::Bottom, BF::Left) => glam::vec3((1 - j) as _, axis, i as _),
                (BF::Top | BF::Bottom, BF::Back) => glam::vec3(i as _, axis, j as _),
                (BF::Top | BF::Bottom, BF::Right) => glam::vec3(j as _, axis, (1 - i) as _),
                (BF::Top | BF::Bottom, BF::Bottom) => glam::vec3(i as _, axis, j as _),
                (BF::Top | BF::Bottom, _) => glam::vec3((1 - i) as _, axis, (1 - j) as _),
            };
            Vertex {
                position: pos + local_pos,
                tex_coords: texture.get(glam::vec2(i as _, j as _)),

                color: texture.color.0,
            }
        });
        if face as u8 & 1 == 0 {
            vertices.reverse();
        }
        vertices
    }
}

impl Chunk {
    pub fn create_render_pipeline(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        chunk_bind_group_layout: &wgpu::BindGroupLayout,
        camera_bind_group_layout: &wgpu::BindGroupLayout,
        light_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> wgpu::RenderPipeline {
        graphics::create_render_pipeline(
            device,
            config,
            "Chunk Render Pipeline",
            &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Chunk Render Pipeline Layout"),
                bind_group_layouts: &[
                    chunk_bind_group_layout,
                    camera_bind_group_layout,
                    light_bind_group_layout,
                ],
                push_constant_ranges: &[],
            }),
            &[Vertex::DESC, ChunkInstance::DESC],
            &device.create_shader_module(wgpu::include_wgsl!("chunk.wgsl")),
        )
    }

    pub fn block_idx_to_pos(idx: usize) -> glam::UVec3 {
        glam::uvec3((idx & 31) as _, (idx >> 5 & 31) as _, (idx >> 10) as _)
    }
    pub fn block_pos_to_idx(pos: glam::UVec3) -> usize {
        pos.x as usize + pos.y as usize * 32 + pos.z as usize * 1024
    }

    pub fn generate(pos: glam::IVec3) -> Self {
        let blocks = (0..1 << 15)
            .map(|i| {
                let pos = Self::block_idx_to_pos(i);

                let dir_id = 4;
                let (id, dir);
                if pos == glam::uvec3(16, 20, 16) {
                    id = dir_id;
                    dir = BlockFace::Front;
                } else if pos == glam::uvec3(18, 20, 16) {
                    id = dir_id;
                    dir = BlockFace::Back;
                } else if pos == glam::uvec3(20, 20, 16) {
                    id = dir_id;
                    dir = BlockFace::Right;
                } else if pos == glam::uvec3(22, 20, 16) {
                    id = dir_id;
                    dir = BlockFace::Left;
                } else if pos == glam::uvec3(24, 20, 16) {
                    id = dir_id;
                    dir = BlockFace::Top;
                } else if pos == glam::uvec3(26, 20, 16) {
                    id = dir_id;
                    dir = BlockFace::Bottom;
                } else {
                    id = match pos.y {
                        0..=9 => 1,
                        10..=14 => 2,
                        15 => 3,
                        16..=31 => 0,
                        _ => unreachable!(),
                    };
                    dir = BlockFace::iter()
                        .take(4)
                        .choose(&mut rand::thread_rng())
                        .unwrap();
                };

                ChunkBlock {
                    id,
                    faces: [!0; 6],
                    faces_bit16: !0,
                    dir,
                    data: None,
                }
            })
            .collect();

        Self {
            pos,
            blocks,
            vertices: Vec::new(),
            vertex_buffer: None,
            index_buffer: None,
        }
    }

    pub fn place_block(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        reg: &BlockRegistry,
        idx: usize,
        id: u32,
        dir: BlockFace,
    ) {
        let block = &mut self.blocks[idx];
        let was_transparent = block.data(reg).is_transparent();

        block.id = id;
        block.dir = dir;
        block.data = None;

        let is_transparent = block.data(reg).is_transparent();

        if self.vertex_buffer.is_none() {
            return;
        }
        self.recreate_buffers_if_full(device);

        let vertex_buffer = self.vertex_buffer.as_ref().unwrap();

        let pos = Self::block_idx_to_pos(idx).as_vec3();
        match (was_transparent, is_transparent) {
            (true, true) => {}
            (true, false) => {
                // place block
                // tracing::info!("Placing block at {:?}", pos);

                for face in BlockFace::iter() {
                    match idx
                        .checked_add_signed(face.ioffset())
                        .filter(|_| !face.is_edge(idx))
                        .and_then(|j| Some((j, self.blocks.get(j)?)))
                    {
                        Some((j, neighbour)) if !neighbour.data(reg).is_transparent() => {
                            self.remove_face(queue, j, face.flip());
                        }
                        _ => {
                            self.add_face(queue, reg, idx, face);
                        }
                    }
                }
            }
            (false, true) => {
                // remove block

                // tracing::info!("Removing block at {:?}", pos);
                for face in BlockFace::iter() {
                    match idx
                        .checked_add_signed(face.ioffset())
                        .filter(|_| !face.is_edge(idx))
                        .and_then(|j| Some((j, self.blocks.get(j)?)))
                    {
                        Some((j, neighbour)) if !neighbour.data(reg).is_transparent() => {
                            self.add_face(queue, reg, j, face.flip());
                        }
                        _ => {
                            self.remove_face(queue, idx, face);
                        }
                    }
                }
            }
            (false, false) => {
                // replace block

                // tracing::info!("Replacing block at {:?}", pos);
                let block = &self.blocks[idx];
                for face in BlockFace::iter() {
                    let Some(face_i) = block.face(face) else {
                        continue;
                    };
                    let vertices = block.gen_face(reg, pos, face);
                    queue.write_buffer(
                        vertex_buffer,
                        (mem::size_of_val(&vertices) * face_i) as _,
                        bytemuck::cast_slice(&vertices),
                    );
                }
            }
        }
    }

    /// Prior to calling this function, call `recreate_buffers_if_full` to ensure that the buffers are large enough
    pub fn add_face(
        &mut self,
        queue: &wgpu::Queue,
        reg: &BlockRegistry,
        idx: usize,
        face: BlockFace,
    ) {
        let Some(vertex_buffer) = &self.vertex_buffer else {
            return;
        };

        let block = &mut self.blocks[idx];

        let face_i = self.vertices.len();
        block.set_face(face, Some(face_i as _));

        let vertices = block.gen_face(reg, Self::block_idx_to_pos(idx).as_vec3(), face);
        self.vertices.push(vertices);
        let bytes = bytemuck::cast_slice(&vertices);
        queue.write_buffer(vertex_buffer, (bytes.len() * face_i) as _, bytes);
    }

    pub fn remove_face(&mut self, queue: &wgpu::Queue, idx: usize, face: BlockFace) {
        let Some(vertex_buffer) = &self.vertex_buffer else {
            return;
        };

        let block = &mut self.blocks[idx];
        let Some(face_i) = block.face(face) else {
            return;
        };
        block.set_face(face, None);
        self.vertices.swap_remove(face_i);

        if face_i == self.vertices.len() {
            return;
        }

        let vertices = self.vertices[face_i];
        let dir = (vertices[1].position - vertices[0].position)
            .cross(vertices[2].position - vertices[0].position);
        let middle = vertices.iter().map(|v| v.position).sum::<glam::Vec3>() / 4.0;
        let pos = middle - 0.5 * dir.normalize();

        let idx = Self::block_pos_to_idx(pos.as_uvec3());
        let face = BlockFace::try_from_dir(dir).unwrap();

        debug_assert_eq!(self.blocks[idx].face(face), Some(self.vertices.len()));
        self.blocks[idx].set_face(face, Some(face_i as _));

        let bytes = bytemuck::cast_slice(&vertices);
        queue.write_buffer(vertex_buffer, (bytes.len() * face_i) as _, bytes);
    }

    pub fn gen_mesh(&mut self, device: &wgpu::Device, reg: &BlockRegistry) {
        self.vertices = Vec::new();

        for i in 0..1 << 15 {
            let pos = Self::block_idx_to_pos(i).as_vec3();

            let block = &self.blocks[i];
            if block.id == 0 {
                continue;
            }

            for face in BlockFace::iter() {
                if i.checked_add_signed(face.ioffset())
                    .filter(|_| !face.is_edge(i))
                    .and_then(|j| self.blocks.get(j))
                    .is_some_and(|neighbour| !neighbour.data(reg).is_transparent())
                {
                    continue;
                }

                let block = &mut self.blocks[i];
                block.set_face(face, Some(self.vertices.len() as _));
                self.vertices.push(block.gen_face(reg, pos, face));
            }
        }

        self.recreate_buffers_if_full(device);
    }

    fn recreate_buffers_if_full(&mut self, device: &wgpu::Device) {
        if self.vertex_buffer.as_ref().is_some_and(|b| {
            4 * mem::size_of::<Vertex>() * (self.vertices.len() + 6) <= b.size() as usize
        }) {
            return;
        }

        let face_capacity = (self.vertices.len() * 5 / 4).max(1024);

        tracing::info!(
            "Recreating buffers with face capacity {face_capacity} from {:?}",
            self.vertex_buffer
                .as_ref()
                .map(|b| b.size() / (4 * mem::size_of::<Vertex>()) as u64)
        );

        let vertex_buffer =
            self.vertex_buffer
                .insert(device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some(&format!("Chunk {} Vertex Buffer", self.pos)),
                    size: (4 * mem::size_of::<Vertex>() * face_capacity) as _,
                    usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: true,
                }));
        let bytes = bytemuck::cast_slice(&self.vertices);
        vertex_buffer
            .slice(..bytes.len() as u64)
            .get_mapped_range_mut()
            .copy_from_slice(bytes);
        vertex_buffer.unmap();

        if (self.index_buffer)
            .as_ref()
            .is_some_and(|b| 24 * face_capacity <= b.size() as usize)
        {
            return;
        }

        self.index_buffer = Some(
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(&format!("Chunk {} Index Buffer", self.pos)),
                contents: bytemuck::cast_slice(
                    &(0..face_capacity as u32)
                        .map(|f| [0, 1, 2, 2, 3, 0].map(|i| 4 * f + i))
                        .flatten()
                        .collect::<Vec<u32>>(),
                ),
                usage: wgpu::BufferUsages::INDEX,
            }),
        );
    }
}
