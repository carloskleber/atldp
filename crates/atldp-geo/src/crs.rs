//! Coordinate reference system utilities for G3.
//!
//! Full `proj4rs` transform (ADR-0013) lands in a later pass. For G3, the
//! equirectangular **local tangent plane** approximation is used, which is
//! accurate to < 0.3 % over extents up to ~50 km — more than adequate for
//! display, camera setup, and profile distance calculations at this stage.
//!
//! Coordinates returned as (east, north) in metres relative to the reference
//! origin. Y-up convention: `east` → x, `elevation` → y, `north` → z.

const METRES_PER_DEGREE: f64 = 111_320.0;

/// Equirectangular local tangent plane anchored at a WGS84 reference point.
#[derive(Debug, Clone, Copy)]
pub struct LocalPlane {
    pub ref_lat: f64,
    pub ref_lon: f64,
}

impl LocalPlane {
    pub fn new(ref_lat: f64, ref_lon: f64) -> Self {
        Self { ref_lat, ref_lon }
    }

    /// WGS84 (lat, lon) → local (east, north) in metres.
    #[inline]
    pub fn to_local(&self, lat: f64, lon: f64) -> [f64; 2] {
        let east = (lon - self.ref_lon) * METRES_PER_DEGREE * self.ref_lat.to_radians().cos();
        let north = (lat - self.ref_lat) * METRES_PER_DEGREE;
        [east, north]
    }

    /// Local (east, north) in metres → WGS84 (lat, lon).
    #[inline]
    pub fn to_geo(&self, east: f64, north: f64) -> [f64; 2] {
        let lat = self.ref_lat + north / METRES_PER_DEGREE;
        let lon = self.ref_lon + east / (METRES_PER_DEGREE * self.ref_lat.to_radians().cos());
        [lat, lon]
    }

    /// Planar distance in metres between two WGS84 points.
    pub fn distance(&self, lat0: f64, lon0: f64, lat1: f64, lon1: f64) -> f64 {
        let [x0, y0] = self.to_local(lat0, lon0);
        let [x1, y1] = self.to_local(lat1, lon1);
        ((x1 - x0).powi(2) + (y1 - y0).powi(2)).sqrt()
    }
}
