use android_activity::ndk::native_window::NativeWindow;
use eframe::egui_wgpu::{RenderState, RendererOptions, ScreenDescriptor};
use raw_window_handle::{
    DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, WindowHandle,
};
use std::sync::Arc;

struct SurfaceWindow(NativeWindow);
impl HasWindowHandle for SurfaceWindow {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        self.0.window_handle()
    }
}
impl HasDisplayHandle for SurfaceWindow {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
        Ok(DisplayHandle::android())
    }
}

/// GPU resources outlive transient native surfaces, but never their owning activity.
pub(super) struct Renderer {
    instance: wgpu::Instance,
    state: RenderState,
    // Drop the surface (and its retained NativeWindow) during TerminateWindow's callback.
    surface: Option<wgpu::Surface<'static>>,
    config: wgpu::SurfaceConfiguration,
}
impl Renderer {
    pub fn new(window: NativeWindow, options: &crate::NativeOptions) -> Result<Self, crate::Error> {
        let instance = pollster::block_on(options.wgpu_options.wgpu_setup.new_instance());
        let size = [window.width() as u32, window.height() as u32];
        let surface = instance
            .create_surface(Arc::new(SurfaceWindow(window.clone())))
            .map_err(failure)?;
        let state = pollster::block_on(RenderState::create(
            &options.wgpu_options,
            &instance,
            Some(&surface),
            RendererOptions {
                dithering: options.dithering,
                ..Default::default()
            },
        ))?;
        let mut config = surface
            .get_default_config(&state.adapter, size[0].max(1), size[1].max(1))
            .ok_or_else(|| failure("no compatible Android surface configuration"))?;
        config.format = state.target_format;
        config.present_mode = options.wgpu_options.present_mode;
        config.desired_maximum_frame_latency = options
            .wgpu_options
            .desired_maximum_frame_latency
            .unwrap_or(2);
        surface.configure(&state.device, &config);
        #[cfg(debug_assertions)]
        eprintln!(
            "rust-flutter android surface native_format={:?} target_format={:?}",
            window.format(),
            config.format
        );
        Ok(Self {
            instance,
            state,
            surface: Some(surface),
            config,
        })
    }
    pub fn attach(&mut self, window: NativeWindow) -> Result<(), crate::Error> {
        self.detach();
        let size = [window.width() as u32, window.height() as u32];
        let surface = self
            .instance
            .create_surface(Arc::new(SurfaceWindow(window.clone())))
            .map_err(failure)?;
        if !surface
            .get_capabilities(&self.state.adapter)
            .formats
            .contains(&self.config.format)
        {
            return Err(failure(
                "Android surface changed to an incompatible pixel format",
            ));
        }
        self.config.width = size[0].max(1);
        self.config.height = size[1].max(1);
        surface.configure(&self.state.device, &self.config);
        #[cfg(debug_assertions)]
        eprintln!(
            "rust-flutter android surface native_format={:?} target_format={:?}",
            window.format(),
            self.config.format
        );
        self.surface = Some(surface);
        Ok(())
    }
    pub fn has_surface(&self) -> bool {
        self.surface.is_some()
    }
    pub fn detach(&mut self) {
        self.surface = None;
    }
    pub fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        if self.config.width != width || self.config.height != height {
            self.config.width = width;
            self.config.height = height;
            if let Some(surface) = &self.surface {
                surface.configure(&self.state.device, &self.config);
            }
        }
    }
    pub fn draw(
        &mut self,
        ctx: &egui::Context,
        output: &egui::FullOutput,
    ) -> Result<bool, crate::Error> {
        let mut renderer = self.state.renderer.write();
        // Upload deltas even if acquisition fails: egui will not resend its texture atlas.
        for (id, delta) in &output.textures_delta.set {
            renderer.update_texture(&self.state.device, &self.state.queue, *id, delta);
        }
        let Some(surface) = &self.surface else {
            for id in &output.textures_delta.free {
                renderer.free_texture(id);
            }
            return Ok(false);
        };
        let frame = match surface.get_current_texture() {
            Ok(frame) => frame,
            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                for id in &output.textures_delta.free {
                    renderer.free_texture(id);
                }
                surface.configure(&self.state.device, &self.config);
                return Ok(false);
            }
            Err(wgpu::SurfaceError::Timeout | wgpu::SurfaceError::Other) => {
                for id in &output.textures_delta.free {
                    renderer.free_texture(id);
                }
                return Ok(false);
            }
            Err(wgpu::SurfaceError::OutOfMemory) => {
                return Err(failure("Android GPU surface ran out of memory"));
            }
        };
        let jobs = ctx.tessellate(output.shapes.clone(), output.pixels_per_point);
        let screen = ScreenDescriptor {
            size_in_pixels: [self.config.width, self.config.height],
            pixels_per_point: output.pixels_per_point,
        };
        let mut encoder =
            self.state
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Rust Flutter Android frame"),
                });
        let callbacks = renderer.update_buffers(
            &self.state.device,
            &self.state.queue,
            &mut encoder,
            &jobs,
            &screen,
        );
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        {
            let pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Rust Flutter Android UI"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            renderer.render(&mut pass.forget_lifetime(), &jobs, &screen);
        }
        self.state.queue.submit(
            callbacks
                .into_iter()
                .chain(std::iter::once(encoder.finish())),
        );
        frame.present();
        for id in &output.textures_delta.free {
            renderer.free_texture(id);
        }
        Ok(true)
    }
}
pub(super) fn failure(error: impl std::fmt::Display) -> crate::Error {
    crate::Error::AppCreation(Box::new(std::io::Error::other(error.to_string())))
}
