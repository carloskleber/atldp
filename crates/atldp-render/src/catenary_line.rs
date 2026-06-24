//! GPU resources for rendering a catenary polyline in the 3D viewport.
//!
//! `CatenaryLineResources` owns the wgpu pipeline, vertex buffer, and camera
//! uniform buffer. It is stored in `egui_wgpu::Renderer::callback_resources`
//! and accessed from `CatenaryCallback` via the egui-wgpu callback mechanism.

use bytemuck::{Pod, Zeroable};

/// Camera uniform: single MVP matrix (row-major, matching WGSL mat4x4<f32>).
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct CameraUniform {
    pub view_proj: [[f32; 4]; 4],
}

/// One vertex of the catenary polyline.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct Vertex {
    pub pos: [f32; 3],
}

/// Preallocated capacity for catenary vertices.
const MAX_VERTICES: u64 = 512;

/// GPU-side resources: pipeline, vertex buffer, camera uniform, bind group.
pub struct CatenaryLineResources {
    pub pipeline: wgpu::RenderPipeline,
    pub vertex_buf: wgpu::Buffer,
    pub camera_buf: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
    pub vertex_count: u32,
}

impl CatenaryLineResources {
    pub fn new(device: &wgpu::Device, surface_format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::include_wgsl!("catenary_line.wgsl"));

        let camera_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("catenary camera uniform"),
            size: std::mem::size_of::<CameraUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("catenary bgl"),
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

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("catenary bg"),
            layout: &bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buf.as_entire_binding(),
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("catenary pipeline layout"),
            bind_group_layouts: &[Some(&bgl)],
            immediate_size: 0,
        });

        let vertex_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("catenary vertices"),
            size: MAX_VERTICES * std::mem::size_of::<Vertex>() as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("catenary 3D"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Vertex>() as u64,
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
                topology: wgpu::PrimitiveTopology::LineStrip,
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
            camera_buf,
            bind_group,
            vertex_count: 0,
        }
    }

    pub fn upload_vertices(&mut self, queue: &wgpu::Queue, vertices: &[Vertex]) {
        let count = vertices.len().min(MAX_VERTICES as usize);
        self.vertex_count = count as u32;
        if count > 0 {
            queue.write_buffer(
                &self.vertex_buf,
                0,
                bytemuck::cast_slice(&vertices[..count]),
            );
        }
    }

    pub fn upload_camera(&self, queue: &wgpu::Queue, uniform: CameraUniform) {
        queue.write_buffer(&self.camera_buf, 0, bytemuck::bytes_of(&uniform));
    }
}

/// egui-wgpu paint callback: updates the GPU buffers and draws the catenary.
pub struct CatenaryCallback {
    pub vertices: Vec<Vertex>,
    pub view_proj: [[f32; 4]; 4],
}

impl egui_wgpu::CallbackTrait for CatenaryCallback {
    fn prepare(
        &self,
        _device: &wgpu::Device,
        queue: &wgpu::Queue,
        _screen_descriptor: &egui_wgpu::ScreenDescriptor,
        _encoder: &mut wgpu::CommandEncoder,
        resources: &mut egui_wgpu::CallbackResources,
    ) -> Vec<wgpu::CommandBuffer> {
        let res: &mut CatenaryLineResources = resources.get_mut().unwrap();
        res.upload_vertices(queue, &self.vertices);
        res.upload_camera(
            queue,
            CameraUniform {
                view_proj: self.view_proj,
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
        let res: &CatenaryLineResources = resources.get().unwrap();
        if res.vertex_count >= 2 {
            render_pass.set_pipeline(&res.pipeline);
            render_pass.set_bind_group(0, &res.bind_group, &[]);
            render_pass.set_vertex_buffer(0, res.vertex_buf.slice(..));
            render_pass.draw(0..res.vertex_count, 0..1);
        }
    }
}
