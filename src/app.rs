use egui::Context;

pub struct App {
    test: f32,
}

impl App {
    pub fn new() -> App {
        App { test: 0.0 }
    }

    pub fn ui(&mut self, context: &Context) {
        egui::Window::new("Window").show(context, |ui| {
            ui.label("Hello world!");
            ui.drag_angle(&mut self.test);
        });
    }

    pub fn draw(&mut self) {}
}
