use std::{cmp::max, collections::HashMap, path::PathBuf};

use anyhow::Result;
use rusqlite::{Connection, Transaction, params};

use crate::{
    NEIGHBORHOOD_SIZE,
    fingerprint::{Fingerprint, FingerprintData},
    utils::longest_increasing_subsequence,
};

#[derive(Debug)]
pub struct SongData {
    // Only title for now
    pub title: String,
}

#[derive(Debug)]
pub struct RankingData {
    pub data: SongData,
    pub score: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Couples {
    pub anchor_address: u32,
    pub anchor_time: u32,
    pub song_id: i32,
}

pub struct DbClient {
    conn: Connection,
}

impl DbClient {
    pub fn new(path: &PathBuf) -> Self {
        let conn = Connection::open(path).unwrap();
        let client = DbClient { conn };
        client.create_tables().unwrap();
        client
    }
    pub fn get_conn<'a>(&'a mut self) -> Transaction<'a> {
        self.conn.transaction().unwrap()
    }
    pub fn create_tables(&self) -> rusqlite::Result<()> {
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS songs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                title TEXT NOT NULL
            )",
            [],
        )?;

        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS fingerprints (
                address INTEGER NOT NULL,
                anchorAddress INTEGER NOT NULL,
                anchorTime INTEGER NOT NULL,
                songID INTEGER NOT NULL,
                PRIMARY KEY (address, anchorAddress, anchorTime, songID)
            )",
            [],
        )?;

        Ok(())
    }

    pub fn register_song(&self, song_data: &SongData) -> Result<i64> {
        let mut stmt = self
            .conn
            .prepare_cached("INSERT INTO songs (title) VALUES (?)")?;
        let result = stmt.execute([&song_data.title])?;

        if result == 0 {
            return Err(rusqlite::Error::StatementChangedRows(0).into());
        }

        let song_id = self.conn.last_insert_rowid();
        Ok(song_id)
    }

    pub fn register_fingerprint<'a>(
        fingerprint_data: &FingerprintData,
        tx: &mut Transaction<'a>,
    ) -> rusqlite::Result<()> {
        let mut stmt = tx.prepare_cached(
            "INSERT OR IGNORE INTO fingerprints (address, anchorAddress, anchorTime, songID) VALUES (?, ?, ?, ?)",
        )?;
        stmt.execute(params![
            &fingerprint_data.fingerprint.address,
            &fingerprint_data.fingerprint.anchor_address,
            &fingerprint_data.fingerprint.anchor_time,
            &fingerprint_data.song_id,
        ])?;

        Ok(())
    }

    pub fn get_song_data(&self, song_id: i32) -> Result<SongData> {
        let mut stmt = self
            .conn
            .prepare_cached("SELECT title FROM songs WHERE id = ?")?;
        let row = stmt.query_row([song_id], |row| Ok(SongData { title: row.get(0)? }))?;

        Ok(row)
    }

    pub fn search(&self, fingerprints: Vec<Fingerprint>, rank: usize) -> Result<Vec<RankingData>> {
        let placeholders: String = std::iter::repeat("?")
            .take(fingerprints.len())
            .collect::<Vec<_>>()
            .join(",");

        let params: Vec<&dyn rusqlite::types::ToSql> = fingerprints
            .iter()
            .map(|f| &f.address as &dyn rusqlite::types::ToSql)
            .collect();

        let mut stmt = self.conn.prepare(&format!(
            "SELECT address, anchorAddress, anchorTime, songID FROM fingerprints WHERE address IN ({})",
            placeholders
        ))?;

        let rows = stmt.query_map(params.as_slice(), |row| {
            Ok(FingerprintData {
                fingerprint: Fingerprint {
                    address: row.get(0)?,
                    anchor_address: row.get(1)?,
                    anchor_time: row.get(2)?,
                },
                song_id: row.get(3)?,
            })
        })?;

        /*
            I know I used lots of Hashmap here :DD
        */

        let mut freq_map = HashMap::with_capacity(fingerprints.len());
        let mut fingerprint_mp = HashMap::with_capacity(fingerprints.len() / NEIGHBORHOOD_SIZE);

        for row_result in rows {
            let row = row_result?;
            let key = Couples {
                anchor_address: row.fingerprint.anchor_address,
                anchor_time: row.fingerprint.anchor_time,
                song_id: row.song_id,
            };
            let count = freq_map.entry(key).or_insert(0);
            *count += 1;

            if *count == NEIGHBORHOOD_SIZE {
                fingerprint_mp
                    .entry(row.song_id)
                    .or_insert_with(Vec::new)
                    .push(row.fingerprint);
            }
        }

        let mut time_mp = HashMap::with_capacity(fingerprints.len());
        for (i, fingerprint) in fingerprints.iter().enumerate() {
            time_mp.insert(fingerprint.address, i);
        }

        /*
            After get the fingerprints for each song from database, we need to verify their temporal coherence with the sample.

            We'll implement a Longest Increasing Subsequence (LIS) algorithm on each song's fingerprint to ensure the chronological
            order of peaks matches our sample's pattern.

            To guard against false positives from random matching peaks, we'll employ a sliding window technique. This window,
            sized to match our sample duration, will move across the LIS results. The maximum number of matching peaks detected
            within any window position will serve as our relevance score for that song.
        */

        let delta =
            fingerprints.last().unwrap().anchor_time - fingerprints.first().unwrap().anchor_time;
        let mut result = Vec::with_capacity(fingerprint_mp.len());
        for (song_id, mut fingerprints) in fingerprint_mp {
            fingerprints.sort_unstable_by_key(|fp| time_mp[&fp.address]);
            let times = fingerprints
                .iter()
                .map(|v| v.anchor_time)
                .collect::<Box<[u32]>>();
            let lis = longest_increasing_subsequence(&times);

            let mut l: usize = 0;
            let mut score = 0;
            for r in 0..lis.len() {
                while lis[r] - lis[l] > delta {
                    l += 1;
                }
                score = max(score, r - l + 1);
            }

            result.push((song_id, score as i32));
        }

        result.sort_by(|a, b| b.1.cmp(&a.1));
        result.truncate(rank);

        let mut result_vec = Vec::with_capacity(result.len());
        for (song_id, score) in result {
            match self.get_song_data(song_id) {
                Ok(data) => result_vec.push(RankingData { data, score }),
                Err(_) => continue,
            }
        }
        Ok(result_vec)
    }
}
