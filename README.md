# Example wgpu and egui integration

Low level example of using `egui-wgpu` and `egui-winit` to draw `egui` with `wgpu`.

Boilerplate taken from [https://sotrh.github.io/learn-wgpu](https://sotrh.github.io/learn-wgpu)

For simplicity egui's indication if it needs redrawing is ignored here.
If your app doesn't need redrawing every frame look at [egui::FullOutput.repaint_after](https://docs.rs/egui/0.20.1/egui/struct.FullOutput.html#structfield.repaint_after) and [egui-winit::EventResponse.repaint](https://docs.rs/egui-winit/0.20.1/egui_winit/struct.EventResponse.html#structfield.repaint)
