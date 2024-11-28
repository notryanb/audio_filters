#![warn(clippy::all, rust_2018_idioms)]

mod app;

pub use app::AudioFilterApp;

pub enum FilterType {
    LowPass,
    HighPass,
    BandPass,
}

pub struct BiQuadFilter {
    sample_rate: f32,
    b0: f32,
    b1: f32,
    b2: f32,
    a0: f32,
    a1: f32,
    a2: f32,
    x1: f32,
    x2: f32,
    y1: f32,
    y2: f32,
}

impl BiQuadFilter {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            sample_rate,
            b0: 1.0,
            a0: 1.0,
            a1: 0.0,
            a2: 0.0,
            b1: 0.0,
            b2: 0.0,
            x1: 0.0,
            x2: 0.0,
            y1: 0.0,
            y2: 0.0,
        }
    }

    pub fn update_coefficients(&mut self, cutoff_frequency: f32, resonance: f32) {
        let w0 = 2.0 * std::f32::consts::PI * cutoff_frequency / self.sample_rate;
        let alpha = w0.sin() / (2.0 * resonance);

        // Low Pass
        //self.b0 = (1 - w0.cos()) / 2.0;
        self.b1 = 1.0 - w0.cos();
        self.b2 = (1.0 - w0.cos()) / 2.0;
        self.a0 = 1.0 + alpha;
        self.a1 = -2.0 * w0.cos();
        self.a2 = 1.0 - alpha;
    }

    pub fn render(&mut self, input_sample: f32) -> f32 {
        let yn = (self.b0 / self.a0) * input_sample
            + (self.b1 / self.a0) * self.x1
            + (self.b2 / self.a0) * self.x2
            - (self.a1 / self.a0) * self.y1
            - (self.a2 / self.a0) * self.y2;

        self.x2 = self.x1;
        self.x1 = input_sample;

        self.y2 = self.y1;
        self.y1 = yn;

        yn
    }
}

pub struct FirLowPassFilter {
    sample_rate: f32,
    s1: f32,
    s2: f32,
}

impl FirLowPassFilter {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            sample_rate,
            s1: 0.0,
            s2: 0.0,
        }
    }

    // Difference Equation: y[n] = a0.x[n] + a1.x[n-1]
    pub fn render(&mut self, input_sample: f32) -> f32 {
        let y = self.s1 + self.s2 + input_sample;

        //self.s2 = self.s1; // Playing with additional state delays
        self.s1 = input_sample;

        y
    }
}

// JUCE implementation of juce_StateVariableTPTFilter
pub struct StateVariableTPTFilter {
    pub sample_rate: f32,
    filter_type: FilterType,
    // Coefficients
    g: f32,
    h: f32,
    r2: f32,
    // Intermediate State ?
    s1: f32,
    s2: f32,
}

impl StateVariableTPTFilter {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            sample_rate,
            filter_type: FilterType::BandPass,
            g: 0.0,
            h: 0.0,
            r2: 0.0,
            s1: 0.0,
            s2: 0.0,
        }
    }

    pub fn update_coefficients(&mut self, cutoff_freq: f32, resonance: f32) {
        self.g = (std::f32::consts::PI * cutoff_freq / self.sample_rate).tan();
        self.r2 = 1.0 / resonance;
        self.h = 1.0 / (1.0 + self.r2 * self.g + self.g * self.g);
    }

    pub fn render(&mut self, input_sample: f32) -> f32 {
        let y_high_pass = self.h * (input_sample - self.s1 * (self.g + self.r2) - self.s2);
        let y_band_pass = y_high_pass * self.g + self.s1;
        self.s1 = y_high_pass * self.g + y_band_pass;

        let y_low_pass = y_band_pass * self.g + self.s2;
        self.s2 = y_band_pass * self.g + y_low_pass;

        match self.filter_type {
            FilterType::LowPass => return y_low_pass,
            FilterType::BandPass => return y_band_pass,
            FilterType::HighPass => return y_high_pass,
        }
    }
}

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
