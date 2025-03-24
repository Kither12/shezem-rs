use std::{
    fs::File,
    io::{BufReader, Error},
    path::PathBuf,
};

use anyhow::Result;

pub struct Sample {
    pub sample: Vec<f32>,
    pub sample_rate: usize,
}

impl Sample {
    pub fn low_pass_filter(&self, cutoff_freq: f32) -> Sample {
        // IIR low pass filter
        // y[n] = alpha * x[n] + (1.0 - alpha) * y[n-1]

        let fc = cutoff_freq / self.sample_rate as f32;
        let alpha = 2.0 * std::f32::consts::PI * fc / (2.0 * std::f32::consts::PI * fc + 1.0);

        let mut filtered = vec![0.0; self.sample.len()];

        if !self.sample.is_empty() {
            filtered[0] = self.sample[0];
        }

        for i in 1..self.sample.len() {
            filtered[i] = alpha * self.sample[i] + (1.0 - alpha) * filtered[i - 1];
        }

        Sample {
            sample: filtered,
            sample_rate: self.sample_rate,
        }
    }

    pub fn downsample(&mut self, factor: usize) -> Sample {
        /*
            When downsampling by a factor, the Nyquist frequency of the new sample rate
            will be (sample_rate/factor)/2. To prevent aliasing, we need to filter out
            frequencies above this threshold. Using 0.45 instead of 0.5 provides a small
            margin to account for the non-ideal nature of our simple filter.
        */
        let cutoff_freq = (self.sample_rate / factor) as f32 * 0.45;
        let filtered = self.low_pass_filter(cutoff_freq);

        let new_len = filtered.sample.len() / factor;
        let mut downsampled = Vec::with_capacity(new_len);

        for i in (0..filtered.sample.len()).step_by(factor) {
            downsampled.push(filtered.sample[i]);
        }

        Sample {
            sample: downsampled,
            sample_rate: self.sample_rate / factor,
        }
    }

    pub fn read_mp3(path: &PathBuf) -> Result<Self> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let mut decoder = minimp3::Decoder::new(reader);

        let mut mono_samples = Vec::new();
        let mut sampling_rate = 0;

        while let Ok(minimp3::Frame {
            data,
            sample_rate,
            channels,
            ..
        }) = decoder.next_frame()
        {
            if sampling_rate == 0 {
                sampling_rate = sample_rate;
            }

            match channels {
                1 => {
                    mono_samples.extend(data.iter().map(|&s| s as f32));
                }
                2 => {
                    let len = data.len() / 2;
                    mono_samples.reserve(len);

                    for chunk in data.chunks_exact(2) {
                        let avg = (chunk[0] as f32 + chunk[1] as f32) * 0.5;
                        mono_samples.push(avg);
                    }
                }
                _ => panic!("Unsupported number of channels: {}", channels),
            }
        }
        Ok(Sample {
            sample: mono_samples,
            sample_rate: sampling_rate as usize,
        })
    }
}
