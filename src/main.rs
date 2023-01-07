use bytemuck;
use oorandom;
use wgpu::util::DeviceExt;
use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex2dColor {
    position: [f32; 2],
    color: [f32; 3],
}

impl Vertex2dColor {
    const ATTRIBS: [wgpu::VertexAttribute; 2] =
        wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x3];
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex2dColor>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

const VERTICES: &[Vertex2dColor] = &[
    Vertex2dColor {
        position: [0.0, 1.0],
        color: [1.0, 0.0, 0.0],
    },
    Vertex2dColor {
        position: [-1.0, -1.0],
        color: [0.0, 1.0, 0.0],
    },
    Vertex2dColor {
        position: [1.0, -1.0],
        color: [0.0, 0.0, 1.0],
    },
];

const INDICES: &[u16] = &[0, 1, 2];

struct State {
    // global rendering stuff
    surface: wgpu::Surface,
    surface_config: wgpu::SurfaceConfiguration,
    device: wgpu::Device,
    queue: wgpu::Queue,
    window_size: winit::dpi::PhysicalSize<u32>,
    // stuff for rendering color triangle
    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    num_indices: u32,
    //stuff for background color
    background_color: wgpu::Color,
    rng: oorandom::Rand32,
    // egui stuff
    egui: egui::Context,
    egui_winit_state: egui_winit::State,
    egui_renderer: egui_wgpu::renderer::Renderer,
}

impl State {
    async fn new(event_loop: &winit::event_loop::EventLoop<()>, window: &Window) -> Self {
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

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("simple color shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("simple_color_shader.wgsl").into()),
        });

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[],
                push_constant_ranges: &[],
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[Vertex2dColor::desc()],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview: None,
        });

        let egui_renderer =
            egui_wgpu::renderer::Renderer::new(&device, surface_config.format, None, 1);

        State {
            surface: surface,
            surface_config: surface_config,
            device: device,
            queue: queue,
            window_size: size,
            background_color: wgpu::Color {
                r: 0.1,
                g: 0.2,
                b: 0.3,
                a: 1.0,
            },
            rng: oorandom::Rand32::new(2),
            render_pipeline: render_pipeline,
            vertex_buffer: vertex_buffer,
            index_buffer: index_buffer,
            num_indices: INDICES.len() as u32,
            egui: egui::Context::default(),
            egui_winit_state: egui_winit::State::new(event_loop),
            egui_renderer: egui_renderer,
        }
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.window_size = new_size;
            self.surface_config.width = new_size.width;
            self.surface_config.height = new_size.height;
            self.surface.configure(&self.device, &self.surface_config);
        }
    }

    fn input(&mut self, event: &WindowEvent) -> bool {
        let response = self.egui_winit_state.on_event(&self.egui, event);
        if response.consumed {
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
                                    self.background_color.r = self.rng.rand_float() as f64;
                                    self.background_color.g = self.rng.rand_float() as f64;
                                    self.background_color.b = self.rng.rand_float() as f64;
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

    fn update(&mut self) {}

    fn ui(&mut self, window: &Window) -> egui::FullOutput {
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
        output
    }

    fn render(
        &mut self,
        window: &Window,
        egui_output: egui::FullOutput,
    ) -> Result<(), wgpu::SurfaceError> {
        //stuff for rendering this frame
        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        //stuff for rendering triangle
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(self.background_color),
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..self.num_indices, 0, 0..1);
        }
        //stuff for rendering egui
        for (id, image_delta) in egui_output.textures_delta.set {
            self.egui_renderer
                .update_texture(&self.device, &self.queue, id, &image_delta);
        }
        {
            let paint_jobs = self.egui.tessellate(egui_output.shapes);
            let screen_descriptor = egui_wgpu::renderer::ScreenDescriptor {
                size_in_pixels: [self.window_size.width, self.window_size.height],
                pixels_per_point: window.scale_factor() as f32,
            };

            self.egui_renderer.update_buffers(
                &self.device,
                &self.queue,
                &mut encoder,
                &paint_jobs,
                &screen_descriptor,
            );

            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
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

        //submitting frame
        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}

async fn run() {
    env_logger::init();
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("wgpu with egui example")
        .with_inner_size(winit::dpi::PhysicalSize {
            width: 600u32,
            height: 600u32,
        })
        .build(&event_loop)
        .unwrap();
    let mut state = State::new(&event_loop, &window).await;

    event_loop.run(move |event, _, control_flow| match event {
        Event::WindowEvent {
            window_id,
            ref event,
        } if window_id == window.id() && !state.input(event) => match event {
            WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
            WindowEvent::Resized(physical_size) => {
                state.resize(*physical_size);
            }
            WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                state.resize(**new_inner_size);
            }
            _ => {}
        },
        Event::RedrawRequested(window_id) if window_id == window.id() => {
            state.update();
            let egui_output = state.ui(&window);
            match state.render(&window, egui_output) {
                Ok(_) => {}
                Err(wgpu::SurfaceError::Lost) => state.resize(state.window_size),
                Err(wgpu::SurfaceError::OutOfMemory) => *control_flow = ControlFlow::Exit,
                Err(e) => eprintln!("{:?}", e),
            }
        }
        Event::MainEventsCleared => {
            window.request_redraw();
        }
        _ => {}
    })
}

fn main() {
    pollster::block_on(run());
}
