//! ATLDP desktop CAD application — G3 terrain & route (ADR-0011, ADR-0012).
//!
//! Extends the G2 render foundation with:
//!   - `atldp-geo` DEM ingest (HGT/SRTM, pure-Rust, ADR-0013)
//!   - Terrain wireframe in the 3D orbit viewport
//!   - Terrain profile overlaid in the 2D plan/profile viewport
//!   - Camera auto-fit to terrain extents on load
//!
//! Terrain file: set `ATLDP_TERRAIN=/path/to/tile.hgt` or place
//! `S23W043.hgt` in `crates/atldp-geo/tests/data/` (gitignored, see
//! `crates/atldp-geo/tests/fetch_srtm.sh`).  The app runs without terrain;
//! only the catenary is shown in that case.

use std::sync::Arc;

use egui_dock::TabViewer;
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowAttributes, WindowId},
};

use atldp_geo::{
    dem::{wireframe_line_list, Dem, LocalGrid},
    profile::{extract_profile, ProfilePoint},
};
use atldp_render::{
    camera::{Camera2D, OrbitCamera},
    catenary_line::{CatenaryCallback, CatenaryLineResources, Vertex},
    terrain_mesh::{TerrainMeshCallback, TerrainMeshResources, TerrainVertex},
};

// ── terrain state (optional) ───────────────────────────────────────────────

struct TerrainData {
    grid: LocalGrid,
    /// Wireframe LINE_LIST vertices in local-plane metres.
    wireframe: Vec<TerrainVertex>,
    /// Ground profile sampled from SW to NE of the tile (200 points).
    profile: Vec<ProfilePoint>,
}

impl TerrainData {
    /// Load and prepare terrain from an HGT file.  Returns `None` on any error
    /// (e.g., file absent) so the app can run without terrain.
    fn load(path: &std::path::Path, sw_lat: i32, sw_lon: i32) -> Option<Self> {
        let bytes = std::fs::read(path).ok()?;
        let dem = Dem::from_hgt(&bytes, sw_lat, sw_lon)
            .map_err(|e| eprintln!("terrain: HGT parse error: {e}"))
            .ok()?;

        eprintln!(
            "terrain: loaded {} ({} rows × {} cols, grid = {:.0}×{:.0} km)",
            path.display(),
            dem.rows,
            dem.cols,
            dem.to_local_grid(2, 2).east_m / 1000.0,
            dem.to_local_grid(2, 2).north_m / 1000.0,
        );

        let (elev_min, elev_max) = dem.elev_stats();
        eprintln!("terrain: elevation {elev_min:.0}–{elev_max:.0} m");

        // 96×96 display grid (balances detail vs. vertex count).
        let grid = dem.to_local_grid(96, 96);
        let raw_verts = wireframe_line_list(&grid);
        let wireframe: Vec<TerrainVertex> = raw_verts
            .iter()
            .map(|&pos| TerrainVertex { pos })
            .collect();

        // Profile: SW corner → NE corner of the tile, 200 samples.
        let b = dem.bounds;
        let profile = extract_profile(
            &dem,
            b.lat_min + 0.1, b.lon_min + 0.1,
            b.lat_max - 0.1, b.lon_max - 0.1,
            200,
        );

        eprintln!("terrain: {} wireframe vertices, {} profile samples", wireframe.len(), profile.len());
        Some(Self { grid, wireframe, profile })
    }
}

// ── tab identifiers ────────────────────────────────────────────────────────────

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
            .with_title("ATLDP — terrain & route (G3)")
            .with_inner_size(winit::dpi::LogicalSize::new(1280u32, 800u32));
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

    span_m: f64,
    h_tension_n: f64,

    terrain: Option<TerrainData>,
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

        let mut dock_state = egui_dock::DockState::new(vec![Tab::View3D]);
        let [_, right] = dock_state
            .main_surface_mut()
            .split_right(egui_dock::NodeIndex::root(), 0.5, vec![Tab::View2D]);
        let _ = right;

        // Try to load terrain from ATLDP_TERRAIN env var or the default test
        // data path generated by fetch_srtm.sh.
        let terrain = Self::try_load_terrain();

        // Position the camera: if terrain is loaded, orbit its centre at a
        // distance that fits the full extent; otherwise fall back to catenary.
        let orbit = Self::init_camera(&terrain);

        // 2D camera: if terrain, centre on mid-profile; else catenary midspan.
        let cam2d = if let Some(ref t) = terrain {
            let mid_dist = t.profile.last().map(|p| p.distance_m as f32 * 0.5).unwrap_or(150.0);
            let mid_elev = t.grid.elev_min + (t.grid.elev_max - t.grid.elev_min) * 0.5;
            let mut c = Camera2D::new();
            c.center = [mid_dist, mid_elev];
            c.pixels_per_metre = 0.008; // 1 px ≈ 125 m for a ~100 km profile
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
            span_m: 300.0,
            h_tension_n: 30_000.0,
            terrain,
        }
    }

    fn try_load_terrain() -> Option<TerrainData> {
        // 1. ATLDP_TERRAIN env var.
        if let Ok(path) = std::env::var("ATLDP_TERRAIN") {
            let p = std::path::Path::new(&path);
            // Parse SW corner from filename like "S23W043.hgt".
            if let Some((sw_lat, sw_lon)) = parse_hgt_name(p) {
                return TerrainData::load(p, sw_lat, sw_lon);
            }
            eprintln!("terrain: could not parse lat/lon from ATLDP_TERRAIN filename; skipping");
        }

        // 2. Default test tile (generated by fetch_srtm.sh).
        let default = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../atldp-geo/tests/data/S23W043.hgt");
        if default.exists() {
            return TerrainData::load(&default, -23, -43);
        }

        eprintln!("terrain: no HGT file found — running without terrain.");
        eprintln!("         Set ATLDP_TERRAIN=/path/to/tile.hgt");
        eprintln!("         or run crates/atldp-geo/tests/fetch_srtm.sh");
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
            let mut c = OrbitCamera::new();
            c.target = glam::Vec3::new(150.0, -5.0, 0.0);
            c
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
        let cat_verts = catenary_vertices(self.span_m, self.h_tension_n);
        let size = self.window.inner_size();
        let aspect = size.width as f32 / size.height.max(1) as f32;
        let view_proj = self.orbit.view_proj(aspect).to_cols_array_2d();

        // toolbar
        egui::Panel::top("toolbar")
            .exact_size(32.0)
            .show_inside(ui, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.strong("ATLDP");
                    ui.separator();

                    // Terrain status indicator.
                    if let Some(ref t) = self.terrain {
                        ui.colored_label(
                            egui::Color32::from_rgb(80, 200, 120),
                            format!(
                                "Terrain: {:.0}×{:.0} km, {:.0}–{:.0} m",
                                t.grid.east_m / 1000.0,
                                t.grid.north_m / 1000.0,
                                t.grid.elev_min,
                                t.grid.elev_max,
                            ),
                        );
                    } else {
                        ui.colored_label(
                            egui::Color32::from_gray(120),
                            "No terrain  (set ATLDP_TERRAIN or run fetch_srtm.sh)",
                        );
                    }

                    ui.separator();
                    ui.label("Span:");
                    ui.add(
                        egui::DragValue::new(&mut self.span_m)
                            .range(10.0..=2000.0)
                            .speed(1.0)
                            .suffix(" m"),
                    );
                    ui.label("H tension:");
                    ui.add(
                        egui::DragValue::new(&mut self.h_tension_n)
                            .range(500.0..=300_000.0)
                            .speed(100.0)
                            .suffix(" N"),
                    );
                    ui.separator();
                    ui.label("V.Exag:");
                    ui.add(
                        egui::DragValue::new(&mut self.cam2d.vertical_exag)
                            .range(1.0..=50.0)
                            .speed(0.1)
                            .suffix("×"),
                    );
                    ui.separator();
                    let sag = cat_verts
                        .iter()
                        .map(|v| v.pos[1] as f64)
                        .fold(f64::INFINITY, f64::min)
                        .abs();
                    ui.label(format!("sag ≈ {sag:.2} m"));
                });
            });

        let terrain_ref = self.terrain.as_ref();

        egui::CentralPanel::default().show_inside(ui, |ui| {
            egui_dock::DockArea::new(&mut self.dock_state)
                .style(egui_dock::Style::from_egui(ui.style()))
                .show_inside(
                    ui,
                    &mut Viewer {
                        orbit: &mut self.orbit,
                        cam2d: &mut self.cam2d,
                        cat_verts: &cat_verts,
                        view_proj,
                        terrain: terrain_ref,
                    },
                );
        });
    }
}

// ── tab viewer ────────────────────────────────────────────────────────────────

struct Viewer<'a> {
    orbit: &'a mut OrbitCamera,
    cam2d: &'a mut Camera2D,
    cat_verts: &'a [Vertex],
    view_proj: [[f32; 4]; 4],
    terrain: Option<&'a TerrainData>,
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
    fn view3d(&mut self, ui: &mut egui::Ui) {
        let (rect, resp) = ui.allocate_exact_size(
            ui.available_size(),
            egui::Sense::click_and_drag(),
        );

        // Orbit: left-drag rotates, scroll zooms.
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

        // Terrain wireframe callback (drawn first so catenary is on top).
        if let Some(t) = self.terrain {
            let wf = t.wireframe.clone();
            let vp = self.view_proj;
            ui.painter().add(egui_wgpu::Callback::new_paint_callback(
                rect,
                TerrainMeshCallback {
                    vertices: wf,
                    view_proj: vp,
                    elev_min: t.grid.elev_min,
                    elev_max: t.grid.elev_max,
                },
            ));
        }

        // Catenary (shown when no terrain, or always in orbit mode).
        if self.terrain.is_none() {
            let verts: Vec<Vertex> = self.cat_verts.to_vec();
            let vp = self.view_proj;
            ui.painter().add(egui_wgpu::Callback::new_paint_callback(
                rect,
                CatenaryCallback { vertices: verts, view_proj: vp },
            ));
        }

        // HUD overlay.
        let terrain_label = self
            .terrain
            .map(|t| {
                format!(
                    "terrain: {:.0}×{:.0} km  elev {:.0}–{:.0} m",
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

        // Terrain profile (ground line): distance on X, elevation on Y.
        if let Some(t) = self.terrain {
            let profile_pts: Vec<egui::Pos2> = t
                .profile
                .iter()
                .filter(|p| !p.elevation_m.is_nan())
                .map(|p| {
                    let [sx, sy] = self
                        .cam2d
                        .world_to_screen([p.distance_m as f32, p.elevation_m as f32], vp);
                    egui::pos2(rect.left() + sx, rect.top() + sy)
                })
                .collect();

            if profile_pts.len() >= 2 {
                // Filled polygon to ground level.
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
                // Terrain profile line.
                let profile_stroke =
                    egui::Stroke::new(1.5, egui::Color32::from_rgb(100, 180, 60));
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
                            .world_to_screen([p.distance_m as f32, p.elevation_m as f32], vp);
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

        // Catenary overlay (distance = span position, Y = elevation relative to start).
        if self.cat_verts.len() >= 2 {
            let cat_stroke = egui::Stroke::new(2.0, egui::Color32::from_rgb(50, 200, 255));

            // Offset catenary to start at the right of the profile if terrain loaded.
            let cat_x_offset = self
                .terrain
                .and_then(|t| t.profile.last())
                .map(|p| p.distance_m as f32 * 0.4)
                .unwrap_or(0.0);

            // Find the mean terrain elevation at the catenary location for Y offset.
            let cat_y_offset = self
                .terrain
                .and_then(|t| {
                    let frac = 0.4_f64;
                    t.profile
                        .iter()
                        .min_by_key(|p| {
                            let total = t.profile.last().map(|p| p.distance_m).unwrap_or(1.0);
                            ((p.distance_m - frac * total).abs() * 1000.0) as i64
                        })
                        .filter(|p| !p.elevation_m.is_nan())
                        .map(|p| p.elevation_m)
                })
                .unwrap_or(0.0);

            let pts: Vec<egui::Pos2> = self
                .cat_verts
                .iter()
                .map(|v| {
                    let world_x = v.pos[0] + cat_x_offset;
                    let world_y = v.pos[1] + cat_y_offset;
                    let [sx, sy] = self.cam2d.world_to_screen([world_x, world_y], vp);
                    egui::pos2(rect.left() + sx, rect.top() + sy)
                })
                .collect();
            for w in pts.windows(2) {
                painter.line_segment([w[0], w[1]], cat_stroke);
            }
            for p in [pts.first(), pts.last()].into_iter().flatten() {
                painter.circle_stroke(*p, 5.0, egui::Stroke::new(2.0, egui::Color32::from_rgb(255, 200, 80)));
            }
        }

        // Status bar.
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

// ── catenary vertex sampling ──────────────────────────────────────────────────

fn catenary_vertices(span_m: f64, h_tension_n: f64) -> Vec<Vertex> {
    use atldp_core::catenary::solve_catenary;
    const W: f64 = 15.97; // ACSR Drake weight per unit length, N/m
    const N: usize = 120;

    let Ok(sol) = solve_catenary(span_m, 0.0, W, h_tension_n) else {
        return vec![];
    };
    let c = sol.catenary_constant();
    let a = sol.low_point_x;
    let b = -c * (a / c).cosh();

    (0..=N)
        .map(|i| {
            let x = i as f64 / N as f64 * span_m;
            let y = c * ((x - a) / c).cosh() + b;
            Vertex { pos: [x as f32, y as f32, 0.0] }
        })
        .collect()
}

// ── HGT filename parser ───────────────────────────────────────────────────────

/// Parse SW-corner lat/lon from a filename like `S23W043.hgt` or
/// `N48E002.hgt`.  Returns `None` if the filename doesn't match.
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
