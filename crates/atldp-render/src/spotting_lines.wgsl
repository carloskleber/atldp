// Spotting lines shader — vertex-coloured LINE_LIST.
//
// Uniform: view_proj (mat4x4<f32>).
// Vertex: pos vec3<f32> @loc 0, col vec4<f32> @loc 1.
// Used for towers (warm-white vertical + crossarm) and conductors (cyan / red).

struct CameraUniform {
    view_proj: mat4x4<f32>,
}

@group(0) @binding(0)
var<uniform> u: CameraUniform;

struct VertIn  { @location(0) pos: vec3<f32>, @location(1) col: vec4<f32> }
struct VertOut { @builtin(position) clip: vec4<f32>, @location(0) col: vec4<f32> }

@vertex
fn vs_main(v: VertIn) -> VertOut {
    return VertOut(u.view_proj * vec4<f32>(v.pos, 1.0), v.col);
}

@fragment
fn fs_main(v: VertOut) -> @location(0) vec4<f32> {
    return v.col;
}
