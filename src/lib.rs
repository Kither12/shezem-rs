use std::{collections::VecDeque, fs, path::PathBuf};

use anyhow::{Ok, Result};
use db::{DbClient, FingerprintData, SongData};
use sample::Sample;
use spectrogram::{Peak, filter_spectrogram, generate_spectrogram};

pub mod db;
pub mod sample;
pub mod spectrogram;

const NEIGHBORHOOD_SIZE: usize = 5;

pub struct Fingerprint {
    address: u32,
    anchor_time: u32,
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
        for j in i..(i + NEIGHBORHOOD_SIZE) {
            let delta_time = (peaks[j].time - peaks[i].time) * 1000.0;
            /*
                9 bits for storing peaks[i].freq
                9 bits for storing peaks[j].freq
                14 bits for storing delta_time
            */
            let address = (peaks[i].freq << 23) | (peaks[j].freq << 14) | delta_time as u32;
            addresses.push(Fingerprint {
                address,
                anchor_time: (peaks[i].time * 1000.0) as u32,
            });
        }
    }
    addresses
}

pub fn index_folder(path: &PathBuf, database_path: &PathBuf) -> Result<()> {
    let entries: Vec<_> = fs::read_dir(path)?.collect::<Result<_, _>>()?;
    //this stack hold the song ids
    let mut stack = VecDeque::new();

    //First we insert the song information into the song database
    {
        let db_client = DbClient::new(database_path);
        for entry in &entries {
            if entry.path().extension().and_then(|e| e.to_str()) != Some("mp3") {
                continue;
            }

            let title = entry
                .path()
                .file_stem()
                .unwrap()
                .to_str()
                .unwrap()
                .to_string();
            let song_id = db_client.register_song(&SongData { title })?;
            stack.push_back(song_id);
        }
    }

    //Then we bulk insert all the fingerprints into the fingerprint database
    {
        let mut db_client = DbClient::new(database_path);
        let mut tx = db_client.get_conn();
        for entry in entries {
            if entry.path().extension().and_then(|e| e.to_str()) != Some("mp3") {
                continue;
            }

            let mut sample = Sample::read_mp3(&entry.path())?;
            sample = sample.downsample(4);
            let mut spectrogram =
                generate_spectrogram(&sample.sample, spectrogram::WindowSize::S1024, 512);
            let peaks = filter_spectrogram(&mut spectrogram, sample.sample_rate);
            let fingerprints = generate_fingerprint(peaks);

            let song_id = stack.pop_front().unwrap();
            for fingerprint in fingerprints {
                DbClient::register_fingerprint(
                    &FingerprintData {
                        address: fingerprint.address,
                        anchor_time: fingerprint.anchor_time,
                        song_id,
                    },
                    &mut tx,
                )?;
            }
        }
        tx.commit()?;
    }
    Ok(())
}

pub fn search(query_file: &PathBuf, database_path: &PathBuf) -> Result<()> {
    let db_client = DbClient::new(database_path);

    let mut sample = Sample::read_mp3(query_file)?;

    sample = sample.downsample(4);

    let mut spectrogram = generate_spectrogram(&sample.sample, spectrogram::WindowSize::S512, 256);
    let peaks = filter_spectrogram(&mut spectrogram, sample.sample_rate);

    let fingerprints = generate_fingerprint(peaks);
    let addresses = fingerprints.iter().map(|f| f.address).collect::<Vec<_>>();

    db_client.search(addresses)?;
    Ok(())
}
