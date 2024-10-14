

pub struct AudioFilterApp {
}

impl Default for AudioFilterApp {
    fn default() -> Self {
        Self {}
    }
}

impl AudioFilterApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        Default::default()
    }
}

impl eframe::App for AudioFilterApp {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {

    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("Top Panel").show(ctx, |ui| {
            ui.heading("Audio Filter App");
        });
    }
}
