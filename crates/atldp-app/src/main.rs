//! ATLDP desktop CAD application — G5 manual spotting (ADR-0011, ADR-0012).
//!
//! Extends G3 terrain/route with:
//!   - Click-to-place tower spotting on the 2D terrain profile
//!   - Live catenary computation between consecutive towers
//!   - Ground clearance check with colour-coded violation highlights
//!   - Tower and conductor geometry in the 3D viewport (SpottingLines renderer)
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
    dem::{wireframe_line_list, Dem, LocalGrid},
    profile::{extract_profile, ProfilePoint},
};
use atldp_model::Tower;
use atldp_render::{
    camera::{Camera2D, OrbitCamera},
    catenary_line::CatenaryLineResources,
    spotting_lines::{SpottingCallback, SpottingResources, SpottingVertex},
    terrain_mesh::{TerrainMeshResources, TerrainVertex},
};

// ── conductor weight ──────────────────────────────────────────────────────────
const CONDUCTOR_W_N_PER_M: f64 = 15.97; // ACSR Drake
const CATENARY_SAMPLES: usize = 80;

// ── terrain state (optional) ──────────────────────────────────────────────────

struct TerrainData {
    grid: LocalGrid,
    wireframe: Vec<TerrainVertex>,
    profile: Vec<ProfilePoint>,
    /// Total profile length, metres.
    profile_total_dist: f64,
    /// LocalGrid (east, north) of the profile start, metres.
    profile_3d_start: [f32; 2],
    /// LocalGrid (east, north) of the profile end, metres.
    profile_3d_end: [f32; 2],
}

impl TerrainData {
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

        let grid = dem.to_local_grid(96, 96);
        let raw_verts = wireframe_line_list(&grid);
        let wireframe: Vec<TerrainVertex> = raw_verts
            .iter()
            .map(|&pos| TerrainVertex { pos })
            .collect();

        let b = dem.bounds;
        let lat0 = b.lat_min + 0.1;
        let lon0 = b.lon_min + 0.1;
        let lat1 = b.lat_max - 0.1;
        let lon1 = b.lon_max - 0.1;
        let profile = extract_profile(&dem, lat0, lon0, lat1, lon1, 200);
        let profile_total_dist = profile.last().map(|p| p.distance_m).unwrap_or(1.0);

        // Map profile endpoints into the LocalGrid coordinate system.
        let plane = LocalPlane::new(
            (b.lat_min + b.lat_max) * 0.5,
            (b.lon_min + b.lon_max) * 0.5,
        );
        let [e0, n0] = plane.to_local(lat0, lon0);
        let [e1, n1] = plane.to_local(lat1, lon1);

        eprintln!(
            "terrain: {} wireframe verts, {} profile samples, {:.0} km profile",
            wireframe.len(),
            profile.len(),
            profile_total_dist / 1000.0,
        );

        Some(Self {
            grid,
            wireframe,
            profile,
            profile_total_dist,
            profile_3d_start: [e0 as f32, n0 as f32],
            profile_3d_end: [e1 as f32, n1 as f32],
        })
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
        // Binary search for bracketing points.
        let idx = self
            .profile
            .partition_point(|p| p.distance_m < dist_m);
        let p1 = &self.profile[idx - 1];
        let p2 = &self.profile[idx];
        let t = ((dist_m - p1.distance_m) / (p2.distance_m - p1.distance_m)) as f32;
        if p1.elevation_m.is_nan() || p2.elevation_m.is_nan() {
            return f32::NAN;
        }
        p1.elevation_m + t * (p2.elevation_m - p1.elevation_m)
    }

    /// (east, north) in LocalGrid space for a given profile distance.
    fn east_north_at(&self, dist_m: f64) -> [f32; 2] {
        let t = if self.profile_total_dist > 0.0 {
            (dist_m / self.profile_total_dist) as f32
        } else {
            0.0
        };
        let e = self.profile_3d_start[0] + t * (self.profile_3d_end[0] - self.profile_3d_start[0]);
        let n = self.profile_3d_start[1] + t * (self.profile_3d_end[1] - self.profile_3d_start[1]);
        [e, n]
    }
}

// ── tab identifiers ───────────────────────────────────────────────────────────

#[derive(Clone, PartialEq)]
enum Tab {
    View3D,
    View2D,
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
            .with_title("ATLDP — manual spotting (G5)")
            .with_inner_size(winit::dpi::LogicalSize::new(1400u32, 860u32));
        let window = Arc::new(event_loop.create_window(attrs).expect("window"));
        self.state = Some(AppState::new(window));
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _id: WindowId,
        event: WindowEvent,
    ) {
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

        let mut egui_renderer = egui_wgpu::Renderer::new(
            &device,
            fmt,
            egui_wgpu::RendererOptions::default(),
        );
        egui_renderer
            .callback_resources
            .insert(CatenaryLineResources::new(&device, fmt));
        egui_renderer
            .callback_resources
            .insert(TerrainMeshResources::new(&device, fmt));
        egui_renderer
            .callback_resources
            .insert(SpottingResources::new(&device, fmt));

        let mut dock_state = egui_dock::DockState::new(vec![Tab::View3D]);
        let [_, right] = dock_state
            .main_surface_mut()
            .split_right(egui_dock::NodeIndex::root(), 0.5, vec![Tab::View2D]);
        let _ = right;

        let terrain = Self::try_load_terrain();
        let orbit = Self::init_camera(&terrain);

        let cam2d = if let Some(ref t) = terrain {
            let mid_dist = t.profile.last().map(|p| p.distance_m as f32 * 0.5).unwrap_or(150.0);
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
            self.egui_renderer.render(&mut rpass, &primitives, &screen_desc);
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
                        ui.colored_label(
                            egui::Color32::from_gray(120),
                            "No terrain",
                        );
                    }

                    ui.separator();

                    // Spotting toggle
                    let spot_label = if self.spotting_mode { "⬛ Stop spotting" } else { "🗼 Spot towers" };
                    if ui.button(spot_label).clicked() {
                        self.spotting_mode = !self.spotting_mode;
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
                                (egui::Color32::from_rgb(80, 200, 120), format!("✓ min clear {worst:.1} m"))
                            };
                            ui.colored_label(color, label);
                        }
                    }
                });
            });

        // ── right panel: tower / span data ──
        let towers_snap = self.towers.clone();
        let h_tension = self.h_tension_n;
        let min_clear = self.min_clearance_m;
        let terrain_ref = self.terrain.as_ref();

        egui::Panel::right("spotting_panel")
            .default_size(260.0)
            .show_inside(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.strong("Towers");
                    ui.separator();
                    egui::Grid::new("tower_grid")
                        .striped(true)
                        .min_col_width(48.0)
                        .show(ui, |ui| {
                            ui.label("#");
                            ui.label("Dist (km)");
                            ui.label("Gnd (m)");
                            ui.label("Att (m)");
                            ui.end_row();
                            for (i, tw) in towers_snap.iter().enumerate() {
                                ui.label(format!("T{}", i + 1));
                                ui.label(format!("{:.2}", tw.distance_m / 1000.0));
                                ui.label(format!("{:.0}", tw.ground_elevation_m));
                                ui.label(format!("{:.0}", tw.attachment_elevation_m()));
                                ui.end_row();
                            }
                        });

                    if towers_snap.len() >= 2 {
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
                                for i in 0..towers_snap.len() - 1 {
                                    let t1 = &towers_snap[i];
                                    let t2 = &towers_snap[i + 1];
                                    let horiz = t2.distance_m - t1.distance_m;
                                    let (sag, clr) = span_stats(
                                        terrain_ref,
                                        t1,
                                        t2,
                                        h_tension,
                                        min_clear,
                                    );
                                    let clr_str = clr
                                        .map(|c| format!("{c:.1}"))
                                        .unwrap_or_else(|| "-".to_string());
                                    ui.label(format!("S{}-{}", i + 1, i + 2));
                                    ui.label(format!("{horiz:.0}"));
                                    ui.label(format!("{sag:.1}"));
                                    ui.label(clr_str);
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
                        terrain: self.terrain.as_ref(),
                        towers: &mut self.towers,
                        spotting_mode: self.spotting_mode,
                        attachment_height_m: self.attachment_height_m,
                        h_tension_n: self.h_tension_n,
                        min_clearance_m: self.min_clearance_m,
                    },
                );
        });
    }
}

// ── tab viewer ────────────────────────────────────────────────────────────────

struct Viewer<'a> {
    orbit: &'a mut OrbitCamera,
    cam2d: &'a mut Camera2D,
    view_proj: [[f32; 4]; 4],
    terrain: Option<&'a TerrainData>,
    towers: &'a mut Vec<Tower>,
    spotting_mode: bool,
    attachment_height_m: f64,
    h_tension_n: f64,
    min_clearance_m: f64,
}

impl TabViewer for Viewer<'_> {
    type Tab = Tab;

    fn title(&mut self, tab: &mut Tab) -> egui::WidgetText {
        match tab {
            Tab::View3D => "3D view".into(),
            Tab::View2D => "Profile".into(),
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Tab) {
        match tab {
            Tab::View3D => self.view3d(ui),
            Tab::View2D => self.view2d(ui),
        }
    }
}

impl Viewer<'_> {
    // ── 3D view ───────────────────────────────────────────────────────────────

    fn view3d(&mut self, ui: &mut egui::Ui) {
        let (rect, resp) = ui.allocate_exact_size(
            ui.available_size(),
            egui::Sense::click_and_drag(),
        );

        if resp.dragged_by(egui::PointerButton::Primary) {
            let d = resp.drag_delta();
            self.orbit.yaw -= d.x * 0.005;
            self.orbit.pitch = (self.orbit.pitch - d.y * 0.005).clamp(-1.45, 1.45);
        }
        let scroll = ui.input(|i| i.smooth_scroll_delta.y);
        if resp.hovered() && scroll != 0.0 {
            self.orbit.distance = (self.orbit.distance - scroll * self.orbit.distance * 0.01)
                .max(10.0);
        }

        // Terrain wireframe
        if let Some(t) = self.terrain {
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
                SpottingCallback { vertices: spot_verts, view_proj: vp },
            ));
        }

        // HUD
        let terrain_label = self.terrain
            .map(|t| format!(
                "terrain {:.0}×{:.0} km  elev {:.0}–{:.0} m",
                t.grid.east_m / 1000.0, t.grid.north_m / 1000.0,
                t.grid.elev_min, t.grid.elev_max,
            ))
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
        let Some(terrain) = self.terrain else {
            return vec![];
        };
        let mut verts = Vec::new();
        let tower_col = [1.0_f32, 0.95, 0.80, 1.0]; // warm white
        let cond_ok = [0.2_f32, 0.85, 1.0, 1.0];    // cyan
        let cond_vio = [1.0_f32, 0.22, 0.22, 1.0];  // red

        for tw in self.towers.iter() {
            let [e, n] = terrain.east_north_at(tw.distance_m);
            let ground = tw.ground_elevation_m as f32;
            let attach = tw.attachment_elevation_m() as f32;
            // Vertical post
            verts.push(SpottingVertex { pos: [e, ground, n], col: tower_col });
            verts.push(SpottingVertex { pos: [e, attach, n], col: tower_col });
            // Short crossarm (4 m each side along east axis)
            verts.push(SpottingVertex { pos: [e - 4.0, attach, n], col: tower_col });
            verts.push(SpottingVertex { pos: [e + 4.0, attach, n], col: tower_col });
        }

        // Conductors
        for win in self.towers.windows(2) {
            let t1 = &win[0];
            let t2 = &win[1];
            let clearance_ok = terrain
                .profile
                .len() >= 2
                && min_clearance_span(&terrain.profile, t1, t2, self.h_tension_n)
                    >= self.min_clearance_m;
            let col = if clearance_ok { cond_ok } else { cond_vio };

            let pts = catenary_profile_pts(t1, t2, self.h_tension_n, CATENARY_SAMPLES);
            for seg in pts.windows(2) {
                let (d0, y0) = seg[0];
                let (d1, y1) = seg[1];
                let [e0, n0] = terrain.east_north_at(d0);
                let [e1, n1] = terrain.east_north_at(d1);
                verts.push(SpottingVertex { pos: [e0, y0 as f32, n0], col });
                verts.push(SpottingVertex { pos: [e1, y1 as f32, n1], col });
            }
        }

        verts
    }

    // ── 2D profile view ───────────────────────────────────────────────────────

    fn view2d(&mut self, ui: &mut egui::Ui) {
        let (rect, resp) = ui.allocate_exact_size(
            ui.available_size(),
            egui::Sense::click_and_drag(),
        );
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
        if self.spotting_mode {
            if resp.clicked_by(egui::PointerButton::Primary) {
                if let Some(pointer_pos) = resp.interact_pointer_pos() {
                    let vp = [rect.width(), rect.height()];
                    let sx = pointer_pos.x - rect.left();
                    let _sy = pointer_pos.y - rect.top();
                    let world_x = (self.cam2d.center[0] as f64)
                        + ((sx - vp[0] * 0.5) / self.cam2d.pixels_per_metre) as f64;
                    let ground_elev = self.terrain
                        .map(|t| t.elev_at(world_x) as f64)
                        .unwrap_or(0.0);
                    if !ground_elev.is_nan() {
                        // Avoid placing a tower at nearly the same location as the last.
                        let too_close = self.towers.last().map(|tw| {
                            (tw.distance_m - world_x).abs() < 10.0
                        }).unwrap_or(false);
                        if !too_close {
                            self.towers.push(Tower {
                                distance_m: world_x,
                                ground_elevation_m: ground_elev,
                                attachment_height_m: self.attachment_height_m,
                            });
                            // Keep sorted by distance.
                            self.towers.sort_by(|a, b| a.distance_m.partial_cmp(&b.distance_m).unwrap());
                        }
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
        if let Some(t) = self.terrain {
            let profile_pts: Vec<egui::Pos2> = t.profile
                .iter()
                .filter(|p| !p.elevation_m.is_nan())
                .map(|p| {
                    let [sx, sy] = self.cam2d.world_to_screen(
                        [p.distance_m as f32, p.elevation_m as f32],
                        vp,
                    );
                    egui::pos2(rect.left() + sx, rect.top() + sy)
                })
                .collect();

            if profile_pts.len() >= 2 {
                let ground_y = self.cam2d.world_to_screen([0.0, 0.0], vp)[1] + rect.top();
                let ground_y = ground_y.clamp(rect.top(), rect.bottom());
                let mut poly = profile_pts.clone();
                poly.push(egui::pos2(profile_pts.last().unwrap().x, ground_y));
                poly.push(egui::pos2(profile_pts.first().unwrap().x, ground_y));
                painter.add(egui::Shape::convex_polygon(
                    poly,
                    egui::Color32::from_rgba_unmultiplied(50, 90, 40, 80),
                    egui::Stroke::NONE,
                ));
                let profile_stroke = egui::Stroke::new(1.5, egui::Color32::from_rgb(100, 180, 60));
                for w in profile_pts.windows(2) {
                    painter.line_segment([w[0], w[1]], profile_stroke);
                }
            }

            // Elevation annotations at quarter points.
            let total = t.profile.last().map(|p| p.distance_m).unwrap_or(1.0);
            for &frac in &[0.0, 0.25, 0.5, 0.75, 1.0_f64] {
                let target_dist = frac * total;
                if let Some(p) = t.profile.iter().min_by_key(|p| {
                    ((p.distance_m - target_dist).abs() * 1000.0) as i64
                }) {
                    if !p.elevation_m.is_nan() {
                        let [px, py] = self.cam2d.world_to_screen(
                            [p.distance_m as f32, p.elevation_m as f32],
                            vp,
                        );
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
            let clearance_ok = self.terrain.map(|ter| {
                min_clearance_span(&ter.profile, t1, t2, self.h_tension_n)
                    >= self.min_clearance_m
            }).unwrap_or(true);
            let stroke = if clearance_ok { cond_ok_stroke } else { cond_vio_stroke };

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
                if let Some(ter) = self.terrain {
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
            painter.line_segment([egui::pos2(px, py_gnd), egui::pos2(px, py_att)], tower_stroke);
            // Short horizontal crossarm (6 px each side)
            let arm_px = (6.0 * self.cam2d.pixels_per_metre).max(4.0);
            painter.line_segment(
                [egui::pos2(px - arm_px, py_att), egui::pos2(px + arm_px, py_att)],
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
