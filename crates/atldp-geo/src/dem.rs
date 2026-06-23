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
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GeoBounds {
    pub lat_min: f64,
    pub lat_max: f64,
    pub lon_min: f64,
    pub lon_max: f64,
}

impl GeoBounds {
    /// The overlapping window of two boxes, or `None` if they are disjoint (or
    /// touch only along an edge, which has no area).
    pub fn intersection(&self, other: &GeoBounds) -> Option<GeoBounds> {
        let b = GeoBounds {
            lat_min: self.lat_min.max(other.lat_min),
            lat_max: self.lat_max.min(other.lat_max),
            lon_min: self.lon_min.max(other.lon_min),
            lon_max: self.lon_max.min(other.lon_max),
        };
        (b.lat_max > b.lat_min && b.lon_max > b.lon_min).then_some(b)
    }

    /// The smallest box containing both inputs.
    pub fn union(&self, other: &GeoBounds) -> GeoBounds {
        GeoBounds {
            lat_min: self.lat_min.min(other.lat_min),
            lat_max: self.lat_max.max(other.lat_max),
            lon_min: self.lon_min.min(other.lon_min),
            lon_max: self.lon_max.max(other.lon_max),
        }
    }
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

    /// Latitude degrees per row step (cell spacing on the N–S axis).
    fn lat_step(&self) -> f64 {
        (self.bounds.lat_max - self.bounds.lat_min) / (self.rows.max(2) - 1) as f64
    }

    /// Longitude degrees per column step (cell spacing on the W–E axis).
    fn lon_step(&self) -> f64 {
        (self.bounds.lon_max - self.bounds.lon_min) / (self.cols.max(2) - 1) as f64
    }

    /// Crop the DEM to the smallest **cell-aligned** sub-raster that fully covers
    /// `window` (ADR-0022). The result's bounds snap outward to grid lines, so the
    /// cropped tile contains every cell `window` touches; `None` if `window` does
    /// not overlap the tile. The grid resolution is preserved — this selects a
    /// sub-rectangle of the existing samples, it does not resample.
    pub fn crop(&self, window: GeoBounds) -> Option<Dem> {
        let w = self.bounds.intersection(&window)?;
        let lat_step = self.lat_step();
        let lon_step = self.lon_step();

        // Rows run N→S (row 0 = lat_max); cols run W→E (col 0 = lon_min). Expand the
        // index range outward so the crop strictly covers the requested window.
        let r_start = ((self.bounds.lat_max - w.lat_max) / lat_step).floor() as isize;
        let r_end = ((self.bounds.lat_max - w.lat_min) / lat_step).ceil() as isize;
        let c_start = ((w.lon_min - self.bounds.lon_min) / lon_step).floor() as isize;
        let c_end = ((w.lon_max - self.bounds.lon_min) / lon_step).ceil() as isize;

        let r0 = r_start.clamp(0, self.rows as isize - 1) as usize;
        let r1 = r_end.clamp(0, self.rows as isize - 1) as usize;
        let c0 = c_start.clamp(0, self.cols as isize - 1) as usize;
        let c1 = c_end.clamp(0, self.cols as isize - 1) as usize;

        let rows = r1 - r0 + 1;
        let cols = c1 - c0 + 1;
        let mut elevations = Vec::with_capacity(rows * cols);
        for r in r0..=r1 {
            let base = r * self.cols;
            elevations.extend_from_slice(&self.elevations[base + c0..=base + c1]);
        }

        Some(Dem {
            bounds: GeoBounds {
                lat_max: self.bounds.lat_max - r0 as f64 * lat_step,
                lat_min: self.bounds.lat_max - r1 as f64 * lat_step,
                lon_min: self.bounds.lon_min + c0 as f64 * lon_step,
                lon_max: self.bounds.lon_min + c1 as f64 * lon_step,
            },
            rows,
            cols,
            elevations,
        })
    }
}

/// Stitch grid-aligned, equal-resolution DEM tiles into one raster (ADR-0022).
///
/// SRTM tiles share their seam line (tile `[n, n+1]` and `[n+1, n+2]` share the
/// `n+1` row/column), so adjacent tiles are placed on a common grid and their
/// shared edge simply coincides. Tiles need not tile the union completely — gaps
/// stay `f32::NAN`. Returns `None` for an empty input or tiles of differing
/// resolution; a single tile is returned (cloned) unchanged.
pub fn mosaic(tiles: &[Dem]) -> Option<Dem> {
    let first = tiles.first()?;
    if tiles.len() == 1 {
        return Some(first.clone());
    }
    let lat_step = first.lat_step();
    let lon_step = first.lon_step();
    // All tiles must share the grid resolution to stitch without resampling.
    if tiles.iter().any(|t| {
        (t.lat_step() - lat_step).abs() > lat_step * 1e-6
            || (t.lon_step() - lon_step).abs() > lon_step * 1e-6
    }) {
        return None;
    }

    let bounds = tiles
        .iter()
        .skip(1)
        .fold(first.bounds, |acc, t| acc.union(&t.bounds));

    let cols = ((bounds.lon_max - bounds.lon_min) / lon_step).round() as usize + 1;
    let rows = ((bounds.lat_max - bounds.lat_min) / lat_step).round() as usize + 1;
    let mut elevations = vec![f32::NAN; rows * cols];

    for t in tiles {
        // Where this tile's NW corner lands in the global grid.
        let row_off = ((bounds.lat_max - t.bounds.lat_max) / lat_step).round() as usize;
        let col_off = ((t.bounds.lon_min - bounds.lon_min) / lon_step).round() as usize;
        for r in 0..t.rows {
            let gr = row_off + r;
            if gr >= rows {
                continue;
            }
            let dst = gr * cols + col_off;
            let src = r * t.cols;
            let n = t.cols.min(cols - col_off);
            elevations[dst..dst + n].copy_from_slice(&t.elevations[src..src + n]);
        }
    }

    Some(Dem {
        bounds,
        rows,
        cols,
        elevations,
    })
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

#[cfg(test)]
mod tests {
    use super::*;

    /// A 3×3 grid over `[0,1]°×[0,1]°` (step 0.5°), elevations row-major N→S:
    /// row0 (lat 1.0) = 0,1,2; row1 (lat 0.5) = 3,4,5; row2 (lat 0.0) = 6,7,8.
    fn grid3() -> Dem {
        Dem {
            bounds: GeoBounds {
                lat_min: 0.0,
                lat_max: 1.0,
                lon_min: 0.0,
                lon_max: 1.0,
            },
            rows: 3,
            cols: 3,
            elevations: (0..9).map(|i| i as f32).collect(),
        }
    }

    #[test]
    fn crop_selects_cell_aligned_subblock() {
        let dem = grid3();
        let sub = dem
            .crop(GeoBounds {
                lat_min: 0.1,
                lat_max: 0.4,
                lon_min: 0.6,
                lon_max: 0.9,
            })
            .expect("overlaps");
        // Expands outward to the SE 2×2 block: rows 1–2, cols 1–2.
        assert_eq!((sub.rows, sub.cols), (2, 2));
        assert_eq!(sub.elevations, vec![4.0, 5.0, 7.0, 8.0]);
        assert_eq!(sub.bounds.lat_max, 0.5);
        assert_eq!(sub.bounds.lat_min, 0.0);
        assert_eq!(sub.bounds.lon_min, 0.5);
        assert_eq!(sub.bounds.lon_max, 1.0);
    }

    #[test]
    fn crop_disjoint_window_is_none() {
        assert!(grid3()
            .crop(GeoBounds {
                lat_min: 5.0,
                lat_max: 6.0,
                lon_min: 5.0,
                lon_max: 6.0,
            })
            .is_none());
    }

    #[test]
    fn mosaic_stitches_adjacent_tiles_along_shared_seam() {
        let mut a = grid3();
        // Tile B sits immediately east, sharing the lon=1.0 seam column.
        let mut b = grid3();
        b.bounds.lon_min = 1.0;
        b.bounds.lon_max = 2.0;
        // Make the seam consistent: B's west column == A's east column.
        a.elevations = vec![0., 1., 2., 3., 4., 5., 6., 7., 8.];
        b.elevations = vec![2., 30., 40., 5., 60., 70., 8., 90., 100.];

        let m = mosaic(&[a, b]).expect("stitched");
        assert_eq!((m.rows, m.cols), (3, 5));
        assert_eq!(m.bounds.lon_min, 0.0);
        assert_eq!(m.bounds.lon_max, 2.0);
        // Row 0 (lat 1.0): A's 0,1,2 then B's 30,40 past the shared seam at col 2.
        assert_eq!(&m.elevations[0..5], &[0.0, 1.0, 2.0, 30.0, 40.0]);
        // Row 2 (lat 0.0): A's 6,7,8 then B's 90,100.
        assert_eq!(&m.elevations[10..15], &[6.0, 7.0, 8.0, 90.0, 100.0]);
    }

    #[test]
    fn mosaic_single_tile_is_clone_and_empty_is_none() {
        let only = mosaic(std::slice::from_ref(&grid3())).unwrap();
        assert_eq!(only.elevations, grid3().elevations);
        assert!(mosaic(&[]).is_none());
    }
}
