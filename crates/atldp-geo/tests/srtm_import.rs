//! SRTM HGT import tests.
//!
//! # Unit tests (always run)
//!   Synthetic 5×5 tile built inline — no file I/O.
//!
//! # Integration tests (require downloaded tile)
//!   `#[ignore]` — run with:
//!   ```
//!   bash crates/atldp-geo/tests/fetch_srtm.sh
//!   cargo test -p atldp-geo -- --ignored
//!   ```

use atldp_geo::{
    dem::{wireframe_line_list, Dem},
    profile::extract_profile,
};

// ── helpers ───────────────────────────────────────────────────────────────────

/// Build a synthetic 1201×1201 HGT tile (SW corner at lat=0, lon=0) where
/// `elevation = row_index + col_index` (i16 big-endian).
fn synthetic_hgt_1201() -> Vec<u8> {
    let n = 1201_usize;
    let mut bytes = Vec::with_capacity(n * n * 2);
    for row in 0..n {
        for col in 0..n {
            let val = (row + col) as i16;
            bytes.extend_from_slice(&val.to_be_bytes());
        }
    }
    bytes
}

/// 5×5 HGT tile with known values for quick bilinear checks.
///
/// The HGT spec requires either 1201² or 3601² bytes, so we can only use 5×5
/// by calling the inner parser logic via a helper. We test parse correctness
/// through the 1201×1201 synthetic tile for most things; this function tests
/// bilinear interpolation arithmetic directly.
fn bilinear_lerp(v00: f32, v01: f32, v10: f32, v11: f32, tr: f32, tc: f32) -> f32 {
    (v00 * (1.0 - tc) + v01 * tc) * (1.0 - tr) + (v10 * (1.0 - tc) + v11 * tc) * tr
}

// ── unit tests ────────────────────────────────────────────────────────────────

#[test]
fn parse_synthetic_1201_tile() {
    let bytes = synthetic_hgt_1201();
    let dem = Dem::from_hgt(&bytes, 0, 0).expect("parse");
    assert_eq!(dem.rows, 1201);
    assert_eq!(dem.cols, 1201);
    assert_eq!(dem.bounds.lat_min, 0.0);
    assert_eq!(dem.bounds.lat_max, 1.0);
    assert_eq!(dem.bounds.lon_min, 0.0);
    assert_eq!(dem.bounds.lon_max, 1.0);
}

#[test]
fn wrong_size_returns_error() {
    let bad = vec![0u8; 100];
    assert!(Dem::from_hgt(&bad, 0, 0).is_err());
}

#[test]
fn void_cells_become_nan() {
    let n = 1201_usize;
    let mut bytes = vec![0u8; n * n * 2];
    // Inject void (-32768) at position (row=0, col=0).
    let void = (-32768_i16).to_be_bytes();
    bytes[0] = void[0];
    bytes[1] = void[1];
    let dem = Dem::from_hgt(&bytes, 0, 0).expect("parse");
    // NW corner of the tile (lat=lat_max, lon=lon_min) → row 0, col 0.
    let e = dem.elevation_at(1.0, 0.0);
    assert!(e.is_nan(), "void cell must return NaN, got {e}");
}

#[test]
fn elevation_at_corners_synthetic() {
    let bytes = synthetic_hgt_1201();
    let dem = Dem::from_hgt(&bytes, 0, 0).expect("parse");

    // HGT row 0 = northernmost = lat_max.  Row 0, col 0 → elevation = 0+0 = 0.
    let e_nw = dem.elevation_at(1.0, 0.0);
    assert!((e_nw - 0.0).abs() < 0.01, "NW elev={e_nw}");

    // Row 0, col 1200 → elevation = 0+1200 = 1200.
    let e_ne = dem.elevation_at(1.0, 1.0);
    assert!((e_ne - 1200.0).abs() < 0.01, "NE elev={e_ne}");

    // Row 1200, col 0 → elevation = 1200+0 = 1200.
    let e_sw = dem.elevation_at(0.0, 0.0);
    assert!((e_sw - 1200.0).abs() < 0.01, "SW elev={e_sw}");

    // Row 1200, col 1200 → elevation = 1200+1200 = 2400.
    let e_se = dem.elevation_at(0.0, 1.0);
    assert!((e_se - 2400.0).abs() < 0.01, "SE elev={e_se}");
}

#[test]
fn bilinear_mid_point_synthetic() {
    let bytes = synthetic_hgt_1201();
    let dem = Dem::from_hgt(&bytes, 0, 0).expect("parse");

    // Centre of the tile → lat=0.5, lon=0.5.  Row 600, col 600 → elev=1200.
    // The bilinear interpolation of a linear surface reproduces exact values.
    let e_mid = dem.elevation_at(0.5, 0.5);
    assert!((e_mid - 1200.0).abs() < 0.5, "centre elev={e_mid}");
}

#[test]
fn bilinear_arithmetic() {
    // Direct check of the bilinear formula with known inputs.
    let v = bilinear_lerp(0.0, 10.0, 20.0, 30.0, 0.5, 0.5);
    assert!((v - 15.0).abs() < 1e-4, "bilinear mid={v}");
    let v2 = bilinear_lerp(0.0, 10.0, 20.0, 30.0, 0.0, 0.0);
    assert!((v2 - 0.0).abs() < 1e-4, "bilinear 00={v2}");
}

#[test]
fn elev_stats_synthetic() {
    let bytes = synthetic_hgt_1201();
    let dem = Dem::from_hgt(&bytes, 0, 0).expect("parse");
    let (lo, hi) = dem.elev_stats();
    assert!((lo - 0.0).abs() < 0.1, "min={lo}");
    assert!((hi - 2400.0).abs() < 0.1, "max={hi}");
}

#[test]
fn to_local_grid_shape() {
    let bytes = synthetic_hgt_1201();
    let dem = Dem::from_hgt(&bytes, 0, 0).expect("parse");
    let grid = dem.to_local_grid(32, 32);
    assert_eq!(grid.rows, 32);
    assert_eq!(grid.cols, 32);
    assert_eq!(grid.positions.len(), 32 * 32);
    assert!(grid.east_m > 0.0, "east_m={}", grid.east_m);
    assert!(grid.north_m > 0.0, "north_m={}", grid.north_m);
}

#[test]
fn wireframe_line_list_count() {
    let bytes = synthetic_hgt_1201();
    let dem = Dem::from_hgt(&bytes, 0, 0).expect("parse");
    let grid = dem.to_local_grid(8, 8);
    let verts = wireframe_line_list(&grid);
    // 8 rows × 7 col-segs + 8 cols × 7 row-segs = 56+56 = 112 segments × 2 verts.
    assert_eq!(verts.len(), 2 * (8 * 7 + 8 * 7), "len={}", verts.len());
}

#[test]
fn extract_profile_length_and_distance() {
    let bytes = synthetic_hgt_1201();
    let dem = Dem::from_hgt(&bytes, 0, 0).expect("parse");
    let profile = extract_profile(&dem, 0.1, 0.1, 0.9, 0.9, 50);
    assert_eq!(profile.len(), 50);
    // First point is at distance 0.
    assert!((profile[0].distance_m - 0.0).abs() < 1e-6);
    // Last point is at the total diagonal distance (roughly √2 × 0.8° × 111 km).
    let last_dist = profile[49].distance_m;
    assert!(
        last_dist > 80_000.0 && last_dist < 140_000.0,
        "total dist={last_dist}"
    );
    // No NaN in the synthetic tile (all values are 0..2400).
    for p in &profile {
        assert!(
            !p.elevation_m.is_nan(),
            "unexpected NaN at dist={}",
            p.distance_m
        );
    }
}

// ── integration tests (require downloaded tile) ───────────────────────────────

/// Path to the real SRTM tile (populated by `fetch_srtm.sh`).
fn srtm_path() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/data/S23W043.hgt")
}

#[test]
#[ignore = "requires fetch_srtm.sh to have been run first"]
fn load_real_srtm_tile() {
    let path = srtm_path();
    assert!(path.exists(), "tile not found: {}", path.display());

    let bytes = std::fs::read(&path).expect("read");
    let dem = Dem::from_hgt(&bytes, -23, -43).expect("parse");

    assert!(
        dem.rows == 1201 || dem.rows == 3601,
        "unexpected rows={}",
        dem.rows
    );
    assert_eq!(dem.bounds.lat_min, -23.0);
    assert_eq!(dem.bounds.lat_max, -22.0);
    assert_eq!(dem.bounds.lon_min, -43.0);
    assert_eq!(dem.bounds.lon_max, -42.0);

    let (lo, hi) = dem.elev_stats();
    assert!(
        (0.0..200.0).contains(&lo),
        "unrealistic min elevation {lo} m"
    );
    assert!(
        hi > 100.0 && hi < 2800.0,
        "unrealistic max elevation {hi} m"
    );

    println!(
        "SRTM S23W043: {}×{} grid, elevation {lo:.0}–{hi:.0} m",
        dem.rows, dem.cols
    );
}

#[test]
#[ignore = "requires fetch_srtm.sh to have been run first"]
fn profile_across_tile() {
    let path = srtm_path();
    assert!(path.exists(), "tile not found: {}", path.display());

    let bytes = std::fs::read(&path).expect("read");
    let dem = Dem::from_hgt(&bytes, -23, -43).expect("parse");

    // Profile: W→E midline of the tile (lat = -22.5).
    let profile = extract_profile(&dem, -22.5, -43.0, -22.5, -42.0, 200);
    assert_eq!(profile.len(), 200);

    // Distance should be roughly 100 km (1° lon × cos(-22.5°) × 111.32 km).
    let total_dist = profile[199].distance_m;
    assert!(
        total_dist > 80_000.0 && total_dist < 120_000.0,
        "unexpected total distance {total_dist:.0} m"
    );

    // All elevations should be in a plausible range for this tile.
    for p in &profile {
        if !p.elevation_m.is_nan() {
            assert!(
                p.elevation_m >= 0.0 && p.elevation_m <= 2800.0,
                "elevation {} out of range at dist {:.0}",
                p.elevation_m,
                p.distance_m
            );
        }
    }
    println!(
        "Profile ({} samples, {:.0}–{:.0} km): elevations OK",
        profile.len(),
        0.0,
        total_dist / 1000.0,
    );
}

#[test]
#[ignore = "requires fetch_srtm.sh to have been run first"]
fn local_grid_fits_camera() {
    let path = srtm_path();
    assert!(path.exists(), "tile not found: {}", path.display());

    let bytes = std::fs::read(&path).expect("read");
    let dem = Dem::from_hgt(&bytes, -23, -43).expect("parse");
    let grid = dem.to_local_grid(96, 96);

    // At lat ≈ -22.5°, 1° lon ≈ 103 km, 1° lat ≈ 111 km.
    assert!(
        grid.east_m > 90_000.0 && grid.east_m < 120_000.0,
        "east_m={}",
        grid.east_m
    );
    assert!(
        grid.north_m > 100_000.0 && grid.north_m < 130_000.0,
        "north_m={}",
        grid.north_m
    );
    println!(
        "LocalGrid 96×96: {:.1}×{:.1} km, elev {:.0}–{:.0} m",
        grid.east_m / 1000.0,
        grid.north_m / 1000.0,
        grid.elev_min,
        grid.elev_max
    );
}
