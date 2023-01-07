use egui;
use egui_wgpu;
use egui_winit;
use winit::{event::WindowEvent, window::Window};

pub struct GuiState {
    pub egui: egui::Context,
    pub egui_winit_state: egui_winit::State,
    pub egui_renderer: egui_wgpu::renderer::Renderer,
    pub egui_output: Option<egui::FullOutput>,
}

impl GuiState {
    pub fn new(
        device: &wgpu::Device,
        event_loop: &winit::event_loop::EventLoop<()>,
        texture_format: wgpu::TextureFormat,
    ) -> Self {
        let egui_renderer = egui_wgpu::renderer::Renderer::new(&device, texture_format, None, 1);
        GuiState {
            egui: egui::Context::default(),
            egui_winit_state: egui_winit::State::new(event_loop),
            egui_renderer: egui_renderer,
            egui_output: None,
        }
    }

    pub fn input(&mut self, event: &WindowEvent) -> bool {
        let response = self.egui_winit_state.on_event(&self.egui, event);
        response.consumed
    }

    pub fn update(&mut self, window: &Window) {
        let output = self
            .egui
            .run(self.egui_winit_state.take_egui_input(window), |ctx| {
                egui::Window::new("test window").show(ctx, |ui| {
                    ui.heading("wgpu and egui integration example");
                });
            });
        self.egui_winit_state.handle_platform_output(
            window,
            &self.egui,
            output.platform_output.to_owned(),
        );
        self.egui_output = Some(output);
    }

    pub fn render(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        window: &Window,
        window_size: winit::dpi::PhysicalSize<u32>,
    ) {
        if self.egui_output.is_none() {
            return;
        }
        let egui_output = self.egui_output.take().unwrap();
        for (id, image_delta) in egui_output.textures_delta.set.as_slice() {
            self.egui_renderer
                .update_texture(device, queue, *id, &image_delta);
        }
        {
            let paint_jobs = self.egui.tessellate(egui_output.shapes);
            let screen_descriptor = egui_wgpu::renderer::ScreenDescriptor {
                size_in_pixels: [window_size.width, window_size.height],
                pixels_per_point: window.scale_factor() as f32,
            };

            self.egui_renderer.update_buffers(
                device,
                queue,
                encoder,
                &paint_jobs,
                &screen_descriptor,
            );

            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });

            self.egui_renderer
                .render(&mut render_pass, &paint_jobs, &screen_descriptor);
        }
        for id in egui_output.textures_delta.free {
            self.egui_renderer.free_texture(&id);
        }
    }
}
