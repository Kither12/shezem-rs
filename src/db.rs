use std::{collections::HashMap, path::PathBuf};

use rusqlite::{CachedStatement, Connection, Statement, Transaction, params};

use crate::NEIGHBORHOOD_SIZE;

pub struct SongData {
    // Only title for now
    pub title: String,
}

pub struct FingerprintData {
    pub address: u32,
    pub anchor_time: u32,
    pub song_id: i64,
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
                anchorTime INTEGER NOT NULL,
                songID INTEGER NOT NULL,
                PRIMARY KEY (address, anchorTime, songID)
            )",
            [],
        )?;

        Ok(())
    }

    pub fn register_song(&self, song_data: &SongData) -> rusqlite::Result<i64> {
        let mut stmt = self
            .conn
            .prepare_cached("INSERT INTO songs (title) VALUES (?)")?;
        let result = stmt.execute([&song_data.title])?;

        if result == 0 {
            return Err(rusqlite::Error::StatementChangedRows(0));
        }

        let song_id = self.conn.last_insert_rowid();
        Ok(song_id)
    }

    pub fn register_fingerprint<'a>(
        fingerprint_data: &FingerprintData,
        tx: &mut Transaction<'a>,
    ) -> rusqlite::Result<()> {
        let mut stmt = tx.prepare_cached(
            "INSERT OR IGNORE INTO fingerprints (address, anchorTime, songID) VALUES (?, ?, ?)",
        )?;
        stmt.execute(params![
            &fingerprint_data.address,
            &fingerprint_data.anchor_time,
            &fingerprint_data.song_id,
        ])?;

        Ok(())
    }
    pub fn search(&self, addresses: Vec<u32>) -> rusqlite::Result<Vec<FingerprintData>> {
        let placeholders: String = std::iter::repeat("?")
            .take(addresses.len())
            .collect::<Vec<_>>()
            .join(",");

        let params: Vec<&dyn rusqlite::types::ToSql> = addresses
            .iter()
            .map(|addr| addr as &dyn rusqlite::types::ToSql)
            .collect();

        let mut stmt = self.conn.prepare(&format!(
            "SELECT address, anchorTime, songID FROM fingerprints WHERE address IN ({})",
            placeholders
        ))?;

        let rows = stmt.query_map(params.as_slice(), |row| {
            Ok(FingerprintData {
                address: row.get(0)?,
                anchor_time: row.get(1)?,
                song_id: row.get(2)?,
            })
        })?;

        let mut results = Vec::new();
        let mut freq_map = HashMap::new();

        for row_result in rows {
            let row = row_result?;
            let key = (row.song_id, row.anchor_time);

            let count = freq_map.entry(key).or_insert(0);
            *count += 1;

            if *count == NEIGHBORHOOD_SIZE {
                results.push(row);
            }
        }
        Ok(results)
    }
}
