//! ATLDP desktop CAD application — manual spotting through G10c (ADR-0011, ADR-0012).
//!
//! Extends G3 terrain/route with:
//!   - A **plan-view tab** rendering the cropped DEM with the **editable route**
//!     drawn on it: place/drag/kind `Poi` vertices, the ground profile re-derived
//!     live on every edit (G10c, ADR-0022)
//!   - Click-to-place tower spotting on the 2D terrain profile
//!   - Live catenary computation between consecutive towers
//!   - Ground clearance check with colour-coded violation highlights
//!   - Tower and conductor geometry in the 3D viewport (SpottingLines renderer)
//!   - An editable tower table (function / family / height) that re-partitions
//!     tension sections live (G9, ADR-0019); the saved project carries the explicit
//!     `Route` the ground profile is derived from. Structure function is
//!     suspension-or-anchor; "angle" is a deflection property (ADR-0023)
//!   - A tower-elevation tab drawing the selected structure's silhouette and
//!     per-wire attachment points (G10, ADR-0020)
//!
//! Terrain file: set `ATLDP_TERRAIN=/path/to/tile.hgt` or place
//! `S23W043.hgt` in `crates/atldp-geo/tests/data/` (see fetch_srtm.sh).

use std::sync::Arc;

use egui_dock::TabViewer;
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowAttributes, WindowId},
};

use atldp_geo::{
    crs::LocalPlane,
    dem::{mosaic, wireframe_line_list, Dem, GeoBounds, LocalGrid},
    profile::{extract_profile_polyline, ProfilePoint},
};
use atldp_model::{
    analysis, format, report, sheet, ConductorSpec, Poi, PoiKind, ProfileSample, Project, Route,
    StructureFamily, StructureFunction, TerrainRef, TerrainTile, Tower, TowerFamily,
};
use atldp_render::{
    camera::{Camera2D, OrbitCamera},
    catenary_line::CatenaryLineResources,
    spotting_lines::{SpottingCallback, SpottingResources, SpottingVertex},
    terrain_mesh::{TerrainMeshResources, TerrainVertex},
};

// ── conductor weight ──────────────────────────────────────────────────────────
const CONDUCTOR_W_N_PER_M: f64 = 15.97; // ACSR Drake
const CATENARY_SAMPLES: usize = 80;

// ── G6 output paths ───────────────────────────────────────────────────────────
// Dependency-free I/O, consistent with the env-var terrain loading: the project
// and its drafting outputs are written to fixed names in the working directory
// (override the project path with `ATLDP_PROJECT`). A native file dialog is a
// later refinement, not a phase-G6 deliverable.
const REPORT_PATH: &str = "atldp_report.md";
const SHEET_PATH: &str = "atldp_profile.svg";
const DEFAULT_PROJECT_PATH: &str = "atldp_project.atldp";

// ── terrain state (optional) ──────────────────────────────────────────────────

/// Profile samples taken along each route leg (ADR-0019 derived profile).
const SAMPLES_PER_SEGMENT: usize = 60;

struct TerrainData {
    /// The working DEM — a chosen window cropped/mosaicked from the source tiles
    /// (G10c, ADR-0022). Source of truth for elevation queries and route sampling.
    dem: Dem,
    /// Local tangent plane centred on the working DEM (shared by the grid, the
    /// route, and the plan view, so they line up).
    plane: LocalPlane,
    grid: LocalGrid,
    wireframe: Vec<TerrainVertex>,
    /// The **editable route**: ordered POIs the ground profile is derived from
    /// (G9/G10c). Drawn and edited on the plan view; the diagonal seed is just a
    /// starting point.
    route: Vec<Poi>,
    /// Ground profile sampled along the route polyline — **derived** from
    /// [`route`](Self::route) (ADR-0019); re-extracted on every route edit.
    profile: Vec<ProfilePoint>,
    /// Total route length, metres (the last POI's station).
    profile_total_dist: f64,
    /// Provenance of the DEM (tile set + working bounds), carried into projects.
    source: TerrainRef,
}

impl TerrainData {
    /// Load a single HGT tile and adopt the whole tile as the working area — the
    /// headless/dev fallback (`ATLDP_TERRAIN`). The GUI area-selection step
    /// (ADR-0022) instead crops/mosaics a chosen window via [`Self::from_dem`].
    fn load(path: &std::path::Path, sw_lat: i32, sw_lon: i32) -> Option<Self> {
        let bytes = std::fs::read(path).ok()?;
        let dem = Dem::from_hgt(&bytes, sw_lat, sw_lon)
            .map_err(|e| eprintln!("terrain: HGT parse error: {e}"))
            .ok()?;
        eprintln!(
            "terrain: loaded {} ({} rows × {} cols)",
            path.display(),
            dem.rows,
            dem.cols,
        );
        let source = TerrainRef::single_tile(path.display().to_string(), sw_lat, sw_lon);
        Self::from_dem(dem, source)
    }

    /// Resolve a project's terrain set (mosaic the tiles, crop to the working
    /// bounds) and build the live state. Tiles are located by re-deriving their
    /// `HGT` filenames next to a reference path (the loaded project / env tile),
    /// falling back to the recorded `source_path`.
    fn from_terrain_ref(source: &TerrainRef, search_dir: Option<&std::path::Path>) -> Option<Self> {
        let mut dems = Vec::new();
        for tile in &source.tiles {
            let dem = load_tile(tile, search_dir)?;
            dems.push(dem);
        }
        let stitched = mosaic(&dems)?;
        let b = source.bounds;
        let window = GeoBounds {
            lat_min: b.lat_min,
            lat_max: b.lat_max,
            lon_min: b.lon_min,
            lon_max: b.lon_max,
        };
        let dem = stitched.crop(window).unwrap_or(stitched);
        Self::from_dem(dem, source.clone())
    }

    /// Build terrain state from an already-resolved working DEM and its provenance,
    /// seeding an editable two-terminal route across the area (the engineer then
    /// draws the real route on the plan view).
    fn from_dem(dem: Dem, source: TerrainRef) -> Option<Self> {
        let b = dem.bounds;
        let plane = LocalPlane::new((b.lat_min + b.lat_max) * 0.5, (b.lon_min + b.lon_max) * 0.5);
        let grid = dem.to_local_grid(96, 96);
        let wireframe: Vec<TerrainVertex> = wireframe_line_list(&grid)
            .iter()
            .map(|&pos| TerrainVertex { pos })
            .collect();

        // Seed a diagonal terminal→terminal route 10 % in from the area corners;
        // stations, ground elevations and the profile are filled by `recompute`.
        let lerp = |a: f64, c: f64, t: f64| a + t * (c - a);
        let route = vec![
            Poi::terminal(
                lerp(b.lat_min, b.lat_max, 0.1),
                lerp(b.lon_min, b.lon_max, 0.1),
                0.0,
            ),
            Poi::terminal(
                lerp(b.lat_min, b.lat_max, 0.9),
                lerp(b.lon_min, b.lon_max, 0.9),
                0.0,
            ),
        ];

        let mut t = Self {
            dem,
            plane,
            grid,
            wireframe,
            route,
            profile: Vec::new(),
            profile_total_dist: 1.0,
            source,
        };
        t.recompute();
        Some(t)
    }

    /// Re-station the POIs, re-sample their ground elevations and deviation angles,
    /// and re-derive the ground profile from the route polyline (ADR-0019/0022).
    /// Call after any route edit.
    fn recompute(&mut self) {
        let n = self.route.len();
        if n > 0 {
            self.route[0].distance_m = 0.0;
        }
        for i in 1..n {
            let (a, b) = (&self.route[i - 1], &self.route[i]);
            let d = self.plane.distance(a.lat, a.lon, b.lat, b.lon);
            self.route[i].distance_m = self.route[i - 1].distance_m + d;
        }
        // Pre-project to the local plane for the (interior) deflection angles.
        let locs: Vec<[f64; 2]> = self
            .route
            .iter()
            .map(|p| self.plane.to_local(p.lat, p.lon))
            .collect();
        for i in 0..n {
            let (lat, lon) = (self.route[i].lat, self.route[i].lon);
            let e = self.dem.elevation_at(lat, lon);
            let dev = if i == 0 || i + 1 >= n {
                0.0
            } else {
                deflection_deg(locs[i - 1], locs[i], locs[i + 1])
            };
            let p = &mut self.route[i];
            p.ground_elevation_m = if e.is_nan() { 0.0 } else { e as f64 };
            p.deviation_angle_deg = dev;
        }
        let verts: Vec<(f64, f64)> = self.route.iter().map(|p| (p.lat, p.lon)).collect();
        self.profile = extract_profile_polyline(&self.dem, &verts, SAMPLES_PER_SEGMENT);
        self.profile_total_dist = self
            .route
            .last()
            .map(|p| p.distance_m)
            .unwrap_or(0.0)
            .max(1.0);
    }

    /// Terrain elevation at a given profile distance (linear interpolation).
    fn elev_at(&self, dist_m: f64) -> f32 {
        if self.profile.is_empty() {
            return 0.0;
        }
        let first = &self.profile[0];
        let last = self.profile.last().unwrap();
        if dist_m <= first.distance_m {
            return first.elevation_m;
        }
        if dist_m >= last.distance_m {
            return last.elevation_m;
        }
        let idx = self.profile.partition_point(|p| p.distance_m < dist_m);
        let p1 = &self.profile[idx - 1];
        let p2 = &self.profile[idx];
        let t = ((dist_m - p1.distance_m) / (p2.distance_m - p1.distance_m)) as f32;
        if p1.elevation_m.is_nan() || p2.elevation_m.is_nan() {
            return f32::NAN;
        }
        p1.elevation_m + t * (p2.elevation_m - p1.elevation_m)
    }

    /// (east, north) in local-plane space for a station along the **route
    /// polyline** — walks the legs by arc length, so towers stationed past an angle
    /// point land on the bent route, not a straight chord.
    fn east_north_at(&self, dist_m: f64) -> [f32; 2] {
        if self.route.is_empty() {
            return [0.0, 0.0];
        }
        let locs: Vec<[f64; 2]> = self
            .route
            .iter()
            .map(|p| self.plane.to_local(p.lat, p.lon))
            .collect();
        let last = self.route.len() - 1;
        if dist_m <= self.route[0].distance_m {
            return [locs[0][0] as f32, locs[0][1] as f32];
        }
        if dist_m >= self.route[last].distance_m {
            return [locs[last][0] as f32, locs[last][1] as f32];
        }
        let idx = self.route.partition_point(|p| p.distance_m < dist_m).max(1);
        let d0 = self.route[idx - 1].distance_m;
        let d1 = self.route[idx].distance_m;
        let t = if d1 > d0 {
            (dist_m - d0) / (d1 - d0)
        } else {
            0.0
        };
        let e = locs[idx - 1][0] + t * (locs[idx][0] - locs[idx - 1][0]);
        let nth = locs[idx - 1][1] + t * (locs[idx][1] - locs[idx - 1][1]);
        [e as f32, nth as f32]
    }
}

/// Locate and parse one source tile. Tries the recorded `source_path`, then the
/// canonical `±NN±EEE.hgt` name beside `search_dir`.
fn load_tile(tile: &TerrainTile, search_dir: Option<&std::path::Path>) -> Option<Dem> {
    let name = hgt_name(tile.sw_lat, tile.sw_lon);
    let candidates = [
        Some(std::path::PathBuf::from(&tile.source_path)),
        search_dir.map(|d| d.join(&name)),
        tile.source_path
            .rfind(['/', '\\'])
            .map(|i| std::path::PathBuf::from(&tile.source_path[..=i]).join(&name)),
    ];
    for path in candidates.into_iter().flatten() {
        if let Ok(bytes) = std::fs::read(&path) {
            if let Ok(dem) = Dem::from_hgt(&bytes, tile.sw_lat, tile.sw_lon) {
                return Some(dem);
            }
        }
    }
    eprintln!("terrain: could not find tile {name}");
    None
}

/// The canonical SRTM HGT filename for a tile's SW corner (e.g. `S23W043.hgt`).
fn hgt_name(sw_lat: i32, sw_lon: i32) -> String {
    let (ns, lat) = if sw_lat < 0 {
        ('S', -sw_lat)
    } else {
        ('N', sw_lat)
    };
    let (ew, lon) = if sw_lon < 0 {
        ('W', -sw_lon)
    } else {
        ('E', sw_lon)
    };
    format!("{ns}{lat:02}{ew}{lon:03}.hgt")
}

/// Plan deflection at the middle of three local-plane points, degrees (0 = the
/// route runs straight through). The unsigned angle between the in/out legs.
fn deflection_deg(a: [f64; 2], b: [f64; 2], c: [f64; 2]) -> f64 {
    let v1 = [b[0] - a[0], b[1] - a[1]];
    let v2 = [c[0] - b[0], c[1] - b[1]];
    let dot = v1[0] * v2[0] + v1[1] * v2[1];
    let cross = v1[0] * v2[1] - v1[1] * v2[0];
    cross.atan2(dot).to_degrees().abs()
}

/// Hypsometric colour for an elevation between `lo` and `hi`: green lowland →
/// tan → pale highland (plan-view shading only).
fn hypso_color(elev: f32, lo: f32, hi: f32) -> egui::Color32 {
    let t = ((elev - lo) / (hi - lo)).clamp(0.0, 1.0);
    const STOPS: [(f32, [u8; 3]); 4] = [
        (0.0, [58, 104, 64]),
        (0.4, [120, 138, 72]),
        (0.7, [156, 124, 84]),
        (1.0, [228, 228, 230]),
    ];
    let mut i = 1;
    while i < STOPS.len() && t > STOPS[i].0 {
        i += 1;
    }
    let (t0, c0) = STOPS[i - 1];
    let (t1, c1) = STOPS[i.min(STOPS.len() - 1)];
    let f = if t1 > t0 { (t - t0) / (t1 - t0) } else { 0.0 };
    let mix = |a: u8, b: u8| (a as f32 + f * (b as f32 - a as f32)) as u8;
    egui::Color32::from_rgb(mix(c0[0], c1[0]), mix(c0[1], c1[1]), mix(c0[2], c1[2]))
}

/// Marker colour by POI kind on the plan view.
fn poi_color(kind: PoiKind) -> egui::Color32 {
    match kind {
        PoiKind::Terminal => egui::Color32::from_rgb(255, 120, 120),
        PoiKind::Angle => egui::Color32::from_rgb(255, 210, 90),
        PoiKind::Crossing => egui::Color32::from_rgb(120, 200, 255),
        PoiKind::Obstacle => egui::Color32::from_rgb(255, 160, 80),
        PoiKind::Constraint => egui::Color32::from_rgb(190, 190, 190),
    }
}

/// Index of the route leg (1..len) nearest a screen point — where an inserted POI
/// is spliced in. `len` (append) when there are fewer than two points.
fn nearest_leg_index(scr: &[egui::Pos2], p: egui::Pos2) -> usize {
    if scr.len() < 2 {
        return scr.len();
    }
    let mut best = 1;
    let mut best_d = f32::INFINITY;
    for i in 1..scr.len() {
        let d = point_seg_dist(p, scr[i - 1], scr[i]);
        if d < best_d {
            best_d = d;
            best = i;
        }
    }
    best
}

/// Distance from point `p` to segment `a`–`b`, pixels.
fn point_seg_dist(p: egui::Pos2, a: egui::Pos2, b: egui::Pos2) -> f32 {
    let ab = b - a;
    let len2 = ab.length_sq();
    let t = if len2 > 0.0 {
        ((p - a).dot(ab) / len2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    (p - (a + ab * t)).length()
}

// ── tab identifiers ───────────────────────────────────────────────────────────

#[derive(Clone, PartialEq)]
enum Tab {
    /// Top-down map: the cropped DEM with the editable route drawn on it
    /// (G10c, ADR-0022) — the plan-frame sibling of the profile and elevation.
    PlanView,
    View3D,
    View2D,
    /// Structure-scale elevation of the selected tower (G10, ADR-0020).
    TowerElevation,
}

// ── application ───────────────────────────────────────────────────────────────

#[derive(Default)]
struct App {
    state: Option<AppState>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.state.is_some() {
            return;
        }
        let attrs = WindowAttributes::default()
            .with_title("ATLDP — route authoring, spotting & structures (G10c)")
            .with_inner_size(winit::dpi::LogicalSize::new(1400u32, 860u32));
        let window = Arc::new(event_loop.create_window(attrs).expect("window"));
        self.state = Some(AppState::new(window));
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        if let Some(s) = &mut self.state {
            s.on_event(event_loop, event);
        }
    }

    fn about_to_wait(&mut self, _: &ActiveEventLoop) {
        if let Some(s) = &self.state {
            s.window.request_redraw();
        }
    }
}

// ── state ─────────────────────────────────────────────────────────────────────

struct AppState {
    window: Arc<Window>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: wgpu::Surface<'static>,
    surface_config: wgpu::SurfaceConfiguration,

    egui_ctx: egui::Context,
    egui_winit: egui_winit::State,
    egui_renderer: egui_wgpu::Renderer,

    dock_state: egui_dock::DockState<Tab>,

    orbit: OrbitCamera,
    cam2d: Camera2D,

    h_tension_n: f64,
    terrain: Option<TerrainData>,

    // G5 spotting state
    towers: Vec<Tower>,
    spotting_mode: bool,
    attachment_height_m: f64,
    min_clearance_m: f64,

    // G10c route authoring (ADR-0022): draw/move/kind POIs on the plan view.
    route_edit_mode: bool,
    selected_poi: Option<usize>,
    /// Pending "enabling route edit will clear spotted towers" confirmation (G10d).
    confirm_route_edit: bool,

    // G8/G10 structure-family library + the tower selected for the elevation view.
    families: Vec<StructureFamily>,
    selected_tower: Option<usize>,

    // G6 structure/drafting/file-format state
    wind_pressure_pa: f64,
    /// Transient status line for save/load/export feedback.
    status: String,

    // G10d drafting previews (shown in a window before a native "save as…").
    report_preview: Option<String>,
    sheet_svg: Option<String>,
    sheet_texture: Option<egui::TextureHandle>,
}

impl AppState {
    fn new(window: Arc<Window>) -> Self {
        pollster::block_on(Self::init(window))
    }

    async fn init(window: Arc<Window>) -> Self {
        let size = window.inner_size();

        let mut instance_desc = wgpu::InstanceDescriptor::new_without_display_handle();
        instance_desc.backends = wgpu::Backends::PRIMARY;
        let instance = wgpu::Instance::new(instance_desc);

        let surface = instance
            .create_surface(Arc::clone(&window))
            .expect("wgpu surface");

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .expect("adapter");

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                ..Default::default()
            })
            .await
            .expect("device");

        let caps = surface.get_capabilities(&adapter);
        let fmt = caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(caps.formats[0]);

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: fmt,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &surface_config);

        let egui_ctx = egui::Context::default();
        let viewport_id = egui::ViewportId::ROOT;
        let egui_winit = egui_winit::State::new(
            egui_ctx.clone(),
            viewport_id,
            &window,
            Some(window.scale_factor() as f32),
            None,
            Some(2048),
        );

        let mut egui_renderer =
            egui_wgpu::Renderer::new(&device, fmt, egui_wgpu::RendererOptions::default());
        egui_renderer
            .callback_resources
            .insert(CatenaryLineResources::new(&device, fmt));
        egui_renderer
            .callback_resources
            .insert(TerrainMeshResources::new(&device, fmt));
        egui_renderer
            .callback_resources
            .insert(SpottingResources::new(&device, fmt));

        let mut dock_state = egui_dock::DockState::new(vec![Tab::PlanView, Tab::View3D]);
        let [_, right] = dock_state.main_surface_mut().split_right(
            egui_dock::NodeIndex::root(),
            0.5,
            vec![Tab::View2D, Tab::TowerElevation],
        );
        let _ = right;

        let terrain = Self::try_load_terrain();
        let orbit = Self::init_camera(&terrain);

        let cam2d = if let Some(ref t) = terrain {
            let mid_dist = t
                .profile
                .last()
                .map(|p| p.distance_m as f32 * 0.5)
                .unwrap_or(150.0);
            let mid_elev = t.grid.elev_min + (t.grid.elev_max - t.grid.elev_min) * 0.5;
            let mut c = Camera2D::new();
            c.center = [mid_dist, mid_elev];
            c.pixels_per_metre = 0.008;
            c.vertical_exag = 10.0;
            c
        } else {
            Camera2D::new()
        };

        Self {
            window,
            device,
            queue,
            surface,
            surface_config,
            egui_ctx,
            egui_winit,
            egui_renderer,
            dock_state,
            orbit,
            cam2d,
            h_tension_n: 30_000.0,
            terrain,
            towers: Vec::new(),
            spotting_mode: false,
            attachment_height_m: 15.0,
            min_clearance_m: 8.0,
            route_edit_mode: false,
            selected_poi: None,
            confirm_route_edit: false,
            families: StructureFamily::built_in_library(),
            selected_tower: None,
            wind_pressure_pa: 700.0,
            status: String::new(),
            report_preview: None,
            sheet_svg: None,
            sheet_texture: None,
        }
    }

    fn try_load_terrain() -> Option<TerrainData> {
        if let Ok(path) = std::env::var("ATLDP_TERRAIN") {
            let p = std::path::Path::new(&path);
            if let Some((sw_lat, sw_lon)) = parse_hgt_name(p) {
                return TerrainData::load(p, sw_lat, sw_lon);
            }
            eprintln!("terrain: could not parse lat/lon from ATLDP_TERRAIN filename");
        }
        let default = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../atldp-geo/tests/data/S23W043.hgt");
        if default.exists() {
            return TerrainData::load(&default, -23, -43);
        }
        eprintln!("terrain: no HGT file found — running without terrain.");
        eprintln!("         Set ATLDP_TERRAIN=/path/to/tile.hgt");
        None
    }

    fn init_camera(terrain: &Option<TerrainData>) -> OrbitCamera {
        if let Some(t) = terrain {
            let half_east = t.grid.east_m * 0.5;
            let half_north = t.grid.north_m * 0.5;
            let mid_elev = (t.grid.elev_min + t.grid.elev_max) * 0.5;
            let extent = half_east.max(half_north);
            let mut c = OrbitCamera::new();
            c.target = glam::Vec3::new(0.0, mid_elev, 0.0);
            c.distance = extent * 1.4;
            c.pitch = 0.45;
            c.yaw = 0.5;
            c.far = extent * 8.0;
            c
        } else {
            OrbitCamera::new()
        }
    }

    // ── G6: project model, file format, drafting ────────────────────────────

    fn project_path() -> std::path::PathBuf {
        std::env::var("ATLDP_PROJECT")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| std::path::PathBuf::from(DEFAULT_PROJECT_PATH))
    }

    /// Snapshot the current spotting/terrain/parameters into a serializable project.
    fn build_project(&self) -> Project {
        let mut p = Project::new("ATLDP line");
        // The GUI edits a single representative phase wire (G5/G6 behaviour); the
        // multi-wire set (G7) is authored in the file or via the model API.
        p.wires = vec![
            atldp_model::Wire::phase("Phase", ConductorSpec::drake(), 0.0, 0.0)
                .strung(self.h_tension_n),
        ];
        p.parameters.horizontal_tension_n = self.h_tension_n;
        p.parameters.attachment_height_m = self.attachment_height_m;
        p.parameters.min_clearance_m = self.min_clearance_m;
        p.parameters.wind_pressure_pa = self.wind_pressure_pa;
        p.families = self.families.clone();
        if let Some(t) = &self.terrain {
            p.terrain = Some(t.source.clone());
            p.ground_profile = t
                .profile
                .iter()
                .filter(|pp| pp.elevation_m.is_finite())
                .map(|pp| ProfileSample {
                    distance_m: pp.distance_m,
                    elevation_m: pp.elevation_m as f64,
                })
                .collect();
            // G9/G10c (ADR-0019/0022): the profile is *derived* from the explicit,
            // user-drawn route. The plan-view editor keeps `t.route` stationed and
            // ground-sampled (see `TerrainData::recompute`), so it is carried into
            // the project verbatim and the stored profile is its sampling.
            p.route = Some(Route {
                pois: t.route.clone(),
            });
        }
        p.towers = self.towers.clone();
        p
    }

    /// Apply a loaded project's towers and parameters onto the current session
    /// (the terrain itself stays as loaded from its HGT tile).
    fn apply_project(&mut self, p: Project) {
        // Read the representative wire's tension if present, else the parameter.
        self.h_tension_n = p
            .wires
            .first()
            .map(|w| w.tension_n)
            .unwrap_or(p.parameters.horizontal_tension_n);
        self.attachment_height_m = p.parameters.attachment_height_m;
        self.min_clearance_m = p.parameters.min_clearance_m;
        self.wind_pressure_pa = p.parameters.wind_pressure_pa;
        if !p.families.is_empty() {
            self.families = p.families;
        }
        // Re-resolve the terrain set (mosaic + crop to the saved bounds) and adopt
        // the saved route, so the loaded project draws on its own area (ADR-0022).
        if let Some(ref tref) = p.terrain {
            let dir = Self::project_path().parent().map(|d| d.to_path_buf());
            if let Some(mut terrain) = TerrainData::from_terrain_ref(tref, dir.as_deref()) {
                if let Some(route) = p.route.filter(|r| r.pois.len() >= 2) {
                    terrain.route = route.pois;
                    terrain.recompute();
                }
                self.terrain = Some(terrain);
            }
        }
        self.towers = p.towers;
        self.selected_tower = None;
        self.selected_poi = None;
    }

    fn save_project(&mut self) {
        let project = self.build_project();
        let default = Self::project_path();
        let Some(path) = rfd::FileDialog::new()
            .set_title("Save ATLDP project")
            .add_filter("ATLDP project", &["atldp"])
            .set_file_name(file_name_or(&default, DEFAULT_PROJECT_PATH))
            .save_file()
        else {
            return; // cancelled
        };
        self.status = match format::save(&project, &path) {
            Ok(()) => format!("Saved project → {}", path.display()),
            Err(e) => format!("Save failed: {e}"),
        };
    }

    fn load_project(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .set_title("Open ATLDP project")
            .add_filter("ATLDP project", &["atldp"])
            .pick_file()
        else {
            return; // cancelled
        };
        match format::load(&path) {
            Ok(p) => {
                let n = p.towers.len();
                self.apply_project(p);
                self.status = format!("Loaded {n} towers from {}", path.display());
            }
            Err(e) => self.status = format!("Load failed: {e}"),
        }
    }

    /// Build the calculation report and open its preview window; the actual write
    /// happens from the preview's "Save…" button (G10d).
    fn open_report_preview(&mut self) {
        let project = self.build_project();
        self.report_preview = Some(report::markdown(&project));
        self.status = "Calculation report ready — review and save.".to_string();
    }

    /// Build the plan-&-profile SVG sheet, rasterize a preview thumbnail, and open
    /// its preview window; the write happens from the preview's "Save…" button.
    fn open_sheet_preview(&mut self, ctx: &egui::Context) {
        let project = self.build_project();
        let svg = sheet::plan_profile_svg_with(
            &project,
            &analysis::analyze(&project),
            self.cam2d.vertical_exag as f64,
            false,
        );
        self.sheet_texture = rasterize_svg(&svg)
            .map(|img| ctx.load_texture("sheet_preview", img, egui::TextureOptions::LINEAR));
        self.sheet_svg = Some(svg);
        self.status = "Plan & profile sheet ready — review and save.".to_string();
    }

    fn on_event(&mut self, event_loop: &ActiveEventLoop, event: WindowEvent) {
        let resp = self.egui_winit.on_window_event(&self.window, &event);
        if resp.consumed {
            match &event {
                WindowEvent::Resized(sz) => self.resize(*sz),
                WindowEvent::CloseRequested => event_loop.exit(),
                WindowEvent::RedrawRequested => self.render(),
                _ => {}
            }
            return;
        }
        match event {
            WindowEvent::Resized(sz) => self.resize(sz),
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::RedrawRequested => self.render(),
            _ => {}
        }
    }

    fn resize(&mut self, size: winit::dpi::PhysicalSize<u32>) {
        if size.width == 0 || size.height == 0 {
            return;
        }
        self.surface_config.width = size.width;
        self.surface_config.height = size.height;
        self.surface.configure(&self.device, &self.surface_config);
    }

    fn render(&mut self) {
        let surface_texture = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(t) => t,
            wgpu::CurrentSurfaceTexture::Suboptimal(t) => t,
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                self.surface.configure(&self.device, &self.surface_config);
                return;
            }
            e => {
                eprintln!("surface: {e:?}");
                return;
            }
        };
        let view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let size = self.window.inner_size();
        let ppp = self.egui_ctx.pixels_per_point();

        let ctx = self.egui_ctx.clone();
        let raw_input = self.egui_winit.take_egui_input(&self.window);
        let full_output = ctx.run_ui(raw_input, |ui| {
            self.build_ui(ui);
        });

        self.egui_winit
            .handle_platform_output(&self.window, full_output.platform_output);

        let primitives = ctx.tessellate(full_output.shapes, full_output.pixels_per_point);
        let screen_desc = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [size.width, size.height],
            pixels_per_point: ppp,
        };

        for (id, delta) in &full_output.textures_delta.set {
            self.egui_renderer
                .update_texture(&self.device, &self.queue, *id, delta);
        }

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

        self.egui_renderer.update_buffers(
            &self.device,
            &self.queue,
            &mut encoder,
            &primitives,
            &screen_desc,
        );

        {
            let mut rpass = encoder
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("atldp main"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        depth_slice: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                r: 0.10,
                                g: 0.10,
                                b: 0.11,
                                a: 1.0,
                            }),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                })
                .forget_lifetime();
            self.egui_renderer
                .render(&mut rpass, &primitives, &screen_desc);
        }

        for id in &full_output.textures_delta.free {
            self.egui_renderer.free_texture(id);
        }

        self.queue.submit([encoder.finish()]);
        surface_texture.present();
    }

    fn build_ui(&mut self, ui: &mut egui::Ui) {
        let size = self.window.inner_size();
        let aspect = size.width as f32 / size.height.max(1) as f32;
        let view_proj = self.orbit.view_proj(aspect).to_cols_array_2d();

        // ── toolbar ──
        egui::Panel::top("toolbar")
            .exact_size(32.0)
            .show_inside(ui, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.strong("ATLDP");
                    ui.separator();

                    if let Some(ref t) = self.terrain {
                        ui.colored_label(
                            egui::Color32::from_rgb(80, 200, 120),
                            format!(
                                "Terrain {:.0}×{:.0} km  {:.0}–{:.0} m",
                                t.grid.east_m / 1000.0,
                                t.grid.north_m / 1000.0,
                                t.grid.elev_min,
                                t.grid.elev_max,
                            ),
                        );
                    } else {
                        ui.colored_label(egui::Color32::from_gray(120), "No terrain");
                    }

                    ui.separator();

                    // Spotting toggle
                    let spot_label = if self.spotting_mode {
                        "⬛ Stop spotting"
                    } else {
                        "🗼 Spot towers"
                    };
                    if ui.button(spot_label).clicked() {
                        self.spotting_mode = !self.spotting_mode;
                    }

                    // G10c (ADR-0022): draw/move route POIs on the plan view.
                    let route_label = if self.route_edit_mode {
                        "⬛ Stop route edit"
                    } else {
                        "✏ Route edit"
                    };
                    if ui.button(route_label).clicked() {
                        if self.route_edit_mode {
                            // Leaving edit mode never destroys anything.
                            self.route_edit_mode = false;
                            self.selected_poi = None;
                        } else if self.towers.is_empty() {
                            self.route_edit_mode = true;
                        } else {
                            // Editing the route re-stations the line and orphans the
                            // spotted towers — confirm before clearing them (G10d).
                            self.confirm_route_edit = true;
                        }
                    }

                    ui.label("Attach h:");
                    ui.add(
                        egui::DragValue::new(&mut self.attachment_height_m)
                            .range(5.0..=80.0)
                            .speed(0.5)
                            .suffix(" m"),
                    );

                    ui.label("Min clear:");
                    ui.add(
                        egui::DragValue::new(&mut self.min_clearance_m)
                            .range(1.0..=30.0)
                            .speed(0.1)
                            .suffix(" m"),
                    );

                    ui.label("H tens:");
                    ui.add(
                        egui::DragValue::new(&mut self.h_tension_n)
                            .range(500.0..=300_000.0)
                            .speed(100.0)
                            .suffix(" N"),
                    );

                    ui.label("V.Exag:");
                    ui.add(
                        egui::DragValue::new(&mut self.cam2d.vertical_exag)
                            .range(1.0..=50.0)
                            .speed(0.1)
                            .suffix("×"),
                    );

                    ui.label("Wind:");
                    ui.add(
                        egui::DragValue::new(&mut self.wind_pressure_pa)
                            .range(0.0..=3000.0)
                            .speed(10.0)
                            .suffix(" Pa"),
                    );

                    ui.separator();
                    // G6: project file format + drafting exports.
                    if ui.button("💾 Save").clicked() {
                        self.save_project();
                    }
                    if ui.button("📂 Load").clicked() {
                        self.load_project();
                    }
                    if ui.button("📄 Report").clicked() {
                        self.open_report_preview();
                    }
                    if ui.button("🖼 Sheet").clicked() {
                        let ctx = ui.ctx().clone();
                        self.open_sheet_preview(&ctx);
                    }

                    ui.separator();
                    ui.label(format!("Towers: {}", self.towers.len()));

                    if ui.button("Undo").clicked() && !self.towers.is_empty() {
                        self.towers.pop();
                    }
                    if ui.button("Clear").clicked() {
                        self.towers.clear();
                    }

                    // Worst clearance indicator
                    if self.towers.len() >= 2 {
                        if let Some(ref t) = self.terrain {
                            let worst = worst_clearance(
                                &t.profile,
                                &self.towers,
                                self.h_tension_n,
                                self.min_clearance_m,
                            );
                            let (color, label) = if worst < 0.0 {
                                (egui::Color32::RED, format!("⚠ min clear {worst:.1} m"))
                            } else if worst < self.min_clearance_m {
                                (egui::Color32::YELLOW, format!("⚠ min clear {worst:.1} m"))
                            } else {
                                (
                                    egui::Color32::from_rgb(80, 200, 120),
                                    format!("✓ min clear {worst:.1} m"),
                                )
                            };
                            ui.colored_label(color, label);
                        }
                    }
                });
            });

        // ── status bar (G6 save/load/export feedback) ──
        if !self.status.is_empty() {
            egui::Panel::bottom("status_bar")
                .exact_size(22.0)
                .show_inside(ui, |ui| {
                    ui.horizontal_centered(|ui| {
                        ui.colored_label(egui::Color32::from_rgb(120, 180, 240), &self.status);
                    });
                });
        }

        // ── right panel: editable tower table + span / load data (G9) ──
        // All read-only results are computed *before* the mutable tower borrow so
        // the table can edit `self.towers` in place; edits show up next frame.
        let analysis = analysis::analyze(&self.build_project());
        let structure_results = analysis.structures;
        let section_count = analysis.sections.len();
        // Pre-render the span table rows (these need the terrain, which we must not
        // borrow across the mutable tower edit below).
        let span_rows: Vec<(String, f64, f64, String)> = {
            let h_tension = self.h_tension_n;
            let min_clear = self.min_clearance_m;
            let terrain_ref = self.terrain.as_ref();
            self.towers
                .windows(2)
                .enumerate()
                .map(|(i, w)| {
                    let horiz = w[1].distance_m - w[0].distance_m;
                    let (sag, clr) = span_stats(terrain_ref, &w[0], &w[1], h_tension, min_clear);
                    let clr_str = clr
                        .map(|c| format!("{c:.1}"))
                        .unwrap_or_else(|| "-".to_string());
                    (format!("S{}-{}", i + 1, i + 2), horiz, sag, clr_str)
                })
                .collect()
        };

        // Disjoint field borrows for the editable table.
        let towers = &mut self.towers;
        let families = &self.families;
        let selected = &mut self.selected_tower;

        egui::Panel::right("spotting_panel")
            .default_size(300.0)
            .show_inside(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.strong("Towers");
                        ui.label(
                            egui::RichText::new(format!("· {section_count} tension section(s)"))
                                .weak(),
                        );
                    });
                    ui.label(
                        egui::RichText::new(
                            "Edit a row's function to re-partition tension sections; \
                             select ▶ to inspect the structure.",
                        )
                        .weak()
                        .size(10.0),
                    );
                    ui.separator();
                    egui::Grid::new("tower_grid")
                        .striped(true)
                        .min_col_width(40.0)
                        .show(ui, |ui| {
                            ui.label("#");
                            ui.label("Dist (km)");
                            ui.label("Function");
                            ui.label("Family");
                            ui.label("Att (m)");
                            ui.label("");
                            ui.end_row();
                            let mut to_remove: Option<usize> = None;
                            for (i, tw) in towers.iter_mut().enumerate() {
                                let is_sel = *selected == Some(i);
                                let label = egui::RichText::new(format!("T{}", i + 1));
                                let label = if is_sel { label.strong() } else { label };
                                ui.label(label);
                                ui.label(format!("{:.2}", tw.distance_m / 1000.0));

                                // Function dropdown — re-partitions sections live.
                                egui::ComboBox::from_id_salt(("fn", i))
                                    .selected_text(tw.function.label())
                                    .width(96.0)
                                    .show_ui(ui, |ui| {
                                        for f in [
                                            StructureFunction::Suspension,
                                            StructureFunction::Anchor,
                                        ] {
                                            ui.selectable_value(&mut tw.function, f, f.label());
                                        }
                                    });

                                // Family dropdown — assigns/clears the G8 family ref.
                                let fam_text = tw
                                    .family
                                    .as_ref()
                                    .and_then(|tf| families.get(tf.family))
                                    .map(|f| f.name.as_str())
                                    .unwrap_or("—");
                                egui::ComboBox::from_id_salt(("fam", i))
                                    .selected_text(fam_text)
                                    .width(120.0)
                                    .show_ui(ui, |ui| {
                                        if ui.selectable_label(tw.family.is_none(), "—").clicked()
                                        {
                                            tw.family = None;
                                        }
                                        for (fi, fam) in families.iter().enumerate() {
                                            let sel = tw
                                                .family
                                                .as_ref()
                                                .map(|tf| tf.family == fi)
                                                .unwrap_or(false);
                                            if ui.selectable_label(sel, &fam.name).clicked() {
                                                tw.family = Some(TowerFamily {
                                                    family: fi,
                                                    height_m: fam.default_height_m,
                                                    effective_height_override_m: None,
                                                    chart_override: None,
                                                });
                                            }
                                        }
                                    });

                                ui.add(
                                    egui::DragValue::new(&mut tw.attachment_height_m)
                                        .range(5.0..=80.0)
                                        .speed(0.5),
                                );

                                ui.horizontal(|ui| {
                                    if ui.small_button("▶").on_hover_text("Inspect").clicked() {
                                        *selected = Some(i);
                                    }
                                    if ui.small_button("✖").on_hover_text("Delete").clicked() {
                                        to_remove = Some(i);
                                    }
                                });
                                ui.end_row();
                            }
                            if let Some(i) = to_remove {
                                towers.remove(i);
                                match *selected {
                                    Some(s) if s == i => *selected = None,
                                    Some(s) if s > i => *selected = Some(s - 1),
                                    _ => {}
                                }
                            }
                        });

                    if !span_rows.is_empty() {
                        ui.add_space(8.0);
                        ui.strong("Spans");
                        ui.separator();
                        egui::Grid::new("span_grid")
                            .striped(true)
                            .min_col_width(48.0)
                            .show(ui, |ui| {
                                ui.label("Span");
                                ui.label("Horiz (m)");
                                ui.label("Sag (m)");
                                ui.label("Min clr (m)");
                                ui.end_row();
                                for (name, horiz, sag, clr) in &span_rows {
                                    ui.label(name);
                                    ui.label(format!("{horiz:.0}"));
                                    ui.label(format!("{sag:.1}"));
                                    ui.label(clr);
                                    ui.end_row();
                                }
                            });
                    }

                    if !structure_results.is_empty() {
                        ui.add_space(8.0);
                        ui.strong("Structure loads");
                        ui.separator();
                        egui::Grid::new("structure_grid")
                            .striped(true)
                            .min_col_width(48.0)
                            .show(ui, |ui| {
                                ui.label("Tower");
                                ui.label("Wind (m)");
                                ui.label("Weight (m)");
                                ui.label("Vert (kN)");
                                ui.label("Chart");
                                ui.end_row();
                                for st in &structure_results {
                                    ui.label(format!("T{}", st.tower + 1));
                                    ui.label(format!("{:.0}", st.wind_span_m));
                                    ui.label(format!("{:.0}", st.weight_span_m));
                                    ui.label(format!("{:.1}", st.vertical_load_n / 1000.0));
                                    match st.chart_ok {
                                        Some(true) => ui.colored_label(
                                            egui::Color32::from_rgb(80, 200, 120),
                                            "✓",
                                        ),
                                        Some(false) => {
                                            ui.colored_label(egui::Color32::RED, "✗ over")
                                        }
                                        None => ui.weak("—"),
                                    };
                                    ui.end_row();
                                }
                            });
                    }
                });
            });

        // ── dock area (3D + 2D views) ──
        egui::CentralPanel::default().show_inside(ui, |ui| {
            egui_dock::DockArea::new(&mut self.dock_state)
                .style(egui_dock::Style::from_egui(ui.style()))
                .show_inside(
                    ui,
                    &mut Viewer {
                        orbit: &mut self.orbit,
                        cam2d: &mut self.cam2d,
                        view_proj,
                        terrain: self.terrain.as_mut(),
                        towers: &mut self.towers,
                        families: &self.families,
                        selected_tower: &mut self.selected_tower,
                        spotting_mode: self.spotting_mode,
                        route_edit_mode: self.route_edit_mode,
                        selected_poi: &mut self.selected_poi,
                        attachment_height_m: self.attachment_height_m,
                        h_tension_n: self.h_tension_n,
                        min_clearance_m: self.min_clearance_m,
                    },
                );
        });

        // ── route-edit confirmation (G10d) ──
        if self.confirm_route_edit {
            egui::Window::new("Edit route?")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
                .show(ui.ctx(), |ui| {
                    ui.label(format!(
                        "Editing the route re-stations the line and would leave the \
                         {} spotted tower(s) without a reference.",
                        self.towers.len()
                    ));
                    ui.label("Clear the spotted towers and start editing?");
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        if ui.button("Clear & edit").clicked() {
                            self.towers.clear();
                            self.selected_tower = None;
                            self.route_edit_mode = true;
                            self.confirm_route_edit = false;
                        }
                        if ui.button("Cancel").clicked() {
                            self.confirm_route_edit = false;
                        }
                    });
                });
        }

        // ── drafting previews (G10d): review, then native "save as…" ──
        if let Some(md) = self.report_preview.clone() {
            let mut open = true;
            let mut close = false;
            egui::Window::new("Calculation report")
                .collapsible(false)
                .default_size([640.0, 480.0])
                .open(&mut open)
                .show(ui.ctx(), |ui| {
                    ui.horizontal(|ui| {
                        if ui.button("💾 Save as…").clicked() {
                            if let Some(path) = rfd::FileDialog::new()
                                .set_title("Save calculation report")
                                .add_filter("Markdown", &["md"])
                                .set_file_name(REPORT_PATH)
                                .save_file()
                            {
                                self.status = match std::fs::write(&path, &md) {
                                    Ok(()) => format!("Wrote report → {}", path.display()),
                                    Err(e) => format!("Report save failed: {e}"),
                                };
                            }
                        }
                        if ui.button("Close").clicked() {
                            close = true;
                        }
                    });
                    ui.separator();
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        ui.add(
                            egui::TextEdit::multiline(&mut md.as_str())
                                .font(egui::TextStyle::Monospace)
                                .desired_width(f32::INFINITY),
                        );
                    });
                });
            if !open || close {
                self.report_preview = None;
            }
        }

        if let Some(svg) = self.sheet_svg.clone() {
            let mut open = true;
            let mut close = false;
            egui::Window::new("Plan & profile sheet")
                .collapsible(false)
                .default_size([720.0, 360.0])
                .open(&mut open)
                .show(ui.ctx(), |ui| {
                    ui.horizontal(|ui| {
                        if ui.button("💾 Save as…").clicked() {
                            if let Some(path) = rfd::FileDialog::new()
                                .set_title("Save plan & profile sheet")
                                .add_filter("SVG", &["svg"])
                                .set_file_name(SHEET_PATH)
                                .save_file()
                            {
                                self.status = match std::fs::write(&path, &svg) {
                                    Ok(()) => format!("Wrote sheet → {}", path.display()),
                                    Err(e) => format!("Sheet save failed: {e}"),
                                };
                            }
                        }
                        if ui.button("Close").clicked() {
                            close = true;
                        }
                    });
                    ui.separator();
                    egui::ScrollArea::both().show(ui, |ui| match &self.sheet_texture {
                        Some(tex) => {
                            ui.image(egui::load::SizedTexture::from_handle(tex));
                        }
                        None => {
                            ui.weak("Preview unavailable — save the SVG to view it.");
                        }
                    });
                });
            if !open || close {
                self.sheet_svg = None;
                self.sheet_texture = None;
            }
        }
    }
}

// ── tab viewer ────────────────────────────────────────────────────────────────

struct Viewer<'a> {
    orbit: &'a mut OrbitCamera,
    cam2d: &'a mut Camera2D,
    view_proj: [[f32; 4]; 4],
    terrain: Option<&'a mut TerrainData>,
    towers: &'a mut Vec<Tower>,
    families: &'a [StructureFamily],
    selected_tower: &'a mut Option<usize>,
    spotting_mode: bool,
    route_edit_mode: bool,
    selected_poi: &'a mut Option<usize>,
    attachment_height_m: f64,
    h_tension_n: f64,
    min_clearance_m: f64,
}

impl TabViewer for Viewer<'_> {
    type Tab = Tab;

    fn title(&mut self, tab: &mut Tab) -> egui::WidgetText {
        match tab {
            Tab::PlanView => "Plan".into(),
            Tab::View3D => "3D view".into(),
            Tab::View2D => "Profile".into(),
            Tab::TowerElevation => "Tower elevation".into(),
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Tab) {
        match tab {
            Tab::PlanView => self.view_plan(ui),
            Tab::View3D => self.view3d(ui),
            Tab::View2D => self.view2d(ui),
            Tab::TowerElevation => self.view_tower_elevation(ui),
        }
    }
}

impl Viewer<'_> {
    // ── 3D view ───────────────────────────────────────────────────────────────

    fn view3d(&mut self, ui: &mut egui::Ui) {
        let (rect, resp) =
            ui.allocate_exact_size(ui.available_size(), egui::Sense::click_and_drag());

        if resp.dragged_by(egui::PointerButton::Primary) {
            let d = resp.drag_delta();
            self.orbit.yaw -= d.x * 0.005;
            self.orbit.pitch = (self.orbit.pitch - d.y * 0.005).clamp(-1.45, 1.45);
        }
        let scroll = ui.input(|i| i.smooth_scroll_delta.y);
        if resp.hovered() && scroll != 0.0 {
            self.orbit.distance =
                (self.orbit.distance - scroll * self.orbit.distance * 0.01).max(10.0);
        }

        // Terrain wireframe
        if let Some(t) = self.terrain.as_deref() {
            let wf = t.wireframe.clone();
            let vp = self.view_proj;
            ui.painter().add(egui_wgpu::Callback::new_paint_callback(
                rect,
                atldp_render::terrain_mesh::TerrainMeshCallback {
                    vertices: wf,
                    view_proj: vp,
                    elev_min: t.grid.elev_min,
                    elev_max: t.grid.elev_max,
                },
            ));
        }

        // Spotting geometry (towers + conductors)
        let spot_verts = self.build_spotting_vertices_3d();
        if !spot_verts.is_empty() {
            let vp = self.view_proj;
            ui.painter().add(egui_wgpu::Callback::new_paint_callback(
                rect,
                SpottingCallback {
                    vertices: spot_verts,
                    view_proj: vp,
                },
            ));
        }

        // HUD
        let terrain_label = self
            .terrain
            .as_deref()
            .map(|t| {
                format!(
                    "terrain {:.0}×{:.0} km  elev {:.0}–{:.0} m",
                    t.grid.east_m / 1000.0,
                    t.grid.north_m / 1000.0,
                    t.grid.elev_min,
                    t.grid.elev_max,
                )
            })
            .unwrap_or_else(|| "no terrain loaded".to_string());

        ui.painter().text(
            rect.left_bottom() + egui::vec2(6.0, -22.0),
            egui::Align2::LEFT_BOTTOM,
            format!(
                "pitch {:.1}°  yaw {:.1}°  dist {:.0} m",
                self.orbit.pitch.to_degrees(),
                self.orbit.yaw.to_degrees(),
                self.orbit.distance,
            ),
            egui::FontId::monospace(11.0),
            egui::Color32::from_gray(160),
        );
        ui.painter().text(
            rect.left_bottom() + egui::vec2(6.0, -6.0),
            egui::Align2::LEFT_BOTTOM,
            terrain_label,
            egui::FontId::monospace(11.0),
            egui::Color32::from_rgb(80, 200, 120),
        );
    }

    /// Build LINE_LIST vertices for towers and conductors in 3D space.
    fn build_spotting_vertices_3d(&self) -> Vec<SpottingVertex> {
        let Some(terrain) = self.terrain.as_deref() else {
            return vec![];
        };
        let mut verts = Vec::new();
        let tower_col = [1.0_f32, 0.95, 0.80, 1.0]; // warm white
        let cond_ok = [0.2_f32, 0.85, 1.0, 1.0]; // cyan
        let cond_vio = [1.0_f32, 0.22, 0.22, 1.0]; // red

        for tw in self.towers.iter() {
            let [e, n] = terrain.east_north_at(tw.distance_m);
            let ground = tw.ground_elevation_m as f32;
            let attach = tw.attachment_elevation_m() as f32;
            // Vertical post
            verts.push(SpottingVertex {
                pos: [e, ground, n],
                col: tower_col,
            });
            verts.push(SpottingVertex {
                pos: [e, attach, n],
                col: tower_col,
            });
            // Short crossarm (4 m each side along east axis)
            verts.push(SpottingVertex {
                pos: [e - 4.0, attach, n],
                col: tower_col,
            });
            verts.push(SpottingVertex {
                pos: [e + 4.0, attach, n],
                col: tower_col,
            });
        }

        // Conductors
        for win in self.towers.windows(2) {
            let t1 = &win[0];
            let t2 = &win[1];
            let clearance_ok = terrain.profile.len() >= 2
                && min_clearance_span(&terrain.profile, t1, t2, self.h_tension_n)
                    >= self.min_clearance_m;
            let col = if clearance_ok { cond_ok } else { cond_vio };

            let pts = catenary_profile_pts(t1, t2, self.h_tension_n, CATENARY_SAMPLES);
            for seg in pts.windows(2) {
                let (d0, y0) = seg[0];
                let (d1, y1) = seg[1];
                let [e0, n0] = terrain.east_north_at(d0);
                let [e1, n1] = terrain.east_north_at(d1);
                verts.push(SpottingVertex {
                    pos: [e0, y0 as f32, n0],
                    col,
                });
                verts.push(SpottingVertex {
                    pos: [e1, y1 as f32, n1],
                    col,
                });
            }
        }

        verts
    }

    // ── 2D profile view ───────────────────────────────────────────────────────

    fn view2d(&mut self, ui: &mut egui::Ui) {
        let (rect, resp) =
            ui.allocate_exact_size(ui.available_size(), egui::Sense::click_and_drag());
        let painter = ui.painter_at(rect);

        // Pan: right-drag; zoom: scroll.
        if resp.dragged_by(egui::PointerButton::Secondary) {
            let d = resp.drag_delta();
            self.cam2d.center[0] -= d.x / self.cam2d.pixels_per_metre;
            self.cam2d.center[1] += d.y / self.cam2d.scale_y();
        }
        let scroll = ui.input(|i| i.smooth_scroll_delta.y);
        if resp.hovered() && scroll != 0.0 {
            self.cam2d.pixels_per_metre =
                (self.cam2d.pixels_per_metre * (1.0 + scroll * 0.01)).clamp(0.001, 200.0);
        }

        // Tower placement on left-click (primary button, no drag).
        if self.spotting_mode && resp.clicked_by(egui::PointerButton::Primary) {
            if let Some(pointer_pos) = resp.interact_pointer_pos() {
                let vp = [rect.width(), rect.height()];
                let sx = pointer_pos.x - rect.left();
                let _sy = pointer_pos.y - rect.top();
                let world_x = (self.cam2d.center[0] as f64)
                    + ((sx - vp[0] * 0.5) / self.cam2d.pixels_per_metre) as f64;
                let ground_elev = self
                    .terrain
                    .as_deref()
                    .map(|t| t.elev_at(world_x) as f64)
                    .unwrap_or(0.0);
                if !ground_elev.is_nan() {
                    // Avoid placing a tower at nearly the same location as the last.
                    let too_close = self
                        .towers
                        .last()
                        .map(|tw| (tw.distance_m - world_x).abs() < 10.0)
                        .unwrap_or(false);
                    if !too_close {
                        self.towers.push(Tower {
                            distance_m: world_x,
                            ground_elevation_m: ground_elev,
                            attachment_height_m: self.attachment_height_m,
                            ..Default::default()
                        });
                        // Keep sorted by distance.
                        self.towers
                            .sort_by(|a, b| a.distance_m.partial_cmp(&b.distance_m).unwrap());
                    }
                }
            }
        }

        painter.rect_filled(rect, 0.0, egui::Color32::from_rgb(18, 18, 22));

        let sx = self.cam2d.pixels_per_metre;
        let sy = self.cam2d.scale_y();
        let adaptive_spacing = |px_per_m: f32| -> f32 {
            let raw = 60.0 / px_per_m;
            let exp = raw.log10().floor() as i32;
            (10.0_f32).powi(exp)
                * [1.0, 2.0, 5.0]
                    .iter()
                    .copied()
                    .find(|&v| v * px_per_m >= 40.0)
                    .unwrap_or(1.0)
        };
        let grid_m_x = adaptive_spacing(sx);
        let grid_m_y = adaptive_spacing(sy);
        let vp = [rect.width(), rect.height()];
        let grid_stroke = egui::Stroke::new(0.5, egui::Color32::from_gray(40));

        let world_left = self.cam2d.center[0] - vp[0] / (2.0 * sx);
        let world_right = self.cam2d.center[0] + vp[0] / (2.0 * sx);
        let world_top = self.cam2d.center[1] + vp[1] / (2.0 * sy);
        let world_bot = self.cam2d.center[1] - vp[1] / (2.0 * sy);

        let gx0 = (world_left / grid_m_x).floor() as i32;
        let gx1 = (world_right / grid_m_x).ceil() as i32;
        for gx in gx0..=gx1 {
            let wx = gx as f32 * grid_m_x;
            let px = self.cam2d.world_to_screen([wx, 0.0], vp)[0] + rect.left();
            painter.line_segment(
                [egui::pos2(px, rect.top()), egui::pos2(px, rect.bottom())],
                grid_stroke,
            );
        }
        let gy0 = (world_bot / grid_m_y).floor() as i32;
        let gy1 = (world_top / grid_m_y).ceil() as i32;
        for gy in gy0..=gy1 {
            let wy = gy as f32 * grid_m_y;
            let py = self.cam2d.world_to_screen([0.0, wy], vp)[1] + rect.top();
            painter.line_segment(
                [egui::pos2(rect.left(), py), egui::pos2(rect.right(), py)],
                grid_stroke,
            );
        }

        // ── terrain profile ──
        if let Some(t) = self.terrain.as_deref() {
            let profile_pts: Vec<egui::Pos2> = t
                .profile
                .iter()
                .filter(|p| !p.elevation_m.is_nan())
                .map(|p| {
                    let [sx, sy] = self
                        .cam2d
                        .world_to_screen([p.distance_m as f32, p.elevation_m], vp);
                    egui::pos2(rect.left() + sx, rect.top() + sy)
                })
                .collect();

            if profile_pts.len() >= 2 {
                // The terrain is a non-convex polyline — draw it as the ground line
                // only (a `convex_polygon` fill would fan spurious triangles across
                // the relief). No axis-anchored fill.
                let profile_stroke = egui::Stroke::new(1.5, egui::Color32::from_rgb(100, 180, 60));
                for w in profile_pts.windows(2) {
                    painter.line_segment([w[0], w[1]], profile_stroke);
                }
            }

            // Elevation annotations at quarter points.
            let total = t.profile.last().map(|p| p.distance_m).unwrap_or(1.0);
            for &frac in &[0.0, 0.25, 0.5, 0.75, 1.0_f64] {
                let target_dist = frac * total;
                if let Some(p) = t
                    .profile
                    .iter()
                    .min_by_key(|p| ((p.distance_m - target_dist).abs() * 1000.0) as i64)
                {
                    if !p.elevation_m.is_nan() {
                        let [px, py] = self
                            .cam2d
                            .world_to_screen([p.distance_m as f32, p.elevation_m], vp);
                        painter.text(
                            egui::pos2(rect.left() + px, rect.top() + py - 8.0),
                            egui::Align2::CENTER_BOTTOM,
                            format!("{:.0} m", p.elevation_m),
                            egui::FontId::monospace(10.0),
                            egui::Color32::from_gray(180),
                        );
                    }
                }
            }
        }

        // ── catenary conductors between towers ──
        let cond_ok_stroke = egui::Stroke::new(2.0, egui::Color32::from_rgb(50, 200, 255));
        let cond_vio_stroke = egui::Stroke::new(2.0, egui::Color32::RED);

        for win in self.towers.windows(2) {
            let t1 = &win[0];
            let t2 = &win[1];
            let clearance_ok = self
                .terrain
                .as_deref()
                .map(|ter| {
                    min_clearance_span(&ter.profile, t1, t2, self.h_tension_n)
                        >= self.min_clearance_m
                })
                .unwrap_or(true);
            let stroke = if clearance_ok {
                cond_ok_stroke
            } else {
                cond_vio_stroke
            };

            let pts = catenary_profile_pts(t1, t2, self.h_tension_n, CATENARY_SAMPLES);
            let screen_pts: Vec<egui::Pos2> = pts
                .iter()
                .map(|&(d, y)| {
                    let [sx, sy] = self.cam2d.world_to_screen([d as f32, y as f32], vp);
                    egui::pos2(rect.left() + sx, rect.top() + sy)
                })
                .collect();
            for w in screen_pts.windows(2) {
                painter.line_segment([w[0], w[1]], stroke);
            }

            // Clearance annotation at the sag point (mid-span).
            if let Some(&(d_mid, y_mid)) = pts.get(pts.len() / 2) {
                if let Some(ter) = self.terrain.as_deref() {
                    let gnd = ter.elev_at(d_mid) as f64;
                    if !gnd.is_nan() {
                        let clr = y_mid - gnd;
                        let label_col = if clr < self.min_clearance_m {
                            egui::Color32::RED
                        } else {
                            egui::Color32::from_gray(180)
                        };
                        let [px, py] = self.cam2d.world_to_screen([d_mid as f32, y_mid as f32], vp);
                        painter.text(
                            egui::pos2(rect.left() + px, rect.top() + py - 6.0),
                            egui::Align2::CENTER_BOTTOM,
                            format!("{clr:.1} m"),
                            egui::FontId::monospace(9.0),
                            label_col,
                        );
                    }
                }
            }
        }

        // ── tower symbols ──
        let tower_stroke = egui::Stroke::new(2.0, egui::Color32::from_rgb(255, 220, 100));
        for (i, tw) in self.towers.iter().enumerate() {
            let gnd = tw.ground_elevation_m as f32;
            let att = tw.attachment_elevation_m() as f32;
            let d = tw.distance_m as f32;

            let [px, py_gnd] = self.cam2d.world_to_screen([d, gnd], vp);
            let [_, py_att] = self.cam2d.world_to_screen([d, att], vp);
            let px = rect.left() + px;
            let py_gnd = rect.top() + py_gnd;
            let py_att = rect.top() + py_att;

            // Vertical post
            painter.line_segment(
                [egui::pos2(px, py_gnd), egui::pos2(px, py_att)],
                tower_stroke,
            );
            // Short horizontal crossarm (6 px each side)
            let arm_px = (6.0 * self.cam2d.pixels_per_metre).max(4.0);
            painter.line_segment(
                [
                    egui::pos2(px - arm_px, py_att),
                    egui::pos2(px + arm_px, py_att),
                ],
                tower_stroke,
            );
            // Label
            painter.text(
                egui::pos2(px, py_att - 4.0),
                egui::Align2::CENTER_BOTTOM,
                format!("T{}", i + 1),
                egui::FontId::monospace(9.0),
                egui::Color32::from_rgb(255, 220, 100),
            );
        }

        // Spotting cursor hint
        if self.spotting_mode {
            painter.text(
                rect.right_top() + egui::vec2(-6.0, 18.0),
                egui::Align2::RIGHT_TOP,
                "left-click to place tower",
                egui::FontId::monospace(10.0),
                egui::Color32::from_rgb(255, 220, 100),
            );
        }

        // Status bar
        painter.text(
            rect.left_bottom() + egui::vec2(6.0, -18.0),
            egui::Align2::LEFT_BOTTOM,
            format!(
                "grid X={grid_m_x:.0} m  Y={grid_m_y:.0} m  |  {:.3} px/m  |  right-drag pan  scroll zoom",
                self.cam2d.pixels_per_metre
            ),
            egui::FontId::monospace(10.0),
            egui::Color32::from_gray(100),
        );
    }

    // ── plan view & route editor (G10c, ADR-0022) ───────────────────────────────

    /// Top-down map of the cropped DEM with the **editable route** drawn on it. In
    /// route-edit mode, clicking empty map inserts an angle POI on the nearest leg,
    /// dragging a marker moves it, and the selected POI's kind is editable; every
    /// edit re-derives the ground profile (`TerrainData::recompute`) so the profile
    /// and tower table update live (ADR-0019/0022).
    fn view_plan(&mut self, ui: &mut egui::Ui) {
        let edit = self.route_edit_mode;
        let mut sel = *self.selected_poi;
        let Some(t) = self.terrain.as_deref_mut() else {
            ui.centered_and_justified(|ui| {
                ui.weak("No terrain loaded — set ATLDP_TERRAIN or load a project.");
            });
            return;
        };
        let mut dirty = false;

        // ── header / selected-POI controls ──
        ui.horizontal_wrapped(|ui| {
            ui.strong("Plan view");
            ui.separator();
            if edit {
                ui.colored_label(
                    egui::Color32::from_rgb(120, 200, 255),
                    "route edit: click empty map to add a point · drag a point to move it",
                );
            } else {
                ui.weak("enable “Route edit” in the toolbar to draw the line");
            }
            if let Some(i) = sel {
                if i < t.route.len() {
                    ui.separator();
                    ui.label(format!("P{}", i + 1));
                    let mut kind = t.route[i].kind;
                    egui::ComboBox::from_id_salt("poi_kind")
                        .selected_text(kind.label())
                        .show_ui(ui, |ui| {
                            for k in [
                                PoiKind::Terminal,
                                PoiKind::Angle,
                                PoiKind::Crossing,
                                PoiKind::Obstacle,
                                PoiKind::Constraint,
                            ] {
                                ui.selectable_value(&mut kind, k, k.label());
                            }
                        });
                    if kind != t.route[i].kind {
                        t.route[i].kind = kind;
                        dirty = true;
                    }
                    // Endpoints stay terminals; only interior points may be deleted.
                    let interior = i != 0 && i + 1 < t.route.len();
                    if interior && ui.button("✖ delete point").clicked() {
                        t.route.remove(i);
                        sel = None;
                        dirty = true;
                    }
                }
            }
        });

        let avail = ui.available_size();
        let (rect, resp) = ui.allocate_exact_size(avail, egui::Sense::click_and_drag());
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 0.0, egui::Color32::from_rgb(10, 12, 16));

        // Fit the working DEM's local-plane extents into the rect, north up, equal
        // aspect — the same plane the route POIs are projected through.
        let half_e = (t.grid.east_m as f64 * 0.5).max(1.0);
        let half_n = (t.grid.north_m as f64 * 0.5).max(1.0);
        let margin = 14.0_f64;
        let scale = ((rect.width() as f64 - 2.0 * margin) / (2.0 * half_e))
            .min((rect.height() as f64 - 2.0 * margin) / (2.0 * half_n))
            .max(1e-6);
        let (cx, cy) = (rect.center().x as f64, rect.center().y as f64);
        let to_screen =
            |e: f64, north: f64| egui::pos2((cx + e * scale) as f32, (cy - north * scale) as f32);
        let from_screen =
            |p: egui::Pos2| -> [f64; 2] { [(p.x as f64 - cx) / scale, (cy - p.y as f64) / scale] };

        // ── DEM raster as a hypsometric mesh (one draw call) ──
        {
            let grid = &t.grid;
            let (lo, hi) = (grid.elev_min, grid.elev_max.max(grid.elev_min + 1.0));
            let mut mesh = egui::Mesh::default();
            for p in &grid.positions {
                mesh.colored_vertex(
                    to_screen(p[0] as f64, p[2] as f64),
                    hypso_color(p[1], lo, hi),
                );
            }
            for r in 0..grid.rows.saturating_sub(1) {
                for c in 0..grid.cols.saturating_sub(1) {
                    let i = (r * grid.cols + c) as u32;
                    let right = i + 1;
                    let down = i + grid.cols as u32;
                    mesh.add_triangle(i, right, down);
                    mesh.add_triangle(right, down + 1, down);
                }
            }
            painter.add(egui::Shape::mesh(mesh));
        }

        // ── route polyline ──
        let scr: Vec<egui::Pos2> = t
            .route
            .iter()
            .map(|p| {
                let [e, north] = t.plane.to_local(p.lat, p.lon);
                to_screen(e, north)
            })
            .collect();
        for w in scr.windows(2) {
            painter.line_segment(
                [w[0], w[1]],
                egui::Stroke::new(2.0, egui::Color32::from_rgb(255, 210, 90)),
            );
        }

        // ── edit interactions ──
        if edit {
            for (i, &sp) in scr.iter().enumerate() {
                let id = ui.id().with(("poi_marker", i));
                let mrect = egui::Rect::from_center_size(sp, egui::vec2(16.0, 16.0));
                let r = ui.interact(mrect, id, egui::Sense::click_and_drag());
                if r.dragged() {
                    let [e, north] = from_screen(sp + r.drag_delta());
                    let [lat, lon] = t.plane.to_geo(e, north);
                    t.route[i].lat = lat;
                    t.route[i].lon = lon;
                    dirty = true;
                }
                if r.clicked() {
                    sel = Some(i);
                }
            }
            // Click on empty map (not on a marker) inserts an angle POI on the
            // nearest leg.
            if resp.clicked_by(egui::PointerButton::Primary) {
                if let Some(pp) = resp.interact_pointer_pos() {
                    let on_marker = scr.iter().any(|&s| s.distance(pp) < 10.0);
                    if !on_marker && t.route.len() >= 2 {
                        let idx = nearest_leg_index(&scr, pp);
                        let [e, north] = from_screen(pp);
                        let [lat, lon] = t.plane.to_geo(e, north);
                        let mut poi = Poi::terminal(lat, lon, 0.0);
                        poi.kind = PoiKind::Angle;
                        t.route.insert(idx, poi);
                        sel = Some(idx);
                        dirty = true;
                    }
                }
            }
        }

        // ── POI markers + labels ──
        for (i, &sp) in scr.iter().enumerate() {
            let poi = &t.route[i];
            let col = poi_color(poi.kind);
            let is_sel = sel == Some(i);
            painter.circle_filled(sp, if is_sel { 6.0 } else { 4.0 }, col);
            if is_sel {
                painter.circle_stroke(sp, 9.0, egui::Stroke::new(1.5, egui::Color32::WHITE));
            }
            let label = if poi.deviation_angle_deg.abs() > 0.05 {
                format!(
                    "P{} {} · {:.0}°",
                    i + 1,
                    poi.kind.label(),
                    poi.deviation_angle_deg
                )
            } else {
                format!("P{} {}", i + 1, poi.kind.label())
            };
            painter.text(
                sp + egui::vec2(9.0, -9.0),
                egui::Align2::LEFT_BOTTOM,
                label,
                egui::FontId::monospace(10.0),
                col,
            );
        }

        painter.text(
            rect.left_bottom() + egui::vec2(6.0, -6.0),
            egui::Align2::LEFT_BOTTOM,
            format!(
                "{:.0}×{:.0} km · {} points · {:.1} km route · north ↑",
                t.grid.east_m / 1000.0,
                t.grid.north_m / 1000.0,
                t.route.len(),
                t.profile_total_dist / 1000.0,
            ),
            egui::FontId::monospace(10.0),
            egui::Color32::from_gray(150),
        );

        if dirty {
            t.recompute();
        }
        *self.selected_poi = sel;
    }

    // ── tower-elevation view (G10, ADR-0020) ────────────────────────────────────

    /// Draw the selected tower's structure as a real shape: the family silhouette
    /// and every wire attachment point in the structure's elevation frame, scaled
    /// to fit. "Choosing a structure" becomes inspecting a drawing, not a number.
    fn view_tower_elevation(&mut self, ui: &mut egui::Ui) {
        let Some(sel) = *self.selected_tower else {
            ui.centered_and_justified(|ui| {
                ui.weak("Select a tower (▶ in the table) to inspect its structure.");
            });
            return;
        };
        let Some(tower) = self.towers.get(sel) else {
            *self.selected_tower = None;
            return;
        };

        // Resolve the family geometry; fall back to the built-in single-circuit
        // shape so the view is meaningful even for an unassigned tower.
        let height_m = tower.attachment_height_m;
        let default_geom = atldp_model::AttachmentGeometry::single_circuit();
        let (fam_name, geom) = match tower
            .family
            .as_ref()
            .and_then(|tf| self.families.get(tf.family))
        {
            Some(fam) => (fam.name.as_str(), &fam.geometry),
            None => ("(no family — default geometry)", &default_geom),
        };

        ui.horizontal(|ui| {
            ui.strong(format!("T{}", sel + 1));
            ui.label(fam_name);
            ui.weak(format!(
                "· {} · h={:.1} m",
                tower.function.label(),
                height_m
            ));
        });
        ui.separator();

        let (rect, _resp) = ui.allocate_exact_size(ui.available_size(), egui::Sense::hover());
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 0.0, egui::Color32::from_rgb(16, 18, 22));

        // Structure-frame extent: lateral and vertical span of attachments +
        // silhouette, plus the mast down to ground (−height) and a margin.
        let mut min_x = 0.0_f64;
        let mut max_x = 0.0_f64;
        let mut min_y = -height_m;
        let mut max_y = 0.0_f64;
        let mut acc = |x: f64, y: f64| {
            min_x = min_x.min(x);
            max_x = max_x.max(x);
            min_y = min_y.min(y);
            max_y = max_y.max(y);
        };
        for a in &geom.attachments {
            acc(a.lateral_offset_m, a.vertical_offset_m);
        }
        for p in &geom.silhouette {
            acc(p[0], p[1]);
        }
        let span_x = (max_x - min_x).max(1.0);
        let span_y = (max_y - min_y).max(1.0);
        let margin = 40.0;
        let sx = (rect.width() - 2.0 * margin) as f64 / span_x;
        let sy = (rect.height() - 2.0 * margin) as f64 / span_y;
        let scale = sx.min(sy);
        let cx = rect.center().x as f64 - 0.5 * (min_x + max_x) * scale;
        // y grows downward on screen.
        let cy = rect.center().y as f64 + 0.5 * (min_y + max_y) * scale;
        let to_screen = |x: f64, y: f64| -> egui::Pos2 {
            egui::pos2((cx + x * scale) as f32, (cy - y * scale) as f32)
        };

        // Ground line at y = −height.
        let gy = to_screen(min_x, -height_m).y;
        painter.line_segment(
            [egui::pos2(rect.left(), gy), egui::pos2(rect.right(), gy)],
            egui::Stroke::new(1.0, egui::Color32::from_rgb(90, 120, 70)),
        );
        painter.text(
            egui::pos2(rect.left() + 4.0, gy - 2.0),
            egui::Align2::LEFT_BOTTOM,
            "ground",
            egui::FontId::monospace(10.0),
            egui::Color32::from_gray(120),
        );

        // Mast from the lowest silhouette point down to ground.
        let body_stroke = egui::Stroke::new(2.0, egui::Color32::from_rgb(255, 220, 100));
        painter.line_segment(
            [to_screen(0.0, 0.0), to_screen(0.0, -height_m)],
            body_stroke,
        );
        // Silhouette polyline.
        let sil: Vec<egui::Pos2> = geom
            .silhouette
            .iter()
            .map(|p| to_screen(p[0], p[1]))
            .collect();
        for w in sil.windows(2) {
            painter.line_segment([w[0], w[1]], body_stroke);
        }

        // Attachment points, labelled by wire.
        for a in &geom.attachments {
            let p = to_screen(a.lateral_offset_m, a.vertical_offset_m);
            let col = match a.role {
                atldp_model::WireRole::Phase => egui::Color32::from_rgb(50, 200, 255),
                atldp_model::WireRole::Shield => egui::Color32::from_rgb(200, 160, 255),
            };
            painter.circle_filled(p, 4.0, col);
            painter.text(
                p + egui::vec2(7.0, 0.0),
                egui::Align2::LEFT_CENTER,
                format!(
                    "{}  ({:+.1}, {:+.1}) m  → {:.1} m",
                    a.label,
                    a.lateral_offset_m,
                    a.vertical_offset_m,
                    tower.attachment_elevation_m() + a.vertical_offset_m,
                ),
                egui::FontId::monospace(10.0),
                col,
            );
        }

        painter.text(
            rect.left_bottom() + egui::vec2(6.0, -6.0),
            egui::Align2::LEFT_BOTTOM,
            "structure-frame elevation · attachment → conductor elevation (MSL)",
            egui::FontId::monospace(10.0),
            egui::Color32::from_gray(100),
        );
    }
}

// ── catenary helpers ──────────────────────────────────────────────────────────

/// Sample a catenary between two towers: returns `(distance_m, abs_elevation_m)`.
fn catenary_profile_pts(t1: &Tower, t2: &Tower, h_tension_n: f64, n: usize) -> Vec<(f64, f64)> {
    use atldp_core::catenary::solve_catenary;
    let horiz = t2.distance_m - t1.distance_m;
    if horiz <= 0.0 {
        return vec![];
    }
    let elev_diff = t2.attachment_elevation_m() - t1.attachment_elevation_m();
    let Ok(sol) = solve_catenary(horiz, elev_diff, CONDUCTOR_W_N_PER_M, h_tension_n) else {
        return vec![];
    };
    let c = sol.catenary_constant();
    let a = sol.low_point_x;
    // y_rel(x) = c*cosh((x-a)/c) - c*cosh(a/c)  (=0 at x=0, =elev_diff at x=horiz)
    let b = -c * (a / c).cosh();
    let e1 = t1.attachment_elevation_m();

    (0..=n)
        .map(|i| {
            let x = i as f64 / n as f64 * horiz;
            let y_rel = c * ((x - a) / c).cosh() + b;
            (t1.distance_m + x, e1 + y_rel)
        })
        .collect()
}

/// Minimum ground clearance for a single span (metres, negative = violation).
fn min_clearance_span(profile: &[ProfilePoint], t1: &Tower, t2: &Tower, h_tension_n: f64) -> f64 {
    let pts = catenary_profile_pts(t1, t2, h_tension_n, CATENARY_SAMPLES);
    let terrain_at = |d: f64| -> f64 {
        if profile.is_empty() {
            return 0.0;
        }
        let idx = profile.partition_point(|p| p.distance_m < d);
        if idx == 0 {
            return profile[0].elevation_m as f64;
        }
        if idx >= profile.len() {
            return profile.last().unwrap().elevation_m as f64;
        }
        let p1 = &profile[idx - 1];
        let p2 = &profile[idx];
        let t = ((d - p1.distance_m) / (p2.distance_m - p1.distance_m)) as f32;
        (p1.elevation_m + t * (p2.elevation_m - p1.elevation_m)) as f64
    };
    pts.iter()
        .map(|&(d, y)| {
            let gnd = terrain_at(d);
            y - gnd
        })
        .fold(f64::INFINITY, f64::min)
}

/// Worst (minimum) clearance across all spans, for the toolbar indicator.
fn worst_clearance(
    profile: &[ProfilePoint],
    towers: &[Tower],
    h_tension_n: f64,
    _min_clearance_m: f64,
) -> f64 {
    towers
        .windows(2)
        .map(|w| min_clearance_span(profile, &w[0], &w[1], h_tension_n))
        .fold(f64::INFINITY, f64::min)
}

/// Compute sag and minimum clearance for the side-panel span table.
fn span_stats(
    terrain: Option<&TerrainData>,
    t1: &Tower,
    t2: &Tower,
    h_tension_n: f64,
    _min_clearance_m: f64,
) -> (f64, Option<f64>) {
    use atldp_core::catenary::solve_catenary;
    let horiz = t2.distance_m - t1.distance_m;
    if horiz <= 0.0 {
        return (0.0, None);
    }
    let elev_diff = t2.attachment_elevation_m() - t1.attachment_elevation_m();
    let sag = solve_catenary(horiz, elev_diff, CONDUCTOR_W_N_PER_M, h_tension_n)
        .map(|sol| sol.sag)
        .unwrap_or(0.0);
    let clr = terrain.map(|t| min_clearance_span(&t.profile, t1, t2, h_tension_n));
    (sag, clr)
}

// ── drafting helpers (G10d) ───────────────────────────────────────────────────

/// Rasterize an SVG string to an `egui::ColorImage` for in-app preview (resvg).
/// `None` if it cannot be parsed/rendered — the preview then offers save-only.
fn rasterize_svg(svg: &str) -> Option<egui::ColorImage> {
    use resvg::{tiny_skia, usvg};
    let tree = usvg::Tree::from_str(svg, &usvg::Options::default()).ok()?;
    let size = tree.size();
    let (w, h) = (size.width().ceil() as u32, size.height().ceil() as u32);
    if w == 0 || h == 0 {
        return None;
    }
    let mut pixmap = tiny_skia::Pixmap::new(w, h)?;
    resvg::render(
        &tree,
        tiny_skia::Transform::identity(),
        &mut pixmap.as_mut(),
    );
    // The sheet draws an opaque white background, so premultiplied == straight.
    Some(egui::ColorImage::from_rgba_unmultiplied(
        [w as usize, h as usize],
        pixmap.data(),
    ))
}

/// The file name of `path`, or `default` if it has none — the seed for a save dialog.
fn file_name_or(path: &std::path::Path, default: &str) -> String {
    path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(default)
        .to_string()
}

// ── HGT filename parser ───────────────────────────────────────────────────────

fn parse_hgt_name(path: &std::path::Path) -> Option<(i32, i32)> {
    let stem = path.file_stem()?.to_str()?;
    if stem.len() < 7 {
        return None;
    }
    let bytes = stem.as_bytes();
    let lat_sign: i32 = match bytes[0] {
        b'N' | b'n' => 1,
        b'S' | b's' => -1,
        _ => return None,
    };
    let lat: i32 = std::str::from_utf8(&bytes[1..3]).ok()?.parse().ok()?;
    let lon_sign: i32 = match bytes[3] {
        b'E' | b'e' => 1,
        b'W' | b'w' => -1,
        _ => return None,
    };
    let lon: i32 = std::str::from_utf8(&bytes[4..7]).ok()?.parse().ok()?;
    Some((lat_sign * lat, lon_sign * lon))
}

// ── entry point ───────────────────────────────────────────────────────────────

fn main() {
    env_logger::init();
    let event_loop = EventLoop::new().expect("event loop");
    event_loop.run_app(&mut App::default()).expect("run");
}
