//! Digital Elevation Model — HGT (SRTM) raster parser and elevation query.
//!
//! HGT format: signed 16-bit big-endian integers, one row per latitude degree
//! subdivision, ordered N→S then W→E within each row. Void cells are encoded
//! as -32768 and mapped to `f32::NAN`. Grid size is inferred from the byte
//! count: 1201×1201 for SRTM3 (3 arc-second, ~90 m) or 3601×3601 for SRTM1
//! (1 arc-second, ~30 m).
//!
//! The `LocalGrid` type converts a downsampled DEM patch to local-plane
//! coordinates (x=east, y=elevation, z=north, all in metres) ready for the
//! terrain wireframe renderer in `atldp-render`.

use std::io;

use crate::crs::LocalPlane;

// ── geographic bounding box ────────────────────────────────────────────────

/// Axis-aligned bounding box in WGS84 geographic coordinates (degrees).
#[derive(Debug, Clone, Copy)]
pub struct GeoBounds {
    pub lat_min: f64,
    pub lat_max: f64,
    pub lon_min: f64,
    pub lon_max: f64,
}

// ── DEM raster ─────────────────────────────────────────────────────────────

/// A parsed DEM raster with bilinear elevation query.
#[derive(Debug, Clone)]
pub struct Dem {
    pub bounds: GeoBounds,
    /// Number of rows (latitude axis, stored N→S).
    pub rows: usize,
    /// Number of columns (longitude axis, stored W→E).
    pub cols: usize,
    /// Elevation in metres, row-major, N→S then W→E. NoData → `f32::NAN`.
    pub elevations: Vec<f32>,
}

impl Dem {
    /// Parse an SRTM HGT tile.
    ///
    /// `sw_lat` / `sw_lon` are the SW-corner integer degrees of the tile
    /// (e.g., `sw_lat=-23, sw_lon=-43` for file `S23W043.hgt`). The tile
    /// covers exactly `[sw_lat, sw_lat+1] × [sw_lon, sw_lon+1]` degrees.
    pub fn from_hgt(bytes: &[u8], sw_lat: i32, sw_lon: i32) -> io::Result<Self> {
        let n = match bytes.len() {
            b if b == 1201 * 1201 * 2 => 1201_usize,
            b if b == 3601 * 3601 * 2 => 3601_usize,
            b => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("unexpected HGT size: {b} bytes (expected 1201² or 3601² × 2)"),
                ))
            }
        };

        let elevations: Vec<f32> = bytes
            .chunks_exact(2)
            .map(|b| {
                let raw = i16::from_be_bytes([b[0], b[1]]);
                if raw == -32768 {
                    f32::NAN
                } else {
                    raw as f32
                }
            })
            .collect();

        Ok(Self {
            bounds: GeoBounds {
                lat_min: sw_lat as f64,
                lat_max: sw_lat as f64 + 1.0,
                lon_min: sw_lon as f64,
                lon_max: sw_lon as f64 + 1.0,
            },
            rows: n,
            cols: n,
            elevations,
        })
    }

    /// Bilinear elevation interpolation at a WGS84 point.
    ///
    /// Returns `f32::NAN` for out-of-bounds queries or when any of the four
    /// neighbouring cells is a void cell.
    pub fn elevation_at(&self, lat: f64, lon: f64) -> f32 {
        let b = self.bounds;
        if lat < b.lat_min || lat > b.lat_max || lon < b.lon_min || lon > b.lon_max {
            return f32::NAN;
        }
        // HGT rows go N→S (row 0 = lat_max), cols go W→E (col 0 = lon_min).
        let lat_span = b.lat_max - b.lat_min;
        let lon_span = b.lon_max - b.lon_min;
        let row_f = (b.lat_max - lat) / lat_span * (self.rows - 1) as f64;
        let col_f = (lon - b.lon_min) / lon_span * (self.cols - 1) as f64;

        let r0 = row_f.floor() as usize;
        let c0 = col_f.floor() as usize;
        let r1 = (r0 + 1).min(self.rows - 1);
        let c1 = (c0 + 1).min(self.cols - 1);

        let tr = row_f.fract() as f32;
        let tc = col_f.fract() as f32;

        let e = |r: usize, c: usize| self.elevations[r * self.cols + c];
        let v00 = e(r0, c0);
        let v01 = e(r0, c1);
        let v10 = e(r1, c0);
        let v11 = e(r1, c1);

        if [v00, v01, v10, v11].iter().any(|v| v.is_nan()) {
            return f32::NAN;
        }

        (v00 * (1.0 - tc) + v01 * tc) * (1.0 - tr) + (v10 * (1.0 - tc) + v11 * tc) * tr
    }

    /// Elevation statistics for valid (non-void) cells.
    pub fn elev_stats(&self) -> (f32, f32) {
        let mut lo = f32::INFINITY;
        let mut hi = f32::NEG_INFINITY;
        for &e in &self.elevations {
            if !e.is_nan() {
                lo = lo.min(e);
                hi = hi.max(e);
            }
        }
        (
            if lo.is_infinite() { 0.0 } else { lo },
            if hi.is_infinite() { 0.0 } else { hi },
        )
    }
}

// ── local grid ─────────────────────────────────────────────────────────────

/// A downsampled DEM rendered into local-plane coordinates.
///
/// Vertex layout: row-major, N→S (row 0 = northernmost), W→E (col 0 =
/// westernmost). Coordinates: `x` = east, `y` = elevation, `z` = north, all
/// in metres relative to the tile centre.
#[derive(Debug, Clone)]
pub struct LocalGrid {
    pub rows: usize,
    pub cols: usize,
    /// [x_east, y_elev, z_north] in metres, row-major.
    pub positions: Vec<[f32; 3]>,
    /// Approximate east–west extent of the tile, metres.
    pub east_m: f32,
    /// Approximate north–south extent of the tile, metres.
    pub north_m: f32,
    pub elev_min: f32,
    pub elev_max: f32,
}

impl Dem {
    /// Downsample the DEM to a `rows × cols` grid and project into the local
    /// tangent plane centred on the tile.  Pass e.g. `rows=128, cols=128` for
    /// a display-ready grid.
    pub fn to_local_grid(&self, rows: usize, cols: usize) -> LocalGrid {
        let b = self.bounds;
        let plane = LocalPlane::new((b.lat_min + b.lat_max) * 0.5, (b.lon_min + b.lon_max) * 0.5);

        let mut positions = Vec::with_capacity(rows * cols);
        let mut elev_min = f32::INFINITY;
        let mut elev_max = f32::NEG_INFINITY;

        for r in 0..rows {
            let t_r = if rows <= 1 {
                0.5
            } else {
                r as f64 / (rows - 1) as f64
            };
            // Rows go N→S: r=0 → lat_max, r=rows-1 → lat_min.
            let lat = b.lat_max - t_r * (b.lat_max - b.lat_min);

            for c in 0..cols {
                let t_c = if cols <= 1 {
                    0.5
                } else {
                    c as f64 / (cols - 1) as f64
                };
                let lon = b.lon_min + t_c * (b.lon_max - b.lon_min);

                let elev = self.elevation_at(lat, lon);
                let y = if elev.is_nan() { 0.0_f32 } else { elev };
                if !elev.is_nan() {
                    elev_min = elev_min.min(y);
                    elev_max = elev_max.max(y);
                }

                let [x, z] = plane.to_local(lat, lon);
                positions.push([x as f32, y, z as f32]);
            }
        }

        // Half-extents for the tile (used by the app to set camera distance).
        let [east_half, _] = plane.to_local((b.lat_min + b.lat_max) * 0.5, b.lon_max);
        let [_, north_half] = plane.to_local(b.lat_max, (b.lon_min + b.lon_max) * 0.5);

        LocalGrid {
            rows,
            cols,
            positions,
            east_m: (east_half.abs() * 2.0) as f32,
            north_m: (north_half.abs() * 2.0) as f32,
            elev_min: if elev_min.is_infinite() {
                0.0
            } else {
                elev_min
            },
            elev_max: if elev_max.is_infinite() {
                0.0
            } else {
                elev_max
            },
        }
    }
}

/// Generate `LINE_LIST` wireframe vertices from a `LocalGrid`.
///
/// Each pair of consecutive vertices in the returned slice forms one line
/// segment — upload directly to a `LINE_LIST` vertex buffer.
pub fn wireframe_line_list(grid: &LocalGrid) -> Vec<[f32; 3]> {
    let p = |r: usize, c: usize| grid.positions[r * grid.cols + c];
    let row_segs = grid.cols.saturating_sub(1);
    let col_segs = grid.rows.saturating_sub(1);
    let capacity = 2 * (grid.rows * row_segs + grid.cols * col_segs);
    let mut verts = Vec::with_capacity(capacity);

    // Horizontal lines (W→E within each row).
    for r in 0..grid.rows {
        for c in 0..grid.cols - 1 {
            verts.push(p(r, c));
            verts.push(p(r, c + 1));
        }
    }
    // Vertical lines (N→S within each column).
    for c in 0..grid.cols {
        for r in 0..grid.rows - 1 {
            verts.push(p(r, c));
            verts.push(p(r + 1, c));
        }
    }
    verts
}
