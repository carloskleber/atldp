//! GPU resources for the terrain wireframe renderer.
//!
//! Draws a `LINE_LIST` wireframe of a downsampled DEM patch, coloured by
//! elevation. Follows the same egui-wgpu callback pattern as `catenary_line`.

use bytemuck::{Pod, Zeroable};

/// Camera + elevation range uniform, 16-byte aligned.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct TerrainUniform {
    pub view_proj: [[f32; 4]; 4],
    pub elev_min: f32,
    pub elev_max: f32,
    pub _pad: [f32; 2],
}

/// One vertex of the terrain wireframe (same memory layout as catenary Vertex).
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct TerrainVertex {
    pub pos: [f32; 3],
}

/// GPU-side resources: pipeline, vertex buffer, uniform buffer, bind group.
pub struct TerrainMeshResources {
    pub pipeline: wgpu::RenderPipeline,
    pub vertex_buf: wgpu::Buffer,
    pub uniform_buf: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
    pub vertex_count: u32,
    /// Current capacity of the vertex buffer (number of vertices).
    pub capacity: u32,
}

impl TerrainMeshResources {
    /// Initial buffer size — enlarged on the first upload if needed.
    const INITIAL_CAPACITY: u64 = 64 * 64 * 4 * 2; // 64×64 grid, 4 verts/cell, 2 dirs

    pub fn new(device: &wgpu::Device, surface_format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::include_wgsl!("terrain_mesh.wgsl"));

        let uniform_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("terrain uniform"),
            size: std::mem::size_of::<TerrainUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("terrain bgl"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("terrain bg"),
            layout: &bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buf.as_entire_binding(),
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("terrain pipeline layout"),
            bind_group_layouts: &[Some(&bgl)],
            immediate_size: 0,
        });

        let vertex_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("terrain vertices"),
            size: Self::INITIAL_CAPACITY * std::mem::size_of::<TerrainVertex>() as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("terrain wireframe"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<TerrainVertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x3],
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
                topology: wgpu::PrimitiveTopology::LineList,
                ..Default::default()
            },
            depth_stencil: Some(crate::depth_passthrough()),
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        Self {
            pipeline,
            vertex_buf,
            uniform_buf,
            bind_group,
            vertex_count: 0,
            capacity: Self::INITIAL_CAPACITY as u32,
        }
    }

    /// Upload wireframe vertices.  The buffer is recreated if the vertex count
    /// exceeds the current capacity (rare after the first frame).
    pub fn upload_vertices(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        vertices: &[TerrainVertex],
    ) {
        let count = vertices.len() as u32;
        self.vertex_count = count;
        if count == 0 {
            return;
        }
        if count > self.capacity {
            self.capacity = count.next_power_of_two();
            self.vertex_buf = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("terrain vertices (grown)"),
                size: self.capacity as u64 * std::mem::size_of::<TerrainVertex>() as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }
        queue.write_buffer(&self.vertex_buf, 0, bytemuck::cast_slice(vertices));
    }

    pub fn upload_uniform(&self, queue: &wgpu::Queue, uniform: TerrainUniform) {
        queue.write_buffer(&self.uniform_buf, 0, bytemuck::bytes_of(&uniform));
    }
}

/// egui-wgpu paint callback for the terrain wireframe.
pub struct TerrainMeshCallback {
    pub vertices: Vec<TerrainVertex>,
    pub view_proj: [[f32; 4]; 4],
    pub elev_min: f32,
    pub elev_max: f32,
}

impl egui_wgpu::CallbackTrait for TerrainMeshCallback {
    fn prepare(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        _screen_descriptor: &egui_wgpu::ScreenDescriptor,
        _encoder: &mut wgpu::CommandEncoder,
        resources: &mut egui_wgpu::CallbackResources,
    ) -> Vec<wgpu::CommandBuffer> {
        let res: &mut TerrainMeshResources = resources.get_mut().unwrap();
        res.upload_vertices(device, queue, &self.vertices);
        res.upload_uniform(
            queue,
            TerrainUniform {
                view_proj: self.view_proj,
                elev_min: self.elev_min,
                elev_max: self.elev_max,
                _pad: [0.0; 2],
            },
        );
        vec![]
    }

    fn paint(
        &self,
        _info: egui::PaintCallbackInfo,
        render_pass: &mut wgpu::RenderPass<'static>,
        resources: &egui_wgpu::CallbackResources,
    ) {
        let res: &TerrainMeshResources = resources.get().unwrap();
        if res.vertex_count >= 2 {
            render_pass.set_pipeline(&res.pipeline);
            render_pass.set_bind_group(0, &res.bind_group, &[]);
            render_pass.set_vertex_buffer(0, res.vertex_buf.slice(..));
            render_pass.draw(0..res.vertex_count, 0..1);
        }
    }
}
