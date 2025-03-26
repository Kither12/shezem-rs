use std::{collections::VecDeque, fs, path::PathBuf};

use anyhow::{Ok, Result};
use db::{DbClient, SongData};
use fingerprint::{FingerprintData, generate_fingerprint};
use sample::Sample;
use spectrogram::{filter_spectrogram, generate_spectrogram};

pub mod db;
pub mod fingerprint;
pub mod sample;
pub mod spectrogram;
pub mod utils;

const NEIGHBORHOOD_SIZE: usize = 5;

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
            stack.push_back(song_id as i32);
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
                        fingerprint,
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

pub fn search(query_file: &PathBuf, database_path: &PathBuf, rank: usize) -> Result<()> {
    let db_client = DbClient::new(database_path);

    let mut sample = Sample::read_mp3(query_file)?;
    sample = sample.downsample(4);

    let mut spectrogram = generate_spectrogram(&sample.sample, spectrogram::WindowSize::S1024, 512);
    let peaks = filter_spectrogram(&mut spectrogram, sample.sample_rate);

    let fingerprints = generate_fingerprint(peaks);

    let ranking = db_client.search(fingerprints, rank)?;
    for (index, data) in ranking.iter().enumerate() {
        println!("{}. {} (score: {})", index + 1, data.data.title, data.score);
    }
    Ok(())
}
