use std::{
    collections::HashSet,
    marker::PhantomData,
    rc::Rc,
    sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}},
    thread::{self, ThreadId},
    time::Duration,
};
use eframe::{egui, egui_wgpu};
use windows::{
    core::{Interface, IUnknown},
    Win32::{Foundation::HWND, Graphics::DirectComposition::{
        DCompositionCreateDevice2, IDCompositionDevice, IDCompositionTarget, IDCompositionVisual,
    }},
};

type Result<T> = std::result::Result<T, String>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Present { Presented, Skipped(SkipReason) }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum SkipReason { ZeroExtent, Timeout, Occluded, Reconfigured }

#[derive(Default)]
struct Diagnostics { first: Mutex<Option<String>> }
impl Diagnostics {
    fn record(&self, message: String) {
        let mut first = self.first.lock().unwrap_or_else(|poison| poison.into_inner());
        if first.is_none() { *first = Some(message); }
    }
    fn check(&self, stage: &str) -> Result<()> {
        let first = self.first.lock().unwrap_or_else(|poison| poison.into_inner());
        match first.as_ref() {
            Some(error) => Err(format!("{stage}: {error}")),
            None => Ok(()),
        }
    }
}

struct FlushWrites {
    queue: wgpu::Queue,
    submitted: bool,
    cancelled: Arc<AtomicBool>,
    diagnostics: Arc<Diagnostics>,
}
impl Drop for FlushWrites {
    fn drop(&mut self) {
        let healthy = self.diagnostics.first.lock()
            .unwrap_or_else(|poison| poison.into_inner()).is_none();
        if !self.submitted && healthy && !std::thread::panicking()
            && !self.cancelled.load(Ordering::Acquire) {
            self.queue.submit(std::iter::empty::<wgpu::CommandBuffer>());
        }
    }
}

pub(super) struct Gpu {
    surface: Option<wgpu::Surface<'static>>,
    renderer: egui_wgpu::Renderer,
    queue: wgpu::Queue,
    device: wgpu::Device,
    adapter: wgpu::Adapter,
    instance: wgpu::Instance,
    format: wgpu::TextureFormat,
    configured: Option<[u32; 2]>,
    pending_free: HashSet<egui::TextureId>,

    _target: IDCompositionTarget,
    visual: IDCompositionVisual,
    composition: IDCompositionDevice,
    diagnostics: Arc<Diagnostics>,
    cancelled: Arc<AtomicBool>,
    owner: ThreadId,
    _ui_only: PhantomData<Rc<()>>,
}

impl Gpu {

    pub(super) fn new(hwnd: HWND, extent: [u32; 2], cancelled: Arc<AtomicBool>) -> Result<Self> {
        let active = |stage: &str| -> Result<()> {
            if cancelled.load(Ordering::Acquire) { Err(format!("{stage}: editor is closing")) }
            else { Ok(()) }
        };
        active("graphics creation")?;
        if hwnd.0.is_null() { return Err("graphics creation: missing host child HWND".into()); }

        let composition: IDCompositionDevice = unsafe {
            DCompositionCreateDevice2(None::<&IUnknown>)
        }.map_err(|error| format!("DCompositionCreateDevice2: {error}"))?;
        active("before composition target")?;
        let target = unsafe { composition.CreateTargetForHwnd(hwnd, false) }
            .map_err(|error| format!("CreateTargetForHwnd: {error}"))?;
        active("before composition visual")?;
        let visual = unsafe { composition.CreateVisual() }
            .map_err(|error| format!("CreateVisual: {error}"))?;
        active("before composition root")?;
        unsafe { target.SetRoot(&visual) }.map_err(|error| format!("SetRoot: {error}"))?;

        let mut setup = egui_wgpu::WgpuSetupCreateNew::without_display_handle();
        setup.instance_descriptor.backends = wgpu::Backends::DX12;
        setup.instance_descriptor.backend_options.dx12.shader_compiler = wgpu::Dx12Compiler::Fxc;
        active("before GPU instance")?;
        let instance = wgpu::Instance::new(setup.instance_descriptor);
        active("before composition surface")?;

        let surface: wgpu::Surface<'static> = unsafe {
            instance.create_surface_unsafe(wgpu::SurfaceTargetUnsafe::CompositionVisual(visual.as_raw()))
        }.map_err(|error| format!("create composition surface: {error}"))?;
        active("before adapter selection")?;
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: setup.power_preference,
            compatible_surface: Some(&surface),
            ..Default::default()
        })).map_err(|error| format!("request adapter: {error}"))?;
        active("before device creation")?;
        let descriptor = (setup.device_descriptor)(&adapter);
        let (device, queue) = pollster::block_on(adapter.request_device(&descriptor))
            .map_err(|error| format!("request device: {error}"))?;

        let diagnostics = Arc::new(Diagnostics::default());
        let errors = diagnostics.clone();
        device.on_uncaptured_error(Arc::new(move |error: wgpu::Error| {
            errors.record(format!("GPU error: {error}"));
        }));
        let lost = diagnostics.clone();
        device.set_device_lost_callback(move |reason, message| {
            lost.record(format!("GPU device lost ({reason:?}): {message}"));
        });
        active("before renderer creation")?;
        let capabilities = surface.get_capabilities(&adapter);
        let format = egui_wgpu::preferred_framebuffer_format(&capabilities.formats)
            .map_err(|error| format!("surface format: {error}"))?;
        let renderer = egui_wgpu::Renderer::new(&device, format, egui_wgpu::RendererOptions::default());
        diagnostics.check("renderer creation")?;
        let mut gpu = Self {
            surface: Some(surface), renderer, queue, device, adapter, instance,
            format, configured: None, pending_free: HashSet::new(),
            _target: target, visual, composition, diagnostics, cancelled,
            owner: thread::current().id(), _ui_only: PhantomData,
        };
        gpu.check("after renderer creation")?;
        if extent[0] != 0 && extent[1] != 0 { gpu.configure(extent)?; }
        Ok(gpu)
    }

    pub(super) fn max_texture_side(&self) -> usize {
        self.device.limits().max_texture_dimension_2d as usize
    }

    pub(super) fn check_health(&self) -> Result<()> {
        self.check("editor idle")
    }

    fn check(&self, stage: &str) -> Result<()> {
        if self.owner != thread::current().id() {
            return Err(format!("{stage}: graphics used outside its editor UI thread"));
        }
        if self.cancelled.load(Ordering::Acquire) {
            return Err(format!("{stage}: editor is closing"));
        }
        self.diagnostics.check(stage)
    }

    fn configure(&mut self, extent: [u32; 2]) -> Result<()> {
        self.check("before surface configuration")?;
        let limit = self.device.limits().max_texture_dimension_2d;
        if extent[0] == 0 || extent[1] == 0 || extent[0] > limit || extent[1] > limit {
            return Err(format!("surface configuration: unsupported client extent {extent:?} (limit {limit})"));
        }
        let surface = self.surface.as_ref().ok_or("surface configuration: missing surface")?;
        let capabilities = surface.get_capabilities(&self.adapter);
        if !capabilities.formats.contains(&self.format)
            || !capabilities.alpha_modes.contains(&wgpu::CompositeAlphaMode::Opaque) {
            return Err("surface configuration: selected format or opaque presentation is unsupported".into());
        }
        let mut configuration = surface.get_default_config(&self.adapter, extent[0], extent[1])
            .ok_or("surface configuration: no compatible configuration")?;
        configuration.format = self.format;
        configuration.alpha_mode = wgpu::CompositeAlphaMode::Opaque;

        let presentation = egui_wgpu::SurfaceConfig::HIGH_THROUGHPUT;
        configuration.present_mode = presentation.present_mode;
        if let Some(latency) = presentation.desired_maximum_frame_latency {
            configuration.desired_maximum_frame_latency = latency;
        }
        configuration.view_formats = vec![self.format];

        surface.configure(&self.device, &configuration);
        self.check("after surface configuration")?;

        let committed = unsafe { self.composition.Commit() };
        committed
            .map_err(|error| format!("composition Commit after configure: {error}"))?;
        self.check("after composition Commit")?;
        self.configured = Some(extent);
        Ok(())
    }

    fn recreate_surface(&mut self, extent: [u32; 2]) -> Result<()> {
        self.check("before surface recreation")?;
        drop(self.surface.take());
        self.configured = None;
        self.check("after old surface drop")?;

        let surface = unsafe {
            self.instance.create_surface_unsafe(wgpu::SurfaceTargetUnsafe::CompositionVisual(self.visual.as_raw()))
        }.map_err(|error| format!("recreate composition surface: {error}"))?;
        self.surface = Some(surface);
        self.check("after surface recreation")?;
        self.configure(extent)
    }

    fn skipped(&mut self, flush: &mut FlushWrites, reason: SkipReason) -> Result<Present> {
        self.check("before skipped-frame upload submit")?;
        self.queue.submit(std::iter::empty::<wgpu::CommandBuffer>());
        flush.submitted = true;
        self.check("after skipped-frame upload submit")?;

        for id in self.pending_free.drain() { self.renderer.free_texture(&id); }
        self.check("after skipped-frame texture cleanup")?;
        Ok(Present::Skipped(reason))
    }

    pub(super) fn render(
        &mut self,
        ctx: &egui::Context,
        extent: [u32; 2],
        pixels_per_point: f32,
        primitives: &[egui::ClippedPrimitive],
        textures: &mut egui::TexturesDelta,
    ) -> Result<Present> {
        self.check("before frame")?;
        if !pixels_per_point.is_finite() || pixels_per_point <= 0.0 {
            return Err("frame: invalid pixels_per_point".into());
        }
        let mut flush = FlushWrites {
            queue: self.queue.clone(), submitted: false,
            cancelled: self.cancelled.clone(), diagnostics: self.diagnostics.clone(),
        };

        for (id, deltas) in textures.set.drain() {
            self.pending_free.remove(&id);
            for delta in deltas {
                self.check("before texture upload")?;
                self.renderer.update_texture(&self.device, &self.queue, id, &delta);
            }
        }
        self.pending_free.extend(textures.free.drain());
        self.check("after texture upload")?;
        if extent[0] == 0 || extent[1] == 0 {
            return self.skipped(&mut flush, SkipReason::ZeroExtent);
        }
        if self.configured != Some(extent) { self.configure(extent)?; }
        self.check("before surface acquisition")?;
        let acquired = self.surface.as_ref().ok_or("frame: missing surface")?.get_current_texture();
        self.check("after surface acquisition")?;
        let (frame, suboptimal) = match acquired {
            wgpu::CurrentSurfaceTexture::Success(frame) => (frame, false),
            wgpu::CurrentSurfaceTexture::Suboptimal(frame) => (frame, true),
            wgpu::CurrentSurfaceTexture::Timeout => {
                ctx.request_repaint_after(Duration::from_millis(16));
                return self.skipped(&mut flush, SkipReason::Timeout);
            }
            wgpu::CurrentSurfaceTexture::Occluded => return self.skipped(&mut flush, SkipReason::Occluded),
            wgpu::CurrentSurfaceTexture::Outdated => {
                self.configure(extent)?;
                ctx.request_repaint();
                return self.skipped(&mut flush, SkipReason::Reconfigured);
            }
            wgpu::CurrentSurfaceTexture::Lost => {
                self.recreate_surface(extent)?;
                ctx.request_repaint();
                return self.skipped(&mut flush, SkipReason::Reconfigured);
            }
            wgpu::CurrentSurfaceTexture::Validation => {
                return Err("surface acquisition: validation failure (see GPU diagnostics)".into());
            }
        };
        let view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());
        self.check("before frame encoding")?;
        let screen = egui_wgpu::ScreenDescriptor { size_in_pixels: extent, pixels_per_point };
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Rocoflex host editor"),
        });
        let callbacks = self.renderer.update_buffers(&self.device, &self.queue, &mut encoder, primitives, &screen);
        self.check("after buffer upload")?;
        {
            let pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Rocoflex editor paint"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view, resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: f64::from(12.0f32 / 255.0), g: f64::from(12.0f32 / 255.0),
                            b: f64::from(12.0f32 / 255.0), a: f64::from(180.0f32 / 255.0),
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None, occlusion_query_set: None, multiview_mask: None,
            });
            self.renderer.render(&mut pass.forget_lifetime(), primitives, &screen);
        }
        self.check("after frame encoding")?;
        let commands = encoder.finish();
        self.check("before frame submit")?;
        self.queue.submit(callbacks.into_iter().chain([commands]));
        flush.submitted = true;
        self.check("after frame submit")?;

        for id in self.pending_free.drain() { self.renderer.free_texture(&id); }
        self.check("before frame present")?;
        self.queue.present(frame);
        self.check("after frame present")?;
        if suboptimal {

            self.configured = None;
            ctx.request_repaint();
        }
        Ok(Present::Presented)
    }
}

impl Drop for Gpu {
    fn drop(&mut self) {

        if let Some(surface) = self.surface.take() {
            drop(surface);
        }
    }
}
