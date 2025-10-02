// Audio encoding for fractal mind training
// Converts audio streams into string representations that hash to Mandelbrot coordinates

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::Stream;
use rustfft::{FftPlanner, num_complex::Complex};
use std::sync::{Arc, Mutex};

/// Encoding strategy for audio input
#[derive(Clone, Copy)]
pub enum AudioEncoding {
    /// FFT frequency bins as comma-separated values (e.g., "0.2,0.8,0.3,0.1,...")
    FrequencyBins { bin_count: usize },

    /// Symbolic representation of frequency bands (e.g., "low:##,mid:--,high:.|")
    BandSymbols { bands: usize },

    /// Onset/beat detection events (e.g., "kick,snare,kick,hat")
    OnsetEvents,

    /// Amplitude envelope over time (e.g., "amp:20,45,80,120,95,...")
    AmplitudeEnvelope { sample_rate_ms: usize },

    /// Pitch estimation (e.g., "pitch:A440,pitch:C523")
    PitchDetection,
}

/// Captures audio and encodes it as strings
pub struct AudioEncoder {
    encoding: AudioEncoding,
    sample_rate: u32,
    buffer: Arc<Mutex<Vec<f32>>>,
    _stream: Stream,
}

impl AudioEncoder {
    /// Initialize audio encoder with specified encoding strategy
    pub fn new(encoding: AudioEncoding) -> Result<Self, Box<dyn std::error::Error>> {
        let host = cpal::default_host();
        let device = host.default_input_device()
            .ok_or("no input device available")?;

        let config = device.default_input_config()?;
        let sample_rate = config.sample_rate().0;

        let buffer = Arc::new(Mutex::new(Vec::new()));
        let buffer_clone = Arc::clone(&buffer);

        let stream = device.build_input_stream(
            &config.into(),
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                let mut buf = buffer_clone.lock().unwrap();
                buf.extend_from_slice(data);

                // Keep buffer from growing unbounded (1 second max)
                let max_len = sample_rate as usize;
                if buf.len() > max_len {
                    let drain_count = buf.len() - max_len;
                    buf.drain(0..drain_count);
                }
            },
            |err| eprintln!("audio stream error: {}", err),
            None,
        )?;

        stream.play()?;

        Ok(AudioEncoder {
            encoding,
            sample_rate,
            buffer,
            _stream: stream,
        })
    }

    /// Encode current audio buffer to string
    pub fn encode_current(&self) -> String {
        let samples = {
            let buf = self.buffer.lock().unwrap();
            buf.clone()
        };

        if samples.is_empty() {
            return "audio:silence".to_string();
        }

        match self.encoding {
            AudioEncoding::FrequencyBins { bin_count } => {
                encode_fft_bins(&samples, bin_count)
            }
            AudioEncoding::BandSymbols { bands } => {
                encode_band_symbols(&samples, bands)
            }
            AudioEncoding::OnsetEvents => {
                encode_onset_events(&samples)
            }
            AudioEncoding::AmplitudeEnvelope { sample_rate_ms } => {
                encode_amplitude_envelope(&samples, self.sample_rate, sample_rate_ms)
            }
            AudioEncoding::PitchDetection => {
                encode_pitch(&samples, self.sample_rate)
            }
        }
    }
}

/// Perform FFT and encode frequency bins
fn encode_fft_bins(samples: &[f32], bin_count: usize) -> String {
    let mut planner = FftPlanner::new();
    let fft_size = samples.len().min(2048).next_power_of_two();
    let fft = planner.plan_fft_forward(fft_size);

    let mut buffer: Vec<Complex<f32>> = samples
        .iter()
        .take(fft_size)
        .map(|&s| Complex::new(s, 0.0))
        .collect();

    // Pad if needed
    while buffer.len() < fft_size {
        buffer.push(Complex::new(0.0, 0.0));
    }

    fft.process(&mut buffer);

    let mut output = String::from("fft:");
    let step = fft_size / 2 / bin_count;

    for i in 0..bin_count {
        let idx = (i * step).min(fft_size / 2 - 1);
        let magnitude = (buffer[idx].re.powi(2) + buffer[idx].im.powi(2)).sqrt();
        output.push_str(&format!("{:.2},", magnitude));
    }

    output
}

/// Encode frequency bands as symbols
fn encode_band_symbols(samples: &[f32], band_count: usize) -> String {
    let mut planner = FftPlanner::new();
    let fft_size = samples.len().min(2048).next_power_of_two();
    let fft = planner.plan_fft_forward(fft_size);

    let mut buffer: Vec<Complex<f32>> = samples
        .iter()
        .take(fft_size)
        .map(|&s| Complex::new(s, 0.0))
        .collect();

    while buffer.len() < fft_size {
        buffer.push(Complex::new(0.0, 0.0));
    }

    fft.process(&mut buffer);

    let mut output = String::from("bands:");
    let bins_per_band = (fft_size / 2) / band_count;

    for band in 0..band_count {
        let start = band * bins_per_band;
        let end = ((band + 1) * bins_per_band).min(fft_size / 2);

        let mut energy = 0.0;
        for i in start..end {
            energy += buffer[i].re.powi(2) + buffer[i].im.powi(2);
        }
        energy = (energy / (end - start) as f32).sqrt();

        let symbol = match (energy * 100.0) as u32 {
            0..=5 => '.',
            6..=15 => '-',
            16..=30 => '|',
            31..=50 => '#',
            _ => '@',
        };
        output.push(symbol);
    }

    output
}

/// Detect onsets/beats using amplitude spikes
fn encode_onset_events(samples: &[f32]) -> String {
    let window_size = 512;
    let mut output = String::from("onset:");

    if samples.len() < window_size * 2 {
        return output;
    }

    let mut prev_energy = 0.0;

    for chunk in samples.chunks(window_size) {
        let energy: f32 = chunk.iter().map(|&s| s.abs()).sum();
        let energy = energy / chunk.len() as f32;

        if energy > prev_energy * 1.5 && energy > 0.1 {
            // Onset detected
            let intensity = match (energy * 100.0) as u32 {
                0..=10 => "soft",
                11..=30 => "med",
                _ => "hard",
            };
            output.push_str(&format!("{},", intensity));
        }

        prev_energy = energy;
    }

    output
}

/// Encode amplitude envelope at specified sample rate
fn encode_amplitude_envelope(samples: &[f32], sample_rate: u32, sample_rate_ms: usize) -> String {
    let samples_per_step = (sample_rate as usize * sample_rate_ms) / 1000;

    let mut output = String::from("amp:");

    for chunk in samples.chunks(samples_per_step) {
        let avg_amplitude: f32 = chunk.iter().map(|&s| s.abs()).sum::<f32>() / chunk.len() as f32;
        let scaled = (avg_amplitude * 1000.0) as u32;
        output.push_str(&format!("{},", scaled));
    }

    output
}

/// Basic pitch detection using zero-crossing rate and autocorrelation
fn encode_pitch(samples: &[f32], sample_rate: u32) -> String {
    if samples.len() < 1024 {
        return "pitch:none".to_string();
    }

    // Simple autocorrelation-based pitch detection
    let window = &samples[samples.len() - 1024..];

    let mut max_corr = 0.0;
    let mut best_lag = 0;

    for lag in 20..500 {
        let mut corr = 0.0;
        for i in 0..(window.len() - lag) {
            corr += window[i] * window[i + lag];
        }
        if corr > max_corr {
            max_corr = corr;
            best_lag = lag;
        }
    }

    if max_corr > 0.1 {
        let frequency = sample_rate as f32 / best_lag as f32;
        format!("pitch:{:.1}Hz", frequency)
    } else {
        "pitch:none".to_string()
    }
}
