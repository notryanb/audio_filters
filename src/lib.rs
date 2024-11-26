#![warn(clippy::all, rust_2018_idioms)]

mod app;

pub use app::AudioFilterApp;

pub struct StateVariableFilter {
    pub sample_rate: f32,
    g: f32, // cutoff freq
    k: f32, // resonance
    a1: f32,
    a2: f32,
    a3: f32,
    ic1eq: f32,
    ic2eq: f32,
}

impl StateVariableFilter {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            sample_rate,
            g: 0.0,
            k: 0.0,
            a1: 0.0,
            a2: 0.0,
            a3: 0.0,
            ic1eq: 0.0,
            ic2eq: 0.0,
        }
    }

    pub fn reset(&mut self) {
        self.g = 0.0;
        self.k = 0.0;
        self.a1 = 0.0;
        self.a2 = 0.0;
        self.a3 = 0.0;
        self.ic1eq = 0.0;
        self.ic2eq = 0.0;
    }

    pub fn update_coefficients(&mut self, cutoff: f32, q: f32) {
        self.g = (std::f32::consts::PI * cutoff / self.sample_rate).tan();
        self.k = 1.0 / q;
        self.a1 = 1.0 / (1.0 + self.g * (self.g + self.k));
        self.a2 = self.g * self.a1;
        self.a3 = self.g * self.a2;
    }

    pub fn render(&mut self, sample: f32) -> f32 {
        // v1..v3 are voltages at different nodes
        let v3 = sample - self.ic2eq;
        let v1 = self.a1 * self.ic1eq + self.a2 * v3;
        let v2 = self.ic2eq + self.a2 * self.ic1eq + self.a3 * v3; // lp
        self.ic1eq = 2.0 * v1 - self.ic1eq; // state of capacitors
        self.ic2eq = 2.0 * v2 - self.ic2eq; // state of capacitors
        v2
    }
}


