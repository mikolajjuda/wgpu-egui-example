use winit::{event::*, window::Window};

use super::background_drawing::BackgroundState;
use super::gui_drawing::GuiState;
pub struct State {
    // global rendering stuff
    pub surface: wgpu::Surface,
    pub surface_config: wgpu::SurfaceConfiguration,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub window_size: winit::dpi::PhysicalSize<u32>,
    // background stuff
    pub background_state: BackgroundState,
    //gui stuff
    pub gui_state: GuiState,
}

impl State {
    pub async fn new(event_loop: &winit::event_loop::EventLoop<()>, window: &Window) -> Self {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(wgpu::Backends::all());
        let surface = unsafe { instance.create_surface(window) };
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    features: wgpu::Features::empty(),
                    limits: wgpu::Limits::default(),
                    label: None,
                },
                None,
            )
            .await
            .unwrap();

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: egui_wgpu::preferred_framebuffer_format(
                surface.get_supported_formats(&adapter).as_slice(),
            ),
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
        };
        surface.configure(&device, &surface_config);

        let background_state = BackgroundState::new(&device, surface_config.format);
        let gui_state = GuiState::new(&device, event_loop, surface_config.format);
        State {
            surface: surface,
            surface_config: surface_config,
            device: device,
            queue: queue,
            window_size: size,
            background_state: background_state,
            gui_state: gui_state,
        }
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.window_size = new_size;
            self.surface_config.width = new_size.width;
            self.surface_config.height = new_size.height;
            self.surface.configure(&self.device, &self.surface_config);
        }
    }

    pub fn input(&mut self, event: &WindowEvent) -> bool {
        if self.gui_state.input(event) {
            return true;
        }
        match event {
            WindowEvent::KeyboardInput {
                device_id: _,
                input,
                is_synthetic,
            } => {
                if !is_synthetic {
                    #[allow(deprecated)]
                    let KeyboardInput {
                        scancode: _,
                        state,
                        virtual_keycode,
                        modifiers,
                    } = input;
                    if let Some(key_code) = virtual_keycode {
                        match key_code {
                            VirtualKeyCode::Space => {
                                if *state == ElementState::Pressed && modifiers.is_empty() {
                                    self.background_state.randomize_color();
                                    return true;
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
            _ => {}
        }
        false
    }

    pub fn update(&mut self, window: &Window) {
        self.gui_state.update(window);
    }

    pub fn render(&mut self, window: &Window) -> Result<(), wgpu::SurfaceError> {
        //stuff for rendering this frame
        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

        //rendering stuff behind gui
        self.background_state.render(&mut encoder, &view);

        //rendering egui
        self.gui_state.render(
            &self.device,
            &self.queue,
            &mut encoder,
            &view,
            window,
            self.window_size,
        );

        //submitting frame
        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}
