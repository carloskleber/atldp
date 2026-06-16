//! ATLDP rendering layer — wgpu 2D + 3D (ADR-0012).
//!
//! One renderer over Vulkan/DX12/Metal. A 3D engine (terrain mesh, conductors,
//! towers, LiDAR point clouds with octree LOD, picking) and a 2D engine
//! (orthographic plan & profile: grid, snapping, layers) share this crate.
//! Interaction state lives in `atldp-app`, never here.
//!
//! Built in phase G2 (foundation) and G4 (point-cloud LOD). Skeleton only in G0.

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
