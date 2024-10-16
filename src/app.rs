use std::sync::mpsc::Sender;

pub enum AudioCommand {
    SetVolume(f32),
    SetFilterFreq(f32),
    SetResonance(f32),
}

pub struct AudioFilterApp {
    pub vol: f32,
    pub freq_hz: f32,
    pub resonance_q: f32,
    pub audio_tx: Option<Sender<crate::app::AudioCommand>>,
}

impl Default for AudioFilterApp {
    fn default() -> Self {
        Self {
            vol: 0.3,
            freq_hz: 1000.0,
            resonance_q: 0.707,
            audio_tx: None,
        }
    }
}

impl AudioFilterApp {
    pub fn new() -> Self {
        Default::default()
    }
}

impl eframe::App for AudioFilterApp {
    fn save(&mut self, _storage: &mut dyn eframe::Storage) {

    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("Top Panel").show(ctx, |ui| {
            ui.heading("Audio Filter App");
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            let volume_slider = ui.add(egui::Slider::new(&mut self.vol, 0.0..=1.0).text("Volume").logarithmic(true));
            let freq_slider = ui.add(egui::Slider::new(&mut self.freq_hz, 20.0..=18000.0).text("Frequency Hz").logarithmic(true));
            let reso_slider = ui.add(egui::Slider::new(&mut self.resonance_q, 0.707..=50.0).text("Resonance (Q)").logarithmic(true));

            if volume_slider.dragged() {
                if let Some(tx) = &self.audio_tx {
                    _ = tx.send(AudioCommand::SetVolume(self.vol));
                }
            }

            if freq_slider.dragged() {
                if let Some(tx) = &self.audio_tx {
                    _ = tx.send(AudioCommand::SetFilterFreq(self.freq_hz));
                }
            }

            if reso_slider.dragged() {
                if let Some(tx) = &self.audio_tx {
                    _ = tx.send(AudioCommand::SetResonance(self.resonance_q));
                }
            }
        });
    }
}
