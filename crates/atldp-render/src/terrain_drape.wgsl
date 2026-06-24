// Terrain drape shader — textured, filled triangle surface (G10d, ADR-0025).
//
// Draws the working DEM grid as a solid mesh textured with the shared OSM map
// image, so the 3-D view reflects the same basemap as the plan view. Per-vertex
// UVs are computed app-side from each grid vertex's geographic position (the same
// Web-Mercator-correct lon/lat→UV map the plan view uses), so the two views stay
// registered through the shared local plane.
//
// Uniform: view_proj (mat4×4). Vertex: vec3 position (x=east, y=elev, z=north,
// metres) + vec2 uv. Writes/tests depth so the surface occludes itself correctly.

struct DrapeUniform {
    view_proj: mat4x4<f32>,
}

@group(0) @binding(0) var<uniform> u: DrapeUniform;
@group(0) @binding(1) var map_tex: texture_2d<f32>;
@group(0) @binding(2) var map_smp: sampler;

struct VertexInput  { @location(0) pos: vec3<f32>, @location(1) uv: vec2<f32> }
struct VertexOutput { @builtin(position) clip: vec4<f32>, @location(0) uv: vec2<f32> }

@vertex
fn vs_main(v: VertexInput) -> VertexOutput {
    return VertexOutput(u.view_proj * vec4<f32>(v.pos, 1.0), v.uv);
}

@fragment
fn fs_main(v: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(map_tex, map_smp, v.uv);
}
