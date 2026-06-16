//! GPU resources for tower + conductor spotting geometry (G5).
//!
//! Vertex-coloured LINE_LIST: tower symbols (warm white) and catenary conductors
//! (cyan = clearance OK, red = clearance violation). Buffer grows on demand.

use bytemuck::{Pod, Zeroable};

/// One vertex of the spotting LINE_LIST (28 bytes, 4-byte aligned).
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct SpottingVertex {
    pub pos: [f32; 3],
    pub col: [f32; 4],
}

/// Camera uniform: MVP matrix (row-major, matching WGSL `mat4x4<f32>`).
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct SpottingUniform {
    view_proj: [[f32; 4]; 4],
}

/// GPU-side resources for spotting geometry.
pub struct SpottingResources {
    pub pipeline: wgpu::RenderPipeline,
    pub vertex_buf: wgpu::Buffer,
    pub uniform_buf: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
    pub vertex_count: u32,
    pub capacity: u32,
}

impl SpottingResources {
    const INITIAL_CAPACITY: u32 = 2048;

    pub fn new(device: &wgpu::Device, surface_format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::include_wgsl!("spotting_lines.wgsl"));

        let uniform_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("spotting uniform"),
            size: std::mem::size_of::<SpottingUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("spotting bgl"),
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
            label: Some("spotting bg"),
            layout: &bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buf.as_entire_binding(),
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("spotting pipeline layout"),
            bind_group_layouts: &[Some(&bgl)],
            immediate_size: 0,
        });

        let vertex_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("spotting vertices"),
            size: Self::INITIAL_CAPACITY as u64 * std::mem::size_of::<SpottingVertex>() as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("spotting lines 3D"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<SpottingVertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x4],
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
            depth_stencil: None,
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
            capacity: Self::INITIAL_CAPACITY,
        }
    }

    pub fn upload(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        vertices: &[SpottingVertex],
        view_proj: [[f32; 4]; 4],
    ) {
        let count = vertices.len() as u32;
        self.vertex_count = count;
        if count == 0 {
            return;
        }
        if count > self.capacity {
            self.capacity = count.next_power_of_two();
            self.vertex_buf = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("spotting vertices (grown)"),
                size: self.capacity as u64 * std::mem::size_of::<SpottingVertex>() as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }
        queue.write_buffer(&self.vertex_buf, 0, bytemuck::cast_slice(vertices));
        queue.write_buffer(
            &self.uniform_buf,
            0,
            bytemuck::bytes_of(&SpottingUniform { view_proj }),
        );
    }
}

/// egui-wgpu paint callback for tower and conductor geometry in the 3D view.
pub struct SpottingCallback {
    pub vertices: Vec<SpottingVertex>,
    pub view_proj: [[f32; 4]; 4],
}

impl egui_wgpu::CallbackTrait for SpottingCallback {
    fn prepare(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        _screen_descriptor: &egui_wgpu::ScreenDescriptor,
        _encoder: &mut wgpu::CommandEncoder,
        resources: &mut egui_wgpu::CallbackResources,
    ) -> Vec<wgpu::CommandBuffer> {
        let res: &mut SpottingResources = resources.get_mut().unwrap();
        res.upload(device, queue, &self.vertices, self.view_proj);
        vec![]
    }

    fn paint(
        &self,
        _info: egui::PaintCallbackInfo,
        render_pass: &mut wgpu::RenderPass<'static>,
        resources: &egui_wgpu::CallbackResources,
    ) {
        let res: &SpottingResources = resources.get().unwrap();
        if res.vertex_count >= 2 {
            render_pass.set_pipeline(&res.pipeline);
            render_pass.set_bind_group(0, &res.bind_group, &[]);
            render_pass.set_vertex_buffer(0, res.vertex_buf.slice(..));
            render_pass.draw(0..res.vertex_count, 0..1);
        }
    }
}
