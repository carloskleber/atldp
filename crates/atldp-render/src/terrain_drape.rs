//! GPU resources for the textured terrain **drape** (G10d, ADR-0025).
//!
//! Where [`terrain_mesh`](crate::terrain_mesh) draws the DEM as a `LINE_LIST`
//! wireframe, this draws it as a **filled, textured triangle surface** sampling
//! the shared OSM map image — so the 3-D view reflects the same basemap as the
//! plan view. It is drawn *under* the wireframe (additively): when no basemap is
//! loaded the surface is simply not drawn and the 3-D view is unchanged.
//!
//! Unlike the line pipelines, the drape **writes and tests depth**
//! ([`crate::DEPTH_FORMAT`]) so the opaque surface occludes itself correctly; the
//! map image is uploaded once via [`TerrainDrapeResources::set_texture`] (not per
//! frame), and re-uploaded only when the working area / basemap changes.

use bytemuck::{Pod, Zeroable};

/// Camera uniform: single view-projection matrix (row-major, matching WGSL).
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct DrapeUniform {
    pub view_proj: [[f32; 4]; 4],
}

/// One vertex of the drape mesh: position (metres) + map-image UV.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct DrapeVertex {
    pub pos: [f32; 3],
    pub uv: [f32; 2],
}

/// GPU-side resources: pipeline, mesh buffers, uniform, texture + sampler.
pub struct TerrainDrapeResources {
    pipeline: wgpu::RenderPipeline,
    bgl: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    uniform_buf: wgpu::Buffer,
    vertex_buf: wgpu::Buffer,
    index_buf: wgpu::Buffer,
    vertex_capacity: u32,
    index_capacity: u32,
    index_count: u32,
    /// Built once a basemap texture is set; `None` ⇒ nothing to draw.
    bind_group: Option<wgpu::BindGroup>,
}

impl TerrainDrapeResources {
    const INITIAL_VERTS: u64 = 128 * 128;
    const INITIAL_INDICES: u64 = 127 * 127 * 6;

    pub fn new(device: &wgpu::Device, surface_format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::include_wgsl!("terrain_drape.wgsl"));

        let uniform_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("drape uniform"),
            size: std::mem::size_of::<DrapeUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("drape bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("drape sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("drape pipeline layout"),
            bind_group_layouts: &[Some(&bgl)],
            immediate_size: 0,
        });

        let vertex_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("drape vertices"),
            size: Self::INITIAL_VERTS * std::mem::size_of::<DrapeVertex>() as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let index_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("drape indices"),
            size: Self::INITIAL_INDICES * std::mem::size_of::<u32>() as u64,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("terrain drape"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<DrapeVertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2],
                }],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            // The surface participates in depth so it occludes itself correctly
            // (line pipelines stay depth-`Always`/no-write — see `crate::DEPTH_FORMAT`).
            depth_stencil: Some(wgpu::DepthStencilState {
                format: crate::DEPTH_FORMAT,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Less),
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        Self {
            pipeline,
            bgl,
            sampler,
            uniform_buf,
            vertex_buf,
            index_buf,
            vertex_capacity: Self::INITIAL_VERTS as u32,
            index_capacity: Self::INITIAL_INDICES as u32,
            index_count: 0,
            bind_group: None,
        }
    }

    /// Upload (or replace) the basemap texture and (re)build the bind group.
    /// `rgba` is row-major RGBA8 of size `width × height`. Call once per basemap.
    pub fn set_texture(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        rgba: &[u8],
        width: u32,
        height: u32,
    ) {
        if width == 0 || height == 0 || rgba.len() < (width as usize * height as usize * 4) {
            return;
        }
        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("drape map texture"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            // sRGB so the OSM bytes decode to linear for the sRGB surface.
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(width * 4),
                rows_per_image: Some(height),
            },
            size,
        );
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        self.bind_group = Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("drape bg"),
            layout: &self.bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.uniform_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        }));
    }

    /// Forget the current basemap texture — the drape stops drawing (offline /
    /// new working area before its basemap is fetched).
    pub fn clear_texture(&mut self) {
        self.bind_group = None;
        self.index_count = 0;
    }

    fn upload_mesh(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        vertices: &[DrapeVertex],
        indices: &[u32],
    ) {
        self.index_count = indices.len() as u32;
        if vertices.is_empty() || indices.is_empty() {
            return;
        }
        if vertices.len() as u32 > self.vertex_capacity {
            self.vertex_capacity = (vertices.len() as u32).next_power_of_two();
            self.vertex_buf = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("drape vertices (grown)"),
                size: self.vertex_capacity as u64 * std::mem::size_of::<DrapeVertex>() as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }
        if indices.len() as u32 > self.index_capacity {
            self.index_capacity = (indices.len() as u32).next_power_of_two();
            self.index_buf = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("drape indices (grown)"),
                size: self.index_capacity as u64 * std::mem::size_of::<u32>() as u64,
                usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }
        queue.write_buffer(&self.vertex_buf, 0, bytemuck::cast_slice(vertices));
        queue.write_buffer(&self.index_buf, 0, bytemuck::cast_slice(indices));
    }

    fn upload_uniform(&self, queue: &wgpu::Queue, uniform: DrapeUniform) {
        queue.write_buffer(&self.uniform_buf, 0, bytemuck::bytes_of(&uniform));
    }
}

/// egui-wgpu paint callback for the terrain drape. A no-op until a basemap
/// texture has been set on the resources (see [`TerrainDrapeResources::set_texture`]).
pub struct TerrainDrapeCallback {
    pub vertices: Vec<DrapeVertex>,
    pub indices: Vec<u32>,
    pub view_proj: [[f32; 4]; 4],
}

impl egui_wgpu::CallbackTrait for TerrainDrapeCallback {
    fn prepare(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        _screen_descriptor: &egui_wgpu::ScreenDescriptor,
        _encoder: &mut wgpu::CommandEncoder,
        resources: &mut egui_wgpu::CallbackResources,
    ) -> Vec<wgpu::CommandBuffer> {
        let res: &mut TerrainDrapeResources = resources.get_mut().unwrap();
        if res.bind_group.is_some() {
            res.upload_mesh(device, queue, &self.vertices, &self.indices);
            res.upload_uniform(
                queue,
                DrapeUniform {
                    view_proj: self.view_proj,
                },
            );
        }
        vec![]
    }

    fn paint(
        &self,
        _info: egui::PaintCallbackInfo,
        render_pass: &mut wgpu::RenderPass<'static>,
        resources: &egui_wgpu::CallbackResources,
    ) {
        let res: &TerrainDrapeResources = resources.get().unwrap();
        if let Some(bg) = &res.bind_group {
            if res.index_count >= 3 {
                render_pass.set_pipeline(&res.pipeline);
                render_pass.set_bind_group(0, bg, &[]);
                render_pass.set_vertex_buffer(0, res.vertex_buf.slice(..));
                render_pass.set_index_buffer(res.index_buf.slice(..), wgpu::IndexFormat::Uint32);
                render_pass.draw_indexed(0..res.index_count, 0, 0..1);
            }
        }
    }
}
