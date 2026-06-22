//! Ground profile extraction from a DEM along a straight line segment.

use crate::{crs::LocalPlane, dem::Dem};

/// One sample of a ground profile.
#[derive(Debug, Clone, Copy)]
pub struct ProfilePoint {
    /// Cumulative horizontal distance from the start point, metres.
    pub distance_m: f64,
    /// Terrain elevation at this sample, metres. `f32::NAN` = no-data cell.
    pub elevation_m: f32,
}

/// Sample the DEM along the straight line from `(lat0, lon0)` to `(lat1,
/// lon1)`, producing `n_samples` evenly spaced `ProfilePoint`s.
///
/// Distance is computed in the equirectangular local plane centred on the
/// midpoint of the segment, which is accurate to < 0.3 % over spans < 50 km.
pub fn extract_profile(
    dem: &Dem,
    lat0: f64,
    lon0: f64,
    lat1: f64,
    lon1: f64,
    n_samples: usize,
) -> Vec<ProfilePoint> {
    if n_samples == 0 {
        return vec![];
    }

    let plane = LocalPlane::new((lat0 + lat1) * 0.5, (lon0 + lon1) * 0.5);
    let [x0, y0] = plane.to_local(lat0, lon0);
    let [x1, y1] = plane.to_local(lat1, lon1);
    let total_dist = ((x1 - x0).powi(2) + (y1 - y0).powi(2)).sqrt();

    (0..n_samples)
        .map(|i| {
            let t = if n_samples == 1 {
                0.0
            } else {
                i as f64 / (n_samples - 1) as f64
            };
            let lat = lat0 + t * (lat1 - lat0);
            let lon = lon0 + t * (lon1 - lon0);
            ProfilePoint {
                distance_m: t * total_dist,
                elevation_m: dem.elevation_at(lat, lon),
            }
        })
        .collect()
}

/// Sample the DEM along a **multi-segment route polyline** (G9, ADR-0019): the
/// `(lat, lon)` vertices are the route's points of interest in order. Each segment
/// gets `samples_per_segment` evenly-spaced samples; cumulative `distance_m` runs
/// continuously across the whole polyline so the derived profile shares one
/// horizontal axis with the route stations and the spotted towers.
///
/// Vertices are returned exactly once (the shared endpoint between two segments is
/// not duplicated). Fewer than two vertices, or `samples_per_segment == 0`, yields
/// an empty profile. Distances use the same equirectangular local plane as
/// [`extract_profile`], per segment, accurate to < 0.3 % over spans < 50 km.
pub fn extract_profile_polyline(
    dem: &Dem,
    vertices: &[(f64, f64)],
    samples_per_segment: usize,
) -> Vec<ProfilePoint> {
    if vertices.len() < 2 || samples_per_segment == 0 {
        return vec![];
    }

    let mut out = Vec::new();
    let mut cum_dist = 0.0;

    for (seg, pair) in vertices.windows(2).enumerate() {
        let (lat0, lon0) = pair[0];
        let (lat1, lon1) = pair[1];
        let plane = LocalPlane::new((lat0 + lat1) * 0.5, (lon0 + lon1) * 0.5);
        let [x0, y0] = plane.to_local(lat0, lon0);
        let [x1, y1] = plane.to_local(lat1, lon1);
        let seg_len = ((x1 - x0).powi(2) + (y1 - y0).powi(2)).sqrt();

        // Emit the segment start only on the first segment; otherwise it coincides
        // with the previous segment's end.
        let start = if seg == 0 { 0 } else { 1 };
        for i in start..=samples_per_segment {
            let t = i as f64 / samples_per_segment as f64;
            let lat = lat0 + t * (lat1 - lat0);
            let lon = lon0 + t * (lon1 - lon0);
            out.push(ProfilePoint {
                distance_m: cum_dist + t * seg_len,
                elevation_m: dem.elevation_at(lat, lon),
            });
        }
        cum_dist += seg_len;
    }

    out
}
