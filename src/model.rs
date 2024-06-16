use std::{mem, ops, path::Path};

use anyhow::Result;
use futures::future::OptionFuture;
use tokio::{fs, io::AsyncReadExt};
use wgpu::util::DeviceExt;

use crate::{graphics::VertexBuffer, texture::Texture};

#[repr(C, packed(4))]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct InstanceRaw {
    pub model: glam::Mat4,
    pub normal: glam::Mat3,
}

#[derive(Debug, Copy, Clone)]
pub struct Instance {
    pub scale: glam::Vec3,
    pub position: glam::Vec3,
    pub rotation: glam::Quat,
}

impl Instance {
    pub const IDENTITY: Self = Self {
        scale: glam::Vec3::new(1.0, 1.0, 1.0),
        position: glam::Vec3::new(0.0, 0.0, 0.0),
        rotation: glam::Quat::IDENTITY,
    };
}

impl Default for Instance {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl VertexBuffer for Instance {
    type Raw = InstanceRaw;
    fn to_raw(&self) -> InstanceRaw {
        InstanceRaw {
            model: glam::Mat4::from_scale_rotation_translation(
                self.scale,
                self.rotation,
                self.position,
            ),
            normal: glam::Mat3::from_quat(self.rotation),
        }
    }

    const DESC: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: mem::size_of::<InstanceRaw>() as _,
        step_mode: wgpu::VertexStepMode::Instance,
        attributes: &wgpu::vertex_attr_array![
            5 => Float32x4,
            6 => Float32x4,
            7 => Float32x4,
            8 => Float32x4,
            9 => Float32x3,
            10 => Float32x3,
            11 => Float32x3,
        ],
    };
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: glam::Vec3,
    pub tex_coords: glam::Vec2,
    pub normal: glam::Vec3,
    pub tangent: glam::Vec3,
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
            2 => Float32x3, // normal
            3 => Float32x3, // tangent
        ],
    };
}
#[derive(Debug)]
pub struct Mesh {
    pub name: String,
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,

    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
}

impl Mesh {
    pub fn new(device: &wgpu::Device, name: String) -> Self {
        Self {
            vertices: Vec::new(),
            indices: Vec::new(),

            vertex_buffer: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(&format!("{name} / Vertex Buffer")),
                size: 0,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
            index_buffer: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(&format!("{name} / Index Buffer")),
                size: 0,
                usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
            name,
        }
    }

    pub fn new_init(
        device: &wgpu::Device,
        name: String,
        vertices: Vec<Vertex>,
        indices: Vec<u32>,
    ) -> Self {
        Self {
            vertex_buffer: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(&format!("{name} / Vertex Buffer")),
                contents: bytemuck::cast_slice(&vertices),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            }),
            index_buffer: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(&format!("{name} / Index Buffer")),
                contents: bytemuck::cast_slice(&indices),
                usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            }),

            vertices,
            indices,

            name,
        }
    }

    pub fn update_buffers(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        if self.vertex_buffer.size()
            < (mem::size_of::<Vertex>() * self.vertices.len()) as wgpu::BufferAddress
        {
            let size = mem::size_of::<Vertex>() * self.vertices.len();
            self.vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(&format!("{} / Vertex Buffer", self.name)),
                size: wgpu::COPY_BUFFER_ALIGNMENT.max(size.next_power_of_two() as _),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: true,
            });
            self.vertex_buffer.slice(..).get_mapped_range_mut()[..size]
                .copy_from_slice(bytemuck::cast_slice(&self.vertices));
            self.vertex_buffer.unmap();
        } else {
            queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&self.vertices));
        }

        if self.index_buffer.size()
            < (mem::size_of::<u32>() * self.indices.len()) as wgpu::BufferAddress
        {
            let size = 4 * self.indices.len();
            self.index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(&format!("{} / Index Buffer", self.name)),
                size: wgpu::COPY_BUFFER_ALIGNMENT.max(size.next_power_of_two() as _),
                usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: true,
            });
            self.index_buffer.slice(..).get_mapped_range_mut()[..size]
                .copy_from_slice(bytemuck::cast_slice(&self.indices));
            self.index_buffer.unmap();
        } else {
            queue.write_buffer(&self.index_buffer, 0, bytemuck::cast_slice(&self.indices));
        }
    }

    pub fn draw<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>, instances: ops::Range<u32>) {
        render_pass.set_vertex_buffer(
            0,
            self.vertex_buffer
                .slice(..(mem::size_of::<Vertex>() * self.vertices.len()) as u64),
        );
        render_pass.set_index_buffer(
            self.index_buffer.slice(..4 * self.indices.len() as u64),
            wgpu::IndexFormat::Uint32,
        );
        render_pass.draw_indexed(0..self.indices.len() as u32, 0, instances);
    }
}

#[derive(Debug)]
pub struct Material {
    pub name: String,
    pub diffuse_texture: Texture,
    pub normal_texture: Texture,
    pub bind_group: wgpu::BindGroup,
}

impl Material {
    pub fn create_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Material Bind Group Layout"),
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
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        })
    }
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        layout: &wgpu::BindGroupLayout,
        name: String,
        diffuse_texture: Option<Texture>,
        normal_texture: Option<Texture>,
    ) -> Self {
        let diffuse_texture =
            diffuse_texture.unwrap_or_else(|| Texture::dummy(device, queue, image::Rgba([255; 4])));
        let normal_texture = normal_texture
            .unwrap_or_else(|| Texture::dummy(device, queue, image::Rgba([127, 127, 255, 255])));
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(&format!("{name} / Bind Group")),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&diffuse_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&diffuse_texture.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&normal_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&normal_texture.sampler),
                },
            ],
        });
        Self {
            name,
            diffuse_texture,
            normal_texture,
            bind_group,
        }
    }
}

#[derive(Debug)]
pub struct ModelMesh {
    pub mesh: Mesh,
    pub material: usize,
}

impl ops::Deref for ModelMesh {
    type Target = Mesh;
    fn deref(&self) -> &Mesh {
        &self.mesh
    }
}
impl ops::DerefMut for ModelMesh {
    fn deref_mut(&mut self) -> &mut Mesh {
        &mut self.mesh
    }
}

#[derive(Debug)]
pub struct Model {
    pub meshes: Vec<ModelMesh>,
    pub materials: Vec<Material>,
}

impl Model {
    pub async fn load(
        obj_path: impl AsRef<Path>,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        material_layout: &wgpu::BindGroupLayout,
    ) -> Result<Self> {
        let obj_path = obj_path.as_ref();

        let rel_path = |path: &Path| match path.is_relative() {
            true => obj_path.with_file_name(path),
            false => path.to_path_buf(),
        };

        let (obj_models, obj_materials) = tobj::load_obj_buf_async(
            &mut fs::read(obj_path).await?.as_slice(),
            &tobj::LoadOptions {
                single_index: true,
                triangulate: true,
                ..Default::default()
            },
            |path| async move {
                let mut f = fs::File::open(rel_path(path.as_ref()))
                    .await
                    .map_err(|_| tobj::LoadError::OpenFileFailed)?;
                let mut data = Vec::new();
                f.read_to_end(&mut data)
                    .await
                    .map_err(|_| tobj::LoadError::ReadError)?;
                tobj::load_mtl_buf(&mut &data[..])
            },
        )
        .await?;

        let mut materials = Vec::new();
        for obj_material in obj_materials? {
            let diffuse_texture = obj_material.diffuse_texture.map(|path| async move {
                let path = rel_path(path.as_ref());
                Texture::load(device, queue, &path, false, &path.display().to_string()).await
            });
            let normal_texture = obj_material.normal_texture.map(|path| async move {
                let path = rel_path(path.as_ref());
                Texture::load(device, queue, &path, true, &path.display().to_string()).await
            });
            materials.push(Material::new(
                device,
                queue,
                material_layout,
                obj_material.name,
                OptionFuture::from(diffuse_texture).await.transpose()?,
                OptionFuture::from(normal_texture).await.transpose()?,
            ));
        }
        if materials.is_empty() {
            materials.push(Material::new(
                device,
                queue,
                material_layout,
                format!("{} / Default Material", obj_path.display()),
                None,
                None,
            ));
        }

        let meshes = obj_models
            .into_iter()
            .map(|obj_model| {
                let mut vertices: Vec<_> = (obj_model.mesh.positions)
                    .chunks(3)
                    .map(|pos| Vertex {
                        position: glam::Vec3::from_slice(pos),
                        normal: glam::Vec3::ZERO,
                        tex_coords: glam::Vec2::ZERO,
                        tangent: glam::Vec3::ZERO,
                    })
                    .collect();
                for (v, normal) in vertices.iter_mut().zip(obj_model.mesh.normals.chunks(3)) {
                    v.normal = glam::Vec3::from_slice(normal);
                }
                for (v, tex_coords) in vertices.iter_mut().zip(obj_model.mesh.texcoords.chunks(2)) {
                    v.tex_coords = glam::Vec2::from_slice(tex_coords);
                    v.tex_coords.y = 1.0 - v.tex_coords.y;
                }

                let has_tex_coords = !vertices.iter().all(|v| v.tex_coords == glam::Vec2::ZERO);
                let has_normals = !vertices.iter().any(|v| v.normal == glam::Vec3::ZERO);

                for tri in obj_model.mesh.indices.chunks(3) {
                    let [i0, i1, i2] = [tri[0] as usize, tri[1] as usize, tri[2] as usize];
                    let [v0, v1, v2] = [vertices[i0], vertices[i1], vertices[i2]];

                    let delta_pos1 = v1.position - v0.position;
                    let delta_pos2 = v2.position - v0.position;

                    if !has_normals {
                        let normal = delta_pos1.cross(delta_pos2).normalize();
                        vertices[i0].normal += normal;
                        vertices[i1].normal += normal;
                        vertices[i2].normal += normal;
                    }

                    if has_tex_coords {
                        let delta_uv1 = v1.tex_coords - v0.tex_coords;
                        let delta_uv2 = v2.tex_coords - v0.tex_coords;

                        // delta_pos1 = delta_uv1.x * T + delta_uv1.y * B
                        // delta_pos2 = delta_uv2.x * T + delta_uv2.y * B

                        let f = 1.0 / (delta_uv1.x * delta_uv2.y - delta_uv1.y * delta_uv2.x);

                        let tangent = f * (delta_uv2.y * delta_pos1 - delta_uv1.y * delta_pos2);
                        // let bitangent = -f * (delta_uv2.x * delta_pos1 - delta_uv1.x * delta_pos2);

                        vertices[i0].tangent += tangent;
                        vertices[i1].tangent += tangent;
                        vertices[i2].tangent += tangent;
                    }
                }
                // bitangent = cross(norm, tan)
                // tan = cross(bitangent, norm)
                //
                //
                //println!("vertices_num_triangles={:?}", vertices_num_triangles);
                for v in &mut vertices {
                    v.normal = v.normal.normalize();
                    v.tangent = (v.tangent.try_normalize())
                        .or_else(|| v.normal.cross(glam::Vec3::Y).try_normalize())
                        .unwrap_or(glam::Vec3::X);
                }
                // for (i, v) in vertices.iter().enumerate() {
                //     let no_bitan = v.normal.dot(v.tangent);
                //     if 0.2 < no_bitan.abs() {
                //         println!("Vertex {i}: norm * tan = {no_bitan:+.2?}");
                //     }
                // }
                ModelMesh {
                    mesh: Mesh::new_init(device, obj_model.name, vertices, obj_model.mesh.indices),
                    material: obj_model.mesh.material_id.unwrap_or(0),
                }
            })
            .collect();

        Ok(Self { meshes, materials })
    }

    pub fn draw<'a>(
        &'a self,
        render_pass: &mut wgpu::RenderPass<'a>,
        meshes: impl ops::RangeBounds<usize>,
        instances: ops::Range<u32>,
    ) {
        for mesh in &self.meshes[(meshes.start_bound().cloned(), meshes.end_bound().cloned())] {
            let material = &self.materials[mesh.material];
            render_pass.set_bind_group(0, &material.bind_group, &[]);
            mesh.draw(render_pass, instances.clone());
        }
    }
}
