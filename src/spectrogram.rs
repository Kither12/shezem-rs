use std::f32::consts::PI;

use microfft::Complex32;

pub fn hamming_window(samples: &[f32]) -> Vec<f32> {
    let mut windowed_samples = Vec::with_capacity(samples.len());
    let n = samples.len() as f32;
    for (i, sample) in samples.iter().enumerate() {
        let multiplier = 0.54 - 0.46 * (2.0 * PI * i as f32 / n).cos();
        windowed_samples.push(multiplier * sample)
    }
    windowed_samples
}

#[derive(Debug, Clone, Copy)]
pub enum WindowSize {
    S2,
    S4,
    S8,
    S16,
    S32,
    S64,
    S128,
    S256,
    S512,
    S1024,
    S2048,
    S4096,
    S8192,
}

impl From<WindowSize> for usize {
    fn from(window_size: WindowSize) -> Self {
        match window_size {
            WindowSize::S2 => 2,
            WindowSize::S4 => 4,
            WindowSize::S8 => 8,
            WindowSize::S16 => 16,
            WindowSize::S32 => 32,
            WindowSize::S64 => 64,
            WindowSize::S128 => 128,
            WindowSize::S256 => 256,
            WindowSize::S512 => 512,
            WindowSize::S1024 => 1024,
            WindowSize::S2048 => 2048,
            WindowSize::S4096 => 4096,
            WindowSize::S8192 => 8192,
        }
    }
}

impl From<WindowSize> for f32 {
    fn from(window_size: WindowSize) -> Self {
        usize::from(window_size) as f32
    }
}

pub fn apply_fft(sample: &[f32], window_size: WindowSize) -> Vec<Complex32> {
    let size: usize = window_size.into();
    if sample.len() != size {
        panic!("Sample length must match window size");
    }

    let result = match window_size {
        WindowSize::S2 => {
            let mut array = [0.0f32; 2];
            array.copy_from_slice(&sample[0..2]);
            let complex_result = microfft::real::rfft_2(&mut array);
            complex_result.to_vec()
        }
        WindowSize::S4 => {
            let mut array = [0.0f32; 4];
            array.copy_from_slice(&sample[0..4]);
            let complex_result = microfft::real::rfft_4(&mut array);
            complex_result.to_vec()
        }
        WindowSize::S8 => {
            let mut array = [0.0f32; 8];
            array.copy_from_slice(&sample[0..8]);
            let complex_result = microfft::real::rfft_8(&mut array);
            complex_result.to_vec()
        }
        WindowSize::S16 => {
            let mut array = [0.0f32; 16];
            array.copy_from_slice(&sample[0..16]);
            let complex_result = microfft::real::rfft_16(&mut array);
            complex_result.to_vec()
        }
        WindowSize::S32 => {
            let mut array = [0.0f32; 32];
            array.copy_from_slice(&sample[0..32]);
            let complex_result = microfft::real::rfft_32(&mut array);
            complex_result.to_vec()
        }
        WindowSize::S64 => {
            let mut array = [0.0f32; 64];
            array.copy_from_slice(&sample[0..64]);
            let complex_result = microfft::real::rfft_64(&mut array);
            complex_result.to_vec()
        }
        WindowSize::S128 => {
            let mut array = [0.0f32; 128];
            array.copy_from_slice(&sample[0..128]);
            let complex_result = microfft::real::rfft_128(&mut array);
            complex_result.to_vec()
        }
        WindowSize::S256 => {
            let mut array = [0.0f32; 256];
            array.copy_from_slice(&sample[0..256]);
            let complex_result = microfft::real::rfft_256(&mut array);
            complex_result.to_vec()
        }
        WindowSize::S512 => {
            let mut array = [0.0f32; 512];
            array.copy_from_slice(&sample[0..512]);
            let complex_result = microfft::real::rfft_512(&mut array);
            complex_result.to_vec()
        }
        WindowSize::S1024 => {
            let mut array = [0.0f32; 1024];
            array.copy_from_slice(&sample[0..1024]);
            let complex_result = microfft::real::rfft_1024(&mut array);
            complex_result.to_vec()
        }
        WindowSize::S2048 => {
            let mut array = [0.0f32; 2048];
            array.copy_from_slice(&sample[0..2048]);
            let complex_result = microfft::real::rfft_2048(&mut array);
            complex_result.to_vec()
        }
        WindowSize::S4096 => {
            let mut array = [0.0f32; 4096];
            array.copy_from_slice(&sample[0..4096]);
            let complex_result = microfft::real::rfft_4096(&mut array);
            complex_result.to_vec()
        }
        WindowSize::S8192 => {
            let mut array = [0.0f32; 8192];
            array.copy_from_slice(&sample[0..8192]);
            let complex_result = microfft::real::rfft_8192(&mut array);
            complex_result.to_vec()
        }
    };

    result
}

pub struct FFTWindow {
    pub start_idx: usize,
    pub data: Vec<Complex32>,
}

pub fn generate_spectrogram(
    sample: &[f32],
    window_size: WindowSize,
    overlap: usize,
) -> Vec<FFTWindow> {
    let w_size = window_size.into();
    if overlap >= w_size {
        panic!("overlap size must less than window size");
    }

    let mut start: usize = 0;

    let mut spectrogram = Vec::new();

    while start < sample.len() {
        if start + w_size > sample.len() {
            start = sample.len() - w_size;
        }
        let window = hamming_window(&sample[start..start + w_size]);
        let fft_res = apply_fft(&window, window_size);

        spectrogram.push(FFTWindow {
            start_idx: start,
            data: fft_res,
        });

        if start + w_size >= sample.len() {
            break;
        }
        start += w_size - overlap;
    }
    spectrogram
}

#[derive(Debug)]
pub struct Peak {
    pub time: f32,
    pub freq: u32,
}

pub fn filter_spectrogram(spectrogram: &mut Vec<FFTWindow>, sample_rate: usize) -> Vec<Peak> {
    let bands = [(0, 10), (10, 20), (20, 40), (40, 80), (80, 160), (160, 511)];

    let mut peaks = Vec::new();

    for window in spectrogram.iter() {
        let mut strongest_bins = Vec::with_capacity(bands.len());

        for &(start, end) in &bands {
            let end = end.min(window.data.len() - 1);

            let mut max_magnitude = 0.0;
            let mut max_bin = start;

            for bin in start..=end {
                if bin < window.data.len() {
                    let magnitude = window.data[bin].norm_sqr();
                    if magnitude > max_magnitude {
                        max_magnitude = magnitude;
                        max_bin = bin;
                    }
                }
            }

            strongest_bins.push((max_bin, window.data[max_bin]));
        }

        let average_magnitude = strongest_bins
            .iter()
            .map(|(_, complex)| complex.norm_sqr())
            .sum::<f32>()
            / strongest_bins.len() as f32;

        let threshold = average_magnitude;

        for (bin_index, complex) in strongest_bins {
            if complex.norm_sqr() > threshold {
                let time_in_seconds = window.start_idx as f32 / sample_rate as f32;
                peaks.push(Peak {
                    time: time_in_seconds,
                    freq: bin_index as u32,
                });
            }
        }
    }

    peaks
}
