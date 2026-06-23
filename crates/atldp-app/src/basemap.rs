//! Optional, cached OpenStreetMap raster basemap (G10d, ADR-0025).
//!
//! **Imagery only** — the terrain numerics and elevations stay fully offline
//! (ADR-0013). Fetching is **user-triggered**, runs on a background thread, caches
//! tiles on disk, and the app falls back to hypsometric shading when tiles are
//! unavailable (offline, or a fetch fails).

use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::mpsc;

use atldp_geo::dem::GeoBounds;

/// Tile server and the identifying User-Agent. The OSM tile-usage policy requires a
/// valid User-Agent and low volume; the app fetches one window's tiles, then serves
/// from the on-disk cache. The source is a single point of change for a self-hosted
/// or permissively-licensed provider (ADR-0025).
const TILE_URL: &str = "https://tile.openstreetmap.org";
const USER_AGENT: &str = concat!(
    "atldp/",
    env!("CARGO_PKG_VERSION"),
    " (transmission-line design tool; one window of tiles, cached)"
);
const TILE_PX: u32 = 256;
/// Safety cap on tiles composited for one window (≤ 8×8) — bounds memory + requests.
const MAX_TILES: u32 = 64;

/// A composited, georeferenced map image covering (approximately) the working area.
#[derive(Clone)]
pub struct MapImage {
    /// RGBA8, row-major, `width × height`.
    pub rgba: Vec<u8>,
    pub width: u32,
    pub height: u32,
    /// Geographic extent the image covers (the slippy-tile grid rectangle).
    pub bounds: GeoBounds,
}

/// A background basemap fetch in flight; poll it each frame.
pub struct BasemapFetcher {
    rx: mpsc::Receiver<Option<MapImage>>,
}

impl BasemapFetcher {
    /// Spawn a background fetch of the basemap for `bounds`, caching tiles under
    /// `cache_dir`.
    pub fn spawn(bounds: GeoBounds, cache_dir: PathBuf) -> Self {
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let _ = tx.send(fetch_and_composite(bounds, &cache_dir));
        });
        BasemapFetcher { rx }
    }

    /// Non-blocking poll: `Some(Some(img))` once composited, `Some(None)` if the
    /// fetch failed (offline / no tiles), `None` while still working.
    pub fn poll(&mut self) -> Option<Option<MapImage>> {
        match self.rx.try_recv() {
            Ok(v) => Some(v),
            Err(mpsc::TryRecvError::Empty) => None,
            Err(mpsc::TryRecvError::Disconnected) => Some(None),
        }
    }
}

/// Default on-disk tile cache directory (`ATLDP_TILE_CACHE` overrides).
pub fn cache_dir() -> PathBuf {
    std::env::var("ATLDP_TILE_CACHE")
        .map(PathBuf::from)
        .unwrap_or_else(|_| std::env::temp_dir().join("atldp_osm_tiles"))
}

/// Slippy-map (Web Mercator) tile coordinate of a lon/lat at zoom `z` (fractional).
fn lonlat_to_tile(lon: f64, lat: f64, z: u32) -> (f64, f64) {
    let n = (1u64 << z) as f64;
    let x = (lon + 180.0) / 360.0 * n;
    let lat_rad = lat.to_radians();
    let y = (1.0 - (lat_rad.tan() + 1.0 / lat_rad.cos()).ln() / std::f64::consts::PI) / 2.0 * n;
    (x, y)
}

/// (lon west edge, lat north edge) of tile corner `(x, y)` at zoom `z`.
fn tile_nw(x: f64, y: f64, z: u32) -> (f64, f64) {
    let n = (1u64 << z) as f64;
    let lon = x / n * 360.0 - 180.0;
    let lat = (std::f64::consts::PI * (1.0 - 2.0 * y / n)).sinh().atan();
    (lon, lat.to_degrees())
}

/// Highest zoom whose tile grid stays within ~6 tiles across the window — detail
/// bounded by request count.
fn pick_zoom(b: GeoBounds) -> u32 {
    for z in (1..=17).rev() {
        let (x0, _) = lonlat_to_tile(b.lon_min, b.lat_max, z);
        let (x1, _) = lonlat_to_tile(b.lon_max, b.lat_max, z);
        let (_, y0) = lonlat_to_tile(b.lon_min, b.lat_max, z);
        let (_, y1) = lonlat_to_tile(b.lon_min, b.lat_min, z);
        let tx = (x1.floor() - x0.floor()).abs() + 1.0;
        let ty = (y1.floor() - y0.floor()).abs() + 1.0;
        if tx.max(ty) <= 6.0 {
            return z;
        }
    }
    8
}

fn fetch_and_composite(b: GeoBounds, cache_dir: &Path) -> Option<MapImage> {
    let z = pick_zoom(b);
    let (xa, _) = lonlat_to_tile(b.lon_min, b.lat_max, z);
    let (xb, _) = lonlat_to_tile(b.lon_max, b.lat_max, z);
    let (_, ya) = lonlat_to_tile(b.lon_min, b.lat_max, z);
    let (_, yb) = lonlat_to_tile(b.lon_min, b.lat_min, z);
    let nmax = (1i64 << z) - 1;
    let x0 = (xa.floor() as i64).min(xb.floor() as i64).clamp(0, nmax);
    let x1 = (xa.floor() as i64).max(xb.floor() as i64).clamp(0, nmax);
    let y0 = (ya.floor() as i64).min(yb.floor() as i64).clamp(0, nmax);
    let y1 = (ya.floor() as i64).max(yb.floor() as i64).clamp(0, nmax);

    let cols = (x1 - x0 + 1) as u32;
    let rows = (y1 - y0 + 1) as u32;
    if cols == 0 || rows == 0 || cols * rows > MAX_TILES {
        return None;
    }
    let width = cols * TILE_PX;
    let height = rows * TILE_PX;
    let mut rgba = vec![60u8; (width as usize) * (height as usize) * 4];
    for px in rgba.chunks_exact_mut(4) {
        px[3] = 255; // opaque grey where a tile is missing
    }

    let mut any = false;
    for ty in y0..=y1 {
        for tx in x0..=x1 {
            if let Some(tile) = load_or_fetch_tile(z, tx as u64, ty as u64, cache_dir) {
                any = true;
                let ox = (tx - x0) as u32 * TILE_PX;
                let oy = (ty - y0) as u32 * TILE_PX;
                blit(&mut rgba, width, &tile, ox, oy);
            }
        }
    }
    if !any {
        return None;
    }

    let (lon_w, lat_n) = tile_nw(x0 as f64, y0 as f64, z);
    let (lon_e, lat_s) = tile_nw((x1 + 1) as f64, (y1 + 1) as f64, z);
    Some(MapImage {
        rgba,
        width,
        height,
        bounds: GeoBounds {
            lat_min: lat_s,
            lat_max: lat_n,
            lon_min: lon_w,
            lon_max: lon_e,
        },
    })
}

/// Load a tile from the disk cache, else fetch it over HTTPS and cache it.
fn load_or_fetch_tile(z: u32, x: u64, y: u64, cache_dir: &Path) -> Option<image::RgbaImage> {
    let path = cache_dir.join(format!("{z}/{x}/{y}.png"));
    if let Ok(bytes) = std::fs::read(&path) {
        if let Ok(img) = image::load_from_memory(&bytes) {
            return Some(img.to_rgba8());
        }
    }
    let url = format!("{TILE_URL}/{z}/{x}/{y}.png");
    let resp = ureq::get(&url).set("User-Agent", USER_AGENT).call().ok()?;
    let mut buf = Vec::new();
    resp.into_reader().read_to_end(&mut buf).ok()?;
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(&path, &buf);
    image::load_from_memory(&buf).ok().map(|i| i.to_rgba8())
}

/// Copy a 256×256 RGBA tile into the composite at pixel offset `(ox, oy)`.
fn blit(dst: &mut [u8], dst_w: u32, tile: &image::RgbaImage, ox: u32, oy: u32) {
    let tw = tile.width();
    for row in 0..tile.height() {
        let src = tile.as_raw();
        let s = (row * tw * 4) as usize;
        let d = (((oy + row) * dst_w + ox) * 4) as usize;
        let n = (tw * 4) as usize;
        dst[d..d + n].copy_from_slice(&src[s..s + n]);
    }
}
