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
