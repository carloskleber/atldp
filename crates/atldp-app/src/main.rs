//! ATLDP desktop CAD application — G2 render foundation (ADR-0011, ADR-0012).
//!
//! winit + egui + wgpu desktop shell:
//! - egui_dock docked layout (toolbar / 3D orbit viewport / 2D plan viewport)
//! - 3D viewport: wgpu LINE_STRIP catenary, orbit camera (left-drag + scroll)
//! - 2D viewport: egui Painter catenary + grid (right-drag pan, scroll zoom)
//! - Live catenary parameters (span, horizontal tension) from toolbar controls

use std::sync::Arc;

use egui_dock::TabViewer;
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowAttributes, WindowId},
};

use atldp_render::{
    camera::{Camera2D, OrbitCamera},
    catenary_line::{CatenaryCallback, CatenaryLineResources, Vertex},
};

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
            .with_title("ATLDP — render foundation (G2)")
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

    // catenary parameters (editable from UI)
    span_m: f64,
    h_tension_n: f64,
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

        // egui integration
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

        let mut egui_renderer = egui_wgpu::Renderer::new(&device, fmt, egui_wgpu::RendererOptions::default());
        egui_renderer
            .callback_resources
            .insert(CatenaryLineResources::new(&device, fmt));

        // default dock: 3D on left, 2D on right
        let mut dock_state = egui_dock::DockState::new(vec![Tab::View3D]);
        let [_, right] = dock_state
            .main_surface_mut()
            .split_right(egui_dock::NodeIndex::root(), 0.5, vec![Tab::View2D]);
        let _ = right;

        // default camera target at midspan, mid-sag
        let orbit = {
            let mut c = OrbitCamera::new();
            c.target = glam::Vec3::new(150.0, -5.0, 0.0);
            c
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
            cam2d: Camera2D::new(),
            span_m: 300.0,
            h_tension_n: 30_000.0,
        }
    }

    fn on_event(&mut self, event_loop: &ActiveEventLoop, event: WindowEvent) {
        let resp = self.egui_winit.on_window_event(&self.window, &event);

        if resp.consumed {
            // egui handled it — still need to process resize/close
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

        // build egui frame
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
        // precompute catenary vertices from current params
        let verts = catenary_vertices(self.span_m, self.h_tension_n);
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
                    let sag = verts
                        .iter()
                        .map(|v| v.pos[1] as f64)
                        .fold(f64::INFINITY, f64::min)
                        .abs();
                    ui.label(format!("sag ≈ {sag:.2} m"));
                });
            });

        // dock panels
        egui::CentralPanel::default().show_inside(ui, |ui| {
            egui_dock::DockArea::new(&mut self.dock_state)
                .style(egui_dock::Style::from_egui(ui.style()))
                .show_inside(
                    ui,
                    &mut Viewer {
                        orbit: &mut self.orbit,
                        cam2d: &mut self.cam2d,
                        verts: &verts,
                        view_proj,
                    },
                );
        });
    }
}

// ── tab viewer ────────────────────────────────────────────────────────────────

struct Viewer<'a> {
    orbit: &'a mut OrbitCamera,
    cam2d: &'a mut Camera2D,
    verts: &'a [Vertex],
    view_proj: [[f32; 4]; 4],
}

impl TabViewer for Viewer<'_> {
    type Tab = Tab;

    fn title(&mut self, tab: &mut Tab) -> egui::WidgetText {
        match tab {
            Tab::View3D => "3D view".into(),
            Tab::View2D => "2D plan".into(),
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

        // orbit: left-drag rotates, scroll zooms
        if resp.dragged_by(egui::PointerButton::Primary) {
            let d = resp.drag_delta();
            self.orbit.yaw -= d.x * 0.005;
            self.orbit.pitch =
                (self.orbit.pitch - d.y * 0.005).clamp(-1.45, 1.45);
        }
        let scroll = ui.input(|i| i.smooth_scroll_delta.y);
        if resp.hovered() && scroll != 0.0 {
            self.orbit.distance = (self.orbit.distance - scroll * 5.0).max(5.0);
        }

        // kick off the wgpu catenary draw
        let verts: Vec<Vertex> = self.verts.to_vec();
        let vp = self.view_proj;
        ui.painter().add(egui_wgpu::Callback::new_paint_callback(
            rect,
            CatenaryCallback { vertices: verts, view_proj: vp },
        ));

        // overlay info
        ui.painter().text(
            rect.left_bottom() + egui::vec2(6.0, -6.0),
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
    }

    fn view2d(&mut self, ui: &mut egui::Ui) {
        let (rect, resp) = ui.allocate_exact_size(
            ui.available_size(),
            egui::Sense::click_and_drag(),
        );
        let painter = ui.painter_at(rect);

        // pan: right-drag, zoom: scroll
        if resp.dragged_by(egui::PointerButton::Secondary) {
            let d = resp.drag_delta();
            let s = self.cam2d.pixels_per_metre;
            self.cam2d.center[0] -= d.x / s;
            self.cam2d.center[1] += d.y / s;
        }
        let scroll = ui.input(|i| i.smooth_scroll_delta.y);
        if resp.hovered() && scroll != 0.0 {
            self.cam2d.pixels_per_metre =
                (self.cam2d.pixels_per_metre * (1.0 + scroll * 0.01)).clamp(0.05, 200.0);
        }

        // background
        painter.rect_filled(rect, 0.0, egui::Color32::from_rgb(18, 18, 22));

        // grid — spacing adaptive to zoom
        let s = self.cam2d.pixels_per_metre;
        let raw = 60.0 / s; // target ~60px between lines
        let exp = raw.log10().floor() as i32;
        let grid_m = (10.0_f32).powi(exp) * [1.0, 2.0, 5.0]
            .iter()
            .copied()
            .find(|&v| v * s >= 40.0)
            .unwrap_or(1.0);
        let vp = [rect.width(), rect.height()];
        let grid_stroke = egui::Stroke::new(0.5, egui::Color32::from_gray(45));

        let world_left = self.cam2d.center[0] - vp[0] / (2.0 * s);
        let world_right = self.cam2d.center[0] + vp[0] / (2.0 * s);
        let world_top = self.cam2d.center[1] + vp[1] / (2.0 * s);
        let world_bot = self.cam2d.center[1] - vp[1] / (2.0 * s);

        let gx0 = (world_left / grid_m).floor() as i32;
        let gx1 = (world_right / grid_m).ceil() as i32;
        for gx in gx0..=gx1 {
            let wx = gx as f32 * grid_m;
            let sx = self.cam2d.world_to_screen([wx, 0.0], vp)[0] + rect.left();
            painter.line_segment(
                [egui::pos2(sx, rect.top()), egui::pos2(sx, rect.bottom())],
                grid_stroke,
            );
        }
        let gy0 = (world_bot / grid_m).floor() as i32;
        let gy1 = (world_top / grid_m).ceil() as i32;
        for gy in gy0..=gy1 {
            let wy = gy as f32 * grid_m;
            let sy = self.cam2d.world_to_screen([0.0, wy], vp)[1] + rect.top();
            painter.line_segment(
                [egui::pos2(rect.left(), sy), egui::pos2(rect.right(), sy)],
                grid_stroke,
            );
        }

        // catenary curve
        if self.verts.len() >= 2 {
            let cat_stroke = egui::Stroke::new(2.0, egui::Color32::from_rgb(50, 200, 255));
            let pts: Vec<egui::Pos2> = self
                .verts
                .iter()
                .map(|v| {
                    let [sx, sy] = self.cam2d.world_to_screen([v.pos[0], v.pos[1]], vp);
                    egui::pos2(rect.left() + sx, rect.top() + sy)
                })
                .collect();
            for w in pts.windows(2) {
                painter.line_segment([w[0], w[1]], cat_stroke);
            }
            // support pegs
            let peg = egui::Stroke::new(2.0, egui::Color32::from_rgb(255, 200, 80));
            for p in [pts.first(), pts.last()].into_iter().flatten() {
                painter.circle_stroke(*p, 5.0, peg);
            }
        }

        // axis labels
        let lbl_col = egui::Color32::from_gray(100);
        painter.text(
            rect.left_bottom() + egui::vec2(6.0, -18.0),
            egui::Align2::LEFT_BOTTOM,
            format!(
                "grid {grid_m:.0} m   zoom {:.2} px/m   right-drag pan",
                self.cam2d.pixels_per_metre
            ),
            egui::FontId::monospace(11.0),
            lbl_col,
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
    let b = -c * (a / c).cosh(); // shift so y(0) = 0

    (0..=N)
        .map(|i| {
            let x = i as f64 / N as f64 * span_m;
            let y = c * ((x - a) / c).cosh() + b;
            Vertex { pos: [x as f32, y as f32, 0.0] }
        })
        .collect()
}

// ── entry point ───────────────────────────────────────────────────────────────

fn main() {
    env_logger::init();
    let event_loop = EventLoop::new().expect("event loop");
    event_loop.run_app(&mut App::default()).expect("run");
}
