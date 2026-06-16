//! ATLDP desktop CAD application (ADR-0011, ADR-0012).
//!
//! Wires the project model (ADR-0009), the wgpu renderer, the geospatial layer,
//! and the engineering core into a winit/egui shell with a retained document and
//! a command/undo stack. The windowed app is built in phase G2; this G0 skeleton
//! only proves the workspace wiring and reports the linked component versions.

fn main() {
    println!("ATLDP desktop app (skeleton — phase G0)");
    println!("  core   {}", atldp_core::VERSION);
    println!("  geo    {}", atldp_geo::VERSION);
    println!("  model  {}", atldp_model::VERSION);
    println!("  render {}", atldp_render::VERSION);
}
