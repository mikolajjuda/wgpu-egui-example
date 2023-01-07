use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

mod state;
use state::State;
mod background_drawing;
mod gui_drawing;

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
            state.update(&window);
            match state.render(&window) {
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
