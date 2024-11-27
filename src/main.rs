#![warn(clippy::all, rust_2018_idioms)]
//#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on
// Windows in release mode

use atomic_float::AtomicF32;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::atomic::Ordering;
use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex};

mod app;
use filters::{StateVariableFilter, StateVariableTPTFilter};

use crate::app::{AudioCommand, AudioFilterApp};

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

    let svf = Arc::new(Mutex::new(StateVariableFilter::new(44100.0)));
    {
        let mut f = svf.lock().unwrap();
        f.reset();
    }

    // I don't want to store everything in a mutex, so at least for now keep the volume as an Atomic and pass the filter behind
    // an Arc<Mutex>
    let volume = std::sync::Arc::new(AtomicF32::new(0.3));
    let mut cutoff_freq_hz = 1000.0;
    let mut resonance_q = 0.707;
    let (ui_tx, ui_rx) = channel::<crate::app::AudioCommand>();

    let _audio_thread = std::thread::spawn(move || {
        let volume_clone = volume.clone();
        let filter = svf.clone();

        let host = cpal::default_host();

        let device = host
            .default_output_device()
            .expect("Failed to get default output device");
        let config = device
            .default_output_config()
            .expect("Failed to get default device config");

        let stream = match config.sample_format() {
            cpal::SampleFormat::I8 => {
                make_stream::<i8>(&device, &config.into(), volume_clone, filter)
            }
            cpal::SampleFormat::I16 => {
                make_stream::<i16>(&device, &config.into(), volume_clone, filter)
            }
            cpal::SampleFormat::I32 => {
                make_stream::<i32>(&device, &config.into(), volume_clone, filter)
            }
            cpal::SampleFormat::I64 => {
                make_stream::<i64>(&device, &config.into(), volume_clone, filter)
            }
            cpal::SampleFormat::U8 => {
                make_stream::<u8>(&device, &config.into(), volume_clone, filter)
            }
            cpal::SampleFormat::U16 => {
                make_stream::<u16>(&device, &config.into(), volume_clone, filter)
            }
            cpal::SampleFormat::U32 => {
                make_stream::<u32>(&device, &config.into(), volume_clone, filter)
            }
            cpal::SampleFormat::U64 => {
                make_stream::<u64>(&device, &config.into(), volume_clone, filter)
            }
            cpal::SampleFormat::F32 => {
                make_stream::<f32>(&device, &config.into(), volume_clone, filter)
            }
            cpal::SampleFormat::F64 => {
                make_stream::<f64>(&device, &config.into(), volume_clone, filter)
            }
            _sample_format => panic!("Unexpected sample format!!!"),
        };

        // If the stream is disposed of, audio is DONE. The stream needs to be alive for the duration of the app.
        // This is okay because the thread is looping forever to try and read commands from the UI, so it'll never stop.
        // I wonder if the stream could instead be stored in the App's state...?
        stream.play().unwrap();

        // make updates
        loop {
            match ui_rx.try_recv() {
                Ok(cmd) => match cmd {
                    AudioCommand::SetVolume(new_vol) => {
                        volume.store(new_vol, Ordering::Relaxed);
                    }
                    AudioCommand::SetFilterFreq(cuttoff_freq_hz_new) => {
                        cutoff_freq_hz = cuttoff_freq_hz_new;
                        {
                            let mut filter = svf.lock().unwrap();
                            filter.update_coefficients(cutoff_freq_hz, resonance_q);
                        }
                    }
                    AudioCommand::SetResonance(resonance_q_new) => {
                        resonance_q = resonance_q_new;
                        {
                            let mut filter = svf.lock().unwrap();
                            filter.update_coefficients(cutoff_freq_hz, resonance_q);
                        }
                    }
                },
                Err(_) => (),
            }
        }
    }); // Audio Thread End

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([800.0, 600.0])
            .with_min_inner_size([300.0, 220.0]),
        ..Default::default()
    };

    let mut app = AudioFilterApp::new();
    app.audio_tx = Some(ui_tx);

    eframe::run_native(
        "Audio Filters",
        native_options,
        Box::new(|_| Ok(Box::new(app))),
    )
}

fn make_stream<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    volume: Arc<AtomicF32>,
    filter: Arc<Mutex<StateVariableFilter>>,
) -> cpal::Stream
where
    T: cpal::SizedSample + cpal::FromSample<f32>,
{
    let num_channels = config.channels as usize;
    let mut noise_gen = NoiseGen::new();

    let err_fn = |err| eprintln!("Error building output sound stream {}", err);

    let stream = device
        .build_output_stream(
            config,
            move |output: &mut [T], _: &cpal::OutputCallbackInfo| {
                // process_frame(output, &mut noise_gen, &mut svf, vol, num_channels)
                process_frame(
                    output,
                    &mut noise_gen,
                    num_channels,
                    volume.clone(),
                    filter.clone(),
                )
            },
            err_fn,
            None,
        )
        .expect("Failed to build output stream...");

    stream
}

fn process_frame<SampleType>(
    output: &mut [SampleType],
    noise_gen: &mut NoiseGen,
    num_channels: usize,
    volume: Arc<AtomicF32>,
    filter: Arc<Mutex<StateVariableFilter>>,
) where
    SampleType: cpal::Sample + cpal::FromSample<f32>,
{
    let volume = volume.load(Ordering::Relaxed);

    for frame in output.chunks_mut(num_channels) {
        let next_noise = noise_gen.next_value();

        // TODO: Not great...  I don't like locks in the audio thread as they're unbounded.
        // Lock for the shortest span possible... just to render the filter's output.
        let filter_sample: f32;
        {
            let mut svf = filter.lock().unwrap();
            filter_sample = svf.render(next_noise);
        }
        let next_sample = filter_sample * volume;
        let value: SampleType = SampleType::from_sample(next_sample);

        for sample in frame.iter_mut() {
            *sample = value;
        }
    }
}
