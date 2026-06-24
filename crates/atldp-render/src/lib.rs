//! ATLDP rendering layer — wgpu 2D + 3D (ADR-0012).
//!
//! One renderer over Vulkan/DX12/Metal. A 3D engine (terrain mesh, conductors,
//! towers, LiDAR point clouds with octree LOD, picking) and a 2D engine
//! (orthographic plan & profile: grid, snapping, layers) share this crate.
//! Interaction state lives in `atldp-app`, never here.

pub mod camera;
pub mod catenary_line;
pub mod spotting_lines;
pub mod terrain_drape;
pub mod terrain_mesh;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Depth-buffer format shared by the 3-D pass (ADR-0025). The drape surface
/// (`terrain_drape`) writes and tests depth so a filled, opaque terrain occludes
/// itself correctly; the line pipelines and egui itself stay depth-`Always` /
/// no-write, so enabling the buffer leaves their painter-order behaviour unchanged.
pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

/// A `DepthStencilState` that neither writes nor tests depth — pass-compatible
/// with [`DEPTH_FORMAT`] while preserving pure painter-order drawing. Used by the
/// line pipelines so the depth buffer can exist without changing their look.
pub fn depth_passthrough() -> wgpu::DepthStencilState {
    wgpu::DepthStencilState {
        format: DEPTH_FORMAT,
        depth_write_enabled: Some(false),
        depth_compare: Some(wgpu::CompareFunction::Always),
        stencil: wgpu::StencilState::default(),
        bias: wgpu::DepthBiasState::default(),
    }
}

/// Create a depth texture view sized to the surface, for the shared 3-D pass.
/// Recreate on resize.
pub fn create_depth_view(device: &wgpu::Device, width: u32, height: u32) -> wgpu::TextureView {
    let tex = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("atldp depth"),
        size: wgpu::Extent3d {
            width: width.max(1),
            height: height.max(1),
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: DEPTH_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    tex.create_view(&wgpu::TextureViewDescriptor::default())
}
