// Terrain wireframe shader — LINE_LIST, elevation-colored.
//
// Uniform layout: view_proj (mat4×4), elev_min (f32), elev_max (f32), pad ×2.
// Vertex layout: vec3<f32> position (x=east, y=elevation, z=north, metres).
//
// Color ramp: dark olive-green (low) → medium green (mid) → gray-white (high).

struct TerrainUniform {
    view_proj: mat4x4<f32>,
    elev_min:  f32,
    elev_max:  f32,
    _pad:      vec2<f32>,
}

@group(0) @binding(0)
var<uniform> u: TerrainUniform;

struct VertexInput  { @location(0) pos: vec3<f32> }
struct VertexOutput { @builtin(position) clip: vec4<f32>, @location(0) t: f32 }

@vertex
fn vs_main(v: VertexInput) -> VertexOutput {
    let range = max(u.elev_max - u.elev_min, 1.0);
    let t = clamp((v.pos.y - u.elev_min) / range, 0.0, 1.0);
    return VertexOutput(u.view_proj * vec4<f32>(v.pos, 1.0), t);
}

@fragment
fn fs_main(v: VertexOutput) -> @location(0) vec4<f32> {
    let t = v.t;
    let low  = vec3<f32>(0.18, 0.38, 0.08);  // dark olive-green
    let mid  = vec3<f32>(0.42, 0.64, 0.22);  // medium green
    let high = vec3<f32>(0.70, 0.70, 0.72);  // gray-white

    var col: vec3<f32>;
    if t < 0.5 {
        col = mix(low, mid, t * 2.0);
    } else {
        col = mix(mid, high, (t - 0.5) * 2.0);
    }
    return vec4<f32>(col, 0.88);
}
