#![warn(clippy::all, rust_2018_idioms)]
//#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on
                                                                   // Windows in release mode

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

pub struct NoiseGen {
    noise_seed: u32,
}

impl NoiseGen {
    pub fn new() -> Self {
        Self { noise_seed: 22222 }
    }

    pub fn reset(&mut self) {
        self.noise_seed = 22222;
    }

    pub fn next_value(&mut self) -> f32 {
        self.noise_seed = self.noise_seed * 196314165 + 907633515;
        let tmp = ((self.noise_seed >> 7) as i32) - 16777216;
        tmp as f32 / 16777216.0f32
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result {
    env_logger::init();

    let _audio_thread = std::thread::spawn(move || {
        let host = cpal::default_host();

        let device = host.default_output_device().expect("Failed to get default output device");
        let config = device.default_output_config().expect("Failed to get default device config");

        match config.sample_format() {
            cpal::SampleFormat::I8 =>  make_stream::<i8>(&device, &config.into()),
            cpal::SampleFormat::I16 => make_stream::<i16>(&device, &config.into()),
            cpal::SampleFormat::I32 => make_stream::<i32>(&device, &config.into()),
            cpal::SampleFormat::I64 => make_stream::<i64>(&device, &config.into()),
            cpal::SampleFormat::U8 =>  make_stream::<u8>(&device, &config.into()),
            cpal::SampleFormat::U16 => make_stream::<u16>(&device, &config.into()),
            cpal::SampleFormat::U32 => make_stream::<u32>(&device, &config.into()),
            cpal::SampleFormat::U64 => make_stream::<u64>(&device, &config.into()),
            cpal::SampleFormat::F32 => make_stream::<f32>(&device, &config.into()),
            cpal::SampleFormat::F64 => make_stream::<f64>(&device, &config.into()),
            sample_format => panic!("Unexpected sample format!!!"),
        }
    });

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([400.0, 300.0])
            .with_min_inner_size([300.0, 220.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Audio Filters",
        native_options,
        Box::new(|cc| Ok(Box::new(filters::AudioFilterApp::new(cc)))),
    )
}

fn make_stream<T>(device: &cpal::Device, config: &cpal::StreamConfig) 
    where T: cpal::SizedSample + cpal::FromSample<f32> 
{
    let num_channels = config.channels as usize;
    let mut noise_gen = NoiseGen::new();
    let err_fn = |err| eprintln!("Error building output sound stream {}", err);

    let stream = device
        .build_output_stream(
            config,
            move |output: &mut [T], _: &cpal::OutputCallbackInfo| {
                process_frame(output, &mut noise_gen, num_channels)
            },
            err_fn,
            None,
        )
        .expect("Failed to build output stream...");

    stream.play().unwrap();
    std::thread::sleep(std::time::Duration::from_secs(30));
}

fn process_frame<SampleType>(output: &mut [SampleType], noise_gen: &mut NoiseGen, num_channels: usize)
    where SampleType: cpal::Sample + cpal::FromSample<f32> 
{
    for frame in output.chunks_mut(num_channels) {
        let value: SampleType = SampleType::from_sample(noise_gen.next_value());

        for sample in frame.iter_mut() {
            *sample = value;
        }
    }
}
