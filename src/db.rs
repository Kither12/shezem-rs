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
struct Couples {
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
    fn get_fingerprint_from_database(
        &self,
        fingerprints: &Vec<Fingerprint>,
    ) -> Result<Vec<FingerprintData>> {
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

        let rows = stmt
            .query_map(params.as_slice(), |row| {
                Ok(FingerprintData {
                    fingerprint: Fingerprint {
                        address: row.get(0)?,
                        anchor_address: row.get(1)?,
                        anchor_time: row.get(2)?,
                    },
                    song_id: row.get(3)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(rows)
    }
    pub fn search(&self, fingerprints: Vec<Fingerprint>, rank: usize) -> Result<Vec<RankingData>> {
        let sample_duration =
            fingerprints.last().unwrap().anchor_time - fingerprints.first().unwrap().anchor_time;

        let index_map: HashMap<u32, usize> = fingerprints
            .iter()
            .enumerate()
            .map(|(i, fp)| (fp.address, i))
            .collect();

        let db_result = self.get_fingerprint_from_database(&fingerprints)?;

        // Track fingerprint matches directly by song ID
        let mut song_fingerprints: HashMap<i32, Vec<Fingerprint>> = HashMap::new();

        // Count matches to identify complete neighborhoods
        let mut match_counts: HashMap<Couples, usize> = HashMap::new();

        for row in db_result {
            let key = Couples {
                anchor_address: row.fingerprint.anchor_address,
                anchor_time: row.fingerprint.anchor_time,
                song_id: row.song_id,
            };
            let count = match_counts.entry(key).or_insert(0);
            *count += 1;

            if *count == NEIGHBORHOOD_SIZE {
                song_fingerprints
                    .entry(row.song_id)
                    .or_insert_with(Vec::new)
                    .push(row.fingerprint);
            }
        }

        /*
            After get the fingerprints for each song from database, we need to verify their temporal coherence with the sample.

            We'll implement a Longest Increasing Subsequence (LIS) algorithm on each song's fingerprint to ensure the chronological
            order of peaks matches our sample's pattern.

            To guard against false positives from random matching peaks, we'll employ a sliding window technique. This window,
            sized to match our sample duration, will move across the LIS results. The maximum number of matching peaks detected
            within any window position will serve as our relevance score for that song.
        */

        let mut result = Vec::with_capacity(song_fingerprints.len());
        for (song_id, mut fingerprints) in song_fingerprints {
            fingerprints.sort_unstable_by_key(|fp| index_map[&fp.address]);
            let times = fingerprints
                .iter()
                .map(|v| v.anchor_time)
                .collect::<Box<[u32]>>();
            let lis = longest_increasing_subsequence(&times);

            let mut l: usize = 0;
            let mut score = 0;
            for r in 0..lis.len() {
                while lis[r] - lis[l] > sample_duration {
                    l += 1;
                }
                score = max(score, r - l + 1);
            }

            result.push((song_id, score as i32));
        }

        result.sort_unstable_by_key(|a| -a.1);
        result.truncate(rank);

        Ok(result
            .into_iter()
            .filter_map(|(song_id, score)| {
                self.get_song_data(song_id)
                    .ok()
                    .map(|data| RankingData { data, score })
            })
            .collect::<Vec<_>>())
    }
}
