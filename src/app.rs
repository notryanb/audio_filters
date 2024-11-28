use egui::epaint::*;
use egui::{pos2, vec2, Frame, Pos2, Rect};
use egui_plot::{Line, Plot, PlotPoints};
use std::sync::mpsc::Sender;

use crate::{FirLowPassFilter, StateVariableFilter, StateVariableTPTFilter};

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
    pub filter_freq_res: Option<Vec<f32>>,
}

impl Default for AudioFilterApp {
    fn default() -> Self {
        Self {
            vol: 0.3,
            freq_hz: 1000.0,
            resonance_q: 0.707,
            audio_tx: None,
            filter_freq_res: None,
        }
    }
}

pub trait AppComponent {
    type Context;
    fn add(ctx: &mut Self::Context, ui: &mut eframe::egui::Ui);
}

impl AudioFilterApp {
    pub fn new() -> Self {
        Default::default()
    }
}

impl eframe::App for AudioFilterApp {
    fn save(&mut self, _storage: &mut dyn eframe::Storage) {}

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if *(&self.filter_freq_res.is_none()) {
            use realfft::num_complex::ComplexFloat;
            use realfft::RealFftPlanner;

            let sample_rate = 44100;

            let mut svf = FirLowPassFilter::new(sample_rate as f32);
            //let mut svf = StateVariableTPTFilter::new(sample_rate as f32);
            //let mut svf = StateVariableFilter::new(sample_rate as f32);
            //svf.update_coefficients(2000.0, 0.707);
            let mut impulse: Vec<f32> = (0..sample_rate).map(|_| 0.0).collect();
            impulse[0] = 1.0;

            let mut impulse_response = impulse
                .iter()
                .map(|sample| svf.render(*sample))
                .collect::<Vec<f32>>();

            let mut real_planner = RealFftPlanner::<f32>::new();
            let r2c = real_planner.plan_fft_forward(sample_rate);
            let mut spectrum = r2c.make_output_vec();
            r2c.process(&mut impulse_response[..], &mut spectrum)
                .expect("failed to process FFT");

            // instead of graphing the impulse response, calculate the FFT of the impulse response.
            // Ensure the 20 * log10(spectrum_bin.abs()) is used to plot the absolute value of the
            // spectrum bin converted to dB
            self.filter_freq_res = Some(
                spectrum
                    .iter()
                    .map(|f| (f.re.abs() + std::f32::EPSILON).log10() * 20.0)
                    .collect(),
            );
            //self.filter_freq_res = Some(impulse_response);
        }

        egui::TopBottomPanel::top("Top Panel").show(ctx, |ui| {
            ui.heading("Audio Filter App");
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            let volume_slider = ui.add(egui::Slider::new(&mut self.vol, 0.0..=1.0).text("Volume"));
            let freq_slider = ui.add(
                egui::Slider::new(&mut self.freq_hz, 20.0..=18000.0)
                    .text("Frequency Hz")
                    .logarithmic(true),
            );
            let reso_slider = ui.add(
                egui::Slider::new(&mut self.resonance_q, 0.707..=50.0)
                    .text("Resonance (Q)")
                    .logarithmic(true),
            );

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

            // for 0 to half-nyquist, plot frequency response
            if let Some(filter_freq_res) = &self.filter_freq_res {
                let sin: PlotPoints = filter_freq_res
                    .iter()
                    .enumerate()
                    .map(|(x, y)| [x as f64, *y as f64])
                    .collect();
                let line = Line::new(sin);
                Plot::new("frequencies")
                    .view_aspect(2.0)
                    .show(ui, |plot_ui| plot_ui.line(line));
            }
        });
    }
}
