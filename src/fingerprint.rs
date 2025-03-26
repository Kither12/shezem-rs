use crate::{NEIGHBORHOOD_SIZE, spectrogram::Peak};

#[derive(Debug)]
pub struct FingerprintData {
    pub fingerprint: Fingerprint,
    pub song_id: i32,
}

#[derive(Debug)]
pub struct Fingerprint {
    pub address: u32,
    pub anchor_address: u32,
    pub anchor_time: u32,
}

pub fn generate_fingerprint(mut peaks: Vec<Peak>) -> Vec<Fingerprint> {
    peaks.sort_by(|a, b| {
        a.time
            .partial_cmp(&b.time)
            .unwrap()
            .then(a.freq.partial_cmp(&b.freq).unwrap())
    });

    let mut addresses = Vec::new();
    for i in 0..(peaks.len() - NEIGHBORHOOD_SIZE) {
        let anchor_address = build_address(&peaks[i], &peaks[i + NEIGHBORHOOD_SIZE]);
        for j in i..(i + NEIGHBORHOOD_SIZE) {
            let address = build_address(&peaks[i], &peaks[j]);
            addresses.push(Fingerprint {
                address,
                anchor_address,
                anchor_time: (peaks[i].time * 1000.0) as u32,
            });
        }
    }
    addresses
}

pub fn build_address(peak_a: &Peak, peak_b: &Peak) -> u32 {
    let delta_time = (peak_b.time - peak_a.time) * 1000.0;
    /*
        9 bits for storing peak_a.freq
        9 bits for storing peak_b.freq
        14 bits for storing delta_time
    */
    (peak_a.freq << 23) | (peak_b.freq << 14) | delta_time as u32
}
