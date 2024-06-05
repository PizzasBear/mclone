use crate::mesh;
use bitflags::bitflags;
use cgmath::prelude::*;
use noise::NoiseFn;

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(transparent)]
pub struct BlockId(u32);

impl BlockId {
    #[inline]
    pub const fn new(scope: u32, id: u32) -> Self {
        debug_assert!(scope < (1 << 16));
        debug_assert!(id < 1 << 16);
        Self(scope << 16 | id)
    }

    #[inline]
    pub const fn id(&self) -> usize {
        (self.0 & 0xffff) as _
    }

    #[inline]
    pub const fn scope(&self) -> usize {
        (self.0 >> 16) as _
    }
}

#[derive(Default, Clone, Copy, Debug, Hash, PartialEq, Eq, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C, align(2))]
pub struct Lighting {
    pub sky_red: u8,
    pub blue_green: u8,
}

#[derive(Clone, Debug, Hash)]
#[repr(C)]
pub struct Block {
    pub id: BlockId,
    pub data: Vec<u8>,
}

impl Lighting {
    const ZERO: Self = Self {
        sky_red: 0,
        blue_green: 0,
    };

    #[inline]
    pub const fn new(sky: u8, red: u8, green: u8, blue: u8) -> Self {
        Self {
            sky_red: sky << 4 | red,
            blue_green: blue << 4 | green,
        }
    }

    #[inline]
    pub const fn get_sky(&self) -> u8 {
        self.sky_red >> 4
    }
    #[inline]
    pub const fn get_red(&self) -> u8 {
        self.sky_red & 0xf
    }
    #[inline]
    pub const fn get_blue(&self) -> u8 {
        self.blue_green >> 4
    }
    #[inline]
    pub const fn get_green(&self) -> u8 {
        self.blue_green & 0xf
    }

    #[inline]
    pub fn set_sky(&mut self, intensity: u8) {
        self.sky_red = self.sky_red & 0xf | intensity << 4;
    }
    #[inline]
    pub fn set_red(&mut self, intensity: u8) {
        self.sky_red = self.sky_red & 0xf0 | intensity;
    }
    #[inline]
    pub fn set_blue(&mut self, intensity: u8) {
        self.blue_green = self.blue_green & 0xf | intensity << 4;
    }
    #[inline]
    pub fn set_green(&mut self, intensity: u8) {
        self.blue_green = self.blue_green & 0xf0 | intensity;
    }
}

#[derive(Clone)]
#[repr(transparent)]
pub struct ChunkData([[[Block; 16]; 16]; 16]);

impl std::ops::Deref for ChunkData {
    type Target = [[[Block; 16]; 16]; 16];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for ChunkData {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[allow(dead_code)]
pub struct Chunk {
    data: Box<ChunkData>,
    mesh: mesh::Mesh,
    recreate_mesh: bool,

    x: i32,
    y: i32,
    z: i32,
}

bitflags! {
    pub struct Faces: u8 {
        const TOP = 1<<0;
        const BOTTOM = 1<<1;
        const FRONT = 1<<2;
        const BACK = 1<<3;
        const RIGHT = 1<<4;
        const LEFT = 1<<5;

        const ALL = (1<<6) - 1;
    }
}

struct Item;

#[derive(Clone, Copy)]
pub enum BlockVisibilityData {
    Opaque {
        draw: fn(&mut mesh::Mesh, [usize; 3], Faces, FacedData<Lighting>, data: &[u8]),
    },
    Transparent {
        draw: fn(&mut mesh::Mesh, [usize; 3], data: &[u8]),
        lighting: fn(data: &[u8]) -> Lighting,
    },
}

impl BlockVisibilityData {
    #[inline]
    pub fn is_transparent(&self) -> bool {
        matches!(self, Self::Transparent { .. })
    }
}

#[derive(Clone, Copy)]
pub struct BlockTypeData {
    bvd: BlockVisibilityData,
}

impl BlockTypeData {
    #[inline]
    pub fn is_transparent(&self) -> bool {
        self.bvd.is_transparent()
    }
}

pub const AIR: BlockId = BlockId::new(0, 0);

#[inline]
fn light4f(x: u8) -> f32 {
    (x + 1) as f32 / 16.0
}

#[derive(Default, Clone, Copy)]
pub struct FacedData<T> {
    pub top: T,
    pub bottom: T,
    pub front: T,
    pub back: T,
    pub left: T,
    pub right: T,
}

#[macro_export]
macro_rules! faced_data {
    ($e:expr; reeval 6) => {{
        FacedData {
            top: $e,
            bottom: $e,
            front: $e,
            back: $e,
            left: $e,
            right: $e,
        }
    }};
    ($e:expr; copy 6) => {{
        let e = $e;
        FacedData {
            top: e,
            bottom: e,
            front: e,
            back: e,
            left: e,
            right: e,
        }
    }};
    ($e:expr; clone 6) => {{
        let e = $e;
        FacedData {
            top: e.clone(),
            bottom: e.clone(),
            front: e.clone(),
            back: e.clone(),
            left: e.clone(),
            right: e,
        }
    }};
}

impl Chunk {
    pub fn new<T: From<i32>, N: NoiseFn<[T; 3]>>(
        noise: N,
        chunk_x: i32,
        chunk_y: i32,
        chunk_z: i32,
    ) -> Self {
        for x in 0..16 {
            for y in 0..16 {
                for z in 0..16 {
                    let global_x = chunk_x + x as i32;
                    let global_y = chunk_y + y as i32;
                    let global_z = chunk_z + z as i32;

                    let noise = noise.get([global_x.into(), global_y.into(), global_z.into()]);
                }
            }
        }

        todo!();
    }

    pub fn empty(x: i32, y: i32, z: i32) -> Self {
        let mut mesh = mesh::Mesh::empty(Some(format!("Chunk({},{},{})", x, y, z)));
        mesh.instances_mut().push(mesh::Instance {
            position: cgmath::Point3::new((16 * x) as _, (16 * y) as _, (16 * z) as _),
            rotation: cgmath::Quaternion::one(),
        });

        Self {
            data: Box::new(ChunkData(
                [[[Block {
                    id: AIR,
                    data: vec![0x00_00_f0_00],
                }; 16]; 16]; 16],
            )),
            mesh,
            recreate_mesh: true,

            x,
            y,
            z,
        }
    }

    pub fn draw<'a>(
        &'a mut self,
        device: &mut wgpu::Device,
        render_pass: &mut wgpu::RenderPass<'a>,
        faces: Faces,
        neighbours: FacedData<Option<&Chunk>>,
        block_data: &[Vec<BlockTypeData>],
    ) {
        if self.recreate_mesh {
            self.recreate_mesh = false;

            for x in 0..16 {
                for y in 0..16 {
                    for z in 0..16 {
                        let block = self.data[x][y][z];
                        let btd = &block_data[block.id.scope()][block.id.id()];

                        match btd.bvd {
                            BlockVisibilityData::Opaque { draw } => {
                                let mut faced_lighting = faced_data![Lighting::ZERO; reeval 6];
                                let mut faces = faces;

                                macro_rules! add_neighbour_lighting {
                                    ($face:expr, $side:ident, if ($coords2:expr) {[$x2:expr][$y2:expr][$z2:expr]} else {[$x1:expr][$y1:expr][$z1:expr]}) => {{
                                        if faces.contains($face) {
                                            let neighbour = if $coords2 {
                                                neighbours
                                                    .$side
                                                    .map(|chunk| chunk.data[$x2][$y2][$z2])
                                            } else {
                                                Some(self.data[$x1][$y1][$z1])
                                            };
                                            if let Some(neighbour) = neighbour {
                                                let neighbour_btd = block_data
                                                    [neighbour.id.scope()][neighbour.id.id()];
                                                match neighbour_btd.bvd {
                                                    BlockVisibilityData::Transparent {
                                                        lighting,
                                                        ..
                                                    } => {
                                                        faced_lighting.$side =
                                                            lighting(&neighbour.data);
                                                    }
                                                    BlockVisibilityData::Opaque { .. } => {
                                                        faces.remove($face);
                                                    }
                                                }
                                            } else {
                                                faces.remove($face);
                                            }
                                        }
                                    }};
                                }

                                add_neighbour_lighting!(
                                    Faces::TOP,
                                    top,
                                    if (y == 15) { [x][0][z] } else { [x][y + 1][z] }
                                );
                                add_neighbour_lighting!(
                                    Faces::BOTTOM,
                                    bottom,
                                    if (y == 0) { [x][15][z] } else { [x][y - 1][z] }
                                );
                                add_neighbour_lighting!(
                                    Faces::FRONT,
                                    front,
                                    if (z == 15) { [x][y][0] } else { [x][y][z + 1] }
                                );
                                add_neighbour_lighting!(
                                    Faces::BACK,
                                    back,
                                    if (z == 0) { [x][y][15] } else { [x][y][z - 1] }
                                );
                                add_neighbour_lighting!(
                                    Faces::RIGHT,
                                    right,
                                    if (x == 15) { [0][y][z] } else { [x + 1][y][z] }
                                );
                                add_neighbour_lighting!(
                                    Faces::LEFT,
                                    left,
                                    if (x == 0) { [15][y][z] } else { [x - 1][y][z] }
                                );

                                draw(
                                    &mut self.mesh,
                                    [x, y, z],
                                    faces,
                                    faced_lighting,
                                    &block.data,
                                );
                            }
                            BlockVisibilityData::Transparent { draw, .. } => {
                                draw(&mut self.mesh, [x, y, z], &block.data)
                            }
                        }

                        // if block.id != AIR {
                        //     if let Some(lighting) = if x == 0 {
                        //         neighbours[0]
                        //             .map(|chunk_data| chunk_data[15][y][z])
                        //             .filter(|block| {
                        //                 block_data[block.id.scope()][block.id.id()].is_transparent
                        //             })
                        //             .map(|block| block.lighting())
                        //     } else {
                        //         let block = self.data[x - 1][y][z];

                        //         if block_data[block.id.scope()][block.id.id()].is_transparent {
                        //             Some(block.lighting())
                        //         } else {
                        //             None
                        //         }
                        //     } {
                        //         let vertices = self.mesh.vertices_mut();
                        //         let idx = vertices.len() as u32;

                        //         let sky_coef = light4f(lighting.get_sky());
                        //         let color = [
                        //             sky_coef.max(light4f(lighting.get_red())),
                        //             sky_coef.max(light4f(lighting.get_green())),
                        //             sky_coef.max(light4f(lighting.get_blue())),
                        //         ];
                        //         vertices.push(mesh::Vertex {
                        //             position: [x as f32, y as f32, z as f32],
                        //             tex_coords: btd.tex_coords0[0],
                        //             color,
                        //         });
                        //         vertices.push(mesh::Vertex {
                        //             position: [x as f32, (y + 1) as f32, z as f32],
                        //             tex_coords: [btd.tex_coords0[0][0], btd.tex_coords1[0][1]],
                        //             color,
                        //         });
                        //         vertices.push(mesh::Vertex {
                        //             position: [x as f32, (y + 1) as f32, (z + 1) as f32],
                        //             tex_coords: btd.tex_coords1[0],
                        //             color,
                        //         });
                        //         vertices.push(mesh::Vertex {
                        //             position: [x as f32, y as f32, (z + 1) as f32],
                        //             tex_coords: [btd.tex_coords1[0][0], btd.tex_coords0[0][1]],
                        //             color,
                        //         });

                        //         let indices = self.mesh.indices_mut();
                        //         indices.push([idx, idx + 2, idx + 1]);
                        //         indices.push([idx, idx + 3, idx + 2]);
                        //     }

                        //     if let Some(lighting) = if x == 15 {
                        //         neighbours[1]
                        //             .map(|chunk_data| chunk_data[0][y][z])
                        //             .filter(|block| {
                        //                 block_data[block.id.scope()][block.id.id()].is_transparent
                        //             })
                        //             .map(|block| block.lighting())
                        //     } else {
                        //         let block = self.data[x + 1][y][z];

                        //         if block_data[block.id.scope()][block.id.id()].is_transparent {
                        //             Some(block.lighting())
                        //         } else {
                        //             None
                        //         }
                        //     } {
                        //         let vertices = self.mesh.vertices_mut();
                        //         let idx = vertices.len() as u32;

                        //         let sky_coef = light4f(lighting.get_sky());
                        //         let color = [
                        //             sky_coef.max(light4f(lighting.get_red())),
                        //             sky_coef.max(light4f(lighting.get_green())),
                        //             sky_coef.max(light4f(lighting.get_blue())),
                        //         ];
                        //         vertices.push(mesh::Vertex {
                        //             position: [(x + 1) as f32, y as f32, z as f32],
                        //             tex_coords: btd.tex_coords0[1],
                        //             color,
                        //         });
                        //         vertices.push(mesh::Vertex {
                        //             position: [(x + 1) as f32, (y + 1) as f32, z as f32],
                        //             tex_coords: [btd.tex_coords0[1][0], btd.tex_coords1[1][1]],
                        //             color,
                        //         });
                        //         vertices.push(mesh::Vertex {
                        //             position: [(x + 1) as f32, (y + 1) as f32, (z + 1) as f32],
                        //             tex_coords: btd.tex_coords1[1],
                        //             color,
                        //         });
                        //         vertices.push(mesh::Vertex {
                        //             position: [(x + 1) as f32, y as f32, (z + 1) as f32],
                        //             tex_coords: [btd.tex_coords1[1][0], btd.tex_coords0[1][1]],
                        //             color,
                        //         });

                        //         let indices = self.mesh.indices_mut();
                        //         indices.push([idx, idx + 1, idx + 2]);
                        //         indices.push([idx, idx + 2, idx + 3]);
                        //     }

                        //     if let Some(lighting) = if y == 0 {
                        //         neighbours[2]
                        //             .map(|chunk_data| chunk_data[x][15][z])
                        //             .filter(|block| {
                        //                 block_data[block.id.scope()][block.id.id()].is_transparent
                        //             })
                        //             .map(|block| block.lighting())
                        //     } else {
                        //         let block = self.data[x][y - 1][z];

                        //         if block_data[block.id.scope()][block.id.id()].is_transparent {
                        //             Some(block.lighting())
                        //         } else {
                        //             None
                        //         }
                        //     } {
                        //         let vertices = self.mesh.vertices_mut();
                        //         let idx = vertices.len() as u32;

                        //         let sky_coef = light4f(lighting.get_sky());
                        //         let color = [
                        //             sky_coef.max(light4f(lighting.get_red())),
                        //             sky_coef.max(light4f(lighting.get_green())),
                        //             sky_coef.max(light4f(lighting.get_blue())),
                        //         ];
                        //         vertices.push(mesh::Vertex {
                        //             position: [x as f32, y as f32, z as f32],
                        //             tex_coords: btd.tex_coords0[2],
                        //             color,
                        //         });
                        //         vertices.push(mesh::Vertex {
                        //             position: [(x + 1) as f32, y as f32, z as f32],
                        //             tex_coords: [btd.tex_coords0[2][0], btd.tex_coords1[2][1]],
                        //             color,
                        //         });
                        //         vertices.push(mesh::Vertex {
                        //             position: [(x + 1) as f32, y as f32, (z + 1) as f32],
                        //             tex_coords: btd.tex_coords1[2],
                        //             color,
                        //         });
                        //         vertices.push(mesh::Vertex {
                        //             position: [x as f32, y as f32, (z + 1) as f32],
                        //             tex_coords: [btd.tex_coords1[2][0], btd.tex_coords0[2][1]],
                        //             color,
                        //         });

                        //         let indices = self.mesh.indices_mut();
                        //         indices.push([idx, idx + 1, idx + 2]);
                        //         indices.push([idx, idx + 2, idx + 3]);
                        //     }

                        //     if let Some(lighting) = if y == 15 {
                        //         neighbours[3]
                        //             .map(|chunk_data| chunk_data[x][0][z])
                        //             .filter(|block| {
                        //                 block_data[block.id.scope()][block.id.id()].is_transparent
                        //             })
                        //             .map(|block| block.lighting())
                        //     } else {
                        //         let block = self.data[x][y + 1][z];

                        //         if block_data[block.id.scope()][block.id.id()].is_transparent {
                        //             Some(block.lighting())
                        //         } else {
                        //             None
                        //         }
                        //     } {
                        //         let vertices = self.mesh.vertices_mut();
                        //         let idx = vertices.len() as u32;

                        //         let sky_coef = light4f(lighting.get_sky());
                        //         let color = [
                        //             sky_coef.max(light4f(lighting.get_red())),
                        //             sky_coef.max(light4f(lighting.get_green())),
                        //             sky_coef.max(light4f(lighting.get_blue())),
                        //         ];
                        //         vertices.push(mesh::Vertex {
                        //             position: [x as f32, (y + 1) as f32, z as f32],
                        //             tex_coords: btd.tex_coords0[3],
                        //             color,
                        //         });
                        //         vertices.push(mesh::Vertex {
                        //             position: [(x + 1) as f32, (y + 1) as f32, z as f32],
                        //             tex_coords: [btd.tex_coords0[3][0], btd.tex_coords1[3][1]],
                        //             color,
                        //         });
                        //         vertices.push(mesh::Vertex {
                        //             position: [(x + 1) as f32, (y + 1) as f32, (z + 1) as f32],
                        //             tex_coords: btd.tex_coords1[3],
                        //             color,
                        //         });
                        //         vertices.push(mesh::Vertex {
                        //             position: [x as f32, (y + 1) as f32, (z + 1) as f32],
                        //             tex_coords: [btd.tex_coords1[3][0], btd.tex_coords0[3][1]],
                        //             color,
                        //         });

                        //         let indices = self.mesh.indices_mut();
                        //         indices.push([idx, idx + 2, idx + 1]);
                        //         indices.push([idx, idx + 3, idx + 2]);
                        //     }

                        //     if let Some(lighting) = if z == 0 {
                        //         neighbours[4]
                        //             .map(|chunk_data| chunk_data[x][y][15])
                        //             .filter(|block| {
                        //                 block_data[block.id.scope()][block.id.id()].is_transparent
                        //             })
                        //             .map(|block| block.lighting())
                        //     } else {
                        //         let block = self.data[x][y][z - 1];

                        //         if block_data[block.id.scope()][block.id.id()].is_transparent {
                        //             Some(block.lighting())
                        //         } else {
                        //             None
                        //         }
                        //     } {
                        //         let vertices = self.mesh.vertices_mut();
                        //         let idx = vertices.len() as u32;

                        //         let sky_coef = light4f(lighting.get_sky());
                        //         let color = [
                        //             sky_coef.max(light4f(lighting.get_red())),
                        //             sky_coef.max(light4f(lighting.get_green())),
                        //             sky_coef.max(light4f(lighting.get_blue())),
                        //         ];
                        //         vertices.push(mesh::Vertex {
                        //             position: [x as f32, y as f32, z as f32],
                        //             tex_coords: btd.tex_coords0[4],
                        //             color,
                        //         });
                        //         vertices.push(mesh::Vertex {
                        //             position: [(x + 1) as f32, y as f32, z as f32],
                        //             tex_coords: [btd.tex_coords1[4][0], btd.tex_coords0[4][1]],
                        //             color,
                        //         });
                        //         vertices.push(mesh::Vertex {
                        //             position: [(x + 1) as f32, (y + 1) as f32, z as f32],
                        //             tex_coords: btd.tex_coords1[4],
                        //             color,
                        //         });
                        //         vertices.push(mesh::Vertex {
                        //             position: [x as f32, (y + 1) as f32, z as f32],
                        //             tex_coords: [btd.tex_coords0[4][0], btd.tex_coords1[4][1]],
                        //             color,
                        //         });

                        //         let indices = self.mesh.indices_mut();
                        //         indices.push([idx, idx + 2, idx + 1]);
                        //         indices.push([idx, idx + 3, idx + 2]);
                        //     }

                        //     if let Some(lighting) = if z == 15 {
                        //         neighbours[5]
                        //             .map(|chunk_data| chunk_data[x][y][0])
                        //             .filter(|block| {
                        //                 block_data[block.id.scope()][block.id.id()].is_transparent
                        //             })
                        //             .map(|block| block.lighting())
                        //     } else {
                        //         let block = self.data[x][y][z + 1];

                        //         if block_data[block.id.scope()][block.id.id()].is_transparent {
                        //             Some(block.lighting())
                        //         } else {
                        //             None
                        //         }
                        //     } {
                        //         let vertices = self.mesh.vertices_mut();
                        //         let idx = vertices.len() as u32;

                        //         let sky_coef = light4f(lighting.get_sky());
                        //         let color = [
                        //             sky_coef.max(light4f(lighting.get_red())),
                        //             sky_coef.max(light4f(lighting.get_green())),
                        //             sky_coef.max(light4f(lighting.get_blue())),
                        //         ];
                        //         vertices.push(mesh::Vertex {
                        //             position: [x as f32, y as f32, (z + 1) as f32],
                        //             tex_coords: btd.tex_coords0[5],
                        //             color,
                        //         });
                        //         vertices.push(mesh::Vertex {
                        //             position: [(x + 1) as f32, y as f32, (z + 1) as f32],
                        //             tex_coords: [btd.tex_coords1[5][0], btd.tex_coords0[5][1]],
                        //             color,
                        //         });
                        //         vertices.push(mesh::Vertex {
                        //             position: [(x + 1) as f32, (y + 1) as f32, (z + 1) as f32],
                        //             tex_coords: btd.tex_coords1[5],
                        //             color,
                        //         });
                        //         vertices.push(mesh::Vertex {
                        //             position: [x as f32, (y + 1) as f32, (z + 1) as f32],
                        //             tex_coords: [btd.tex_coords0[5][0], btd.tex_coords1[5][1]],
                        //             color,
                        //         });

                        //         let indices = self.mesh.indices_mut();
                        //         indices.push([idx, idx + 1, idx + 2]);
                        //         indices.push([idx, idx + 2, idx + 3]);
                        //     }
                        // }
                    }
                }
            }
        }
        self.mesh.draw(device, render_pass);
    }
}

#[test]
fn assert_block_size() {
    assert!(std::mem::size_of::<Block>() == 8);
}
