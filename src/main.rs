use anyhow::Result;
use clap::{Parser, Subcommand};
use shezem_rs::{index_folder, search};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "shezem-rs",
    about = "Index and retrieve audio files",
    version = "0.0.1",
    author = "Kither"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Index {
        #[arg(value_name = "PATH")]
        path: PathBuf,
    },

    Search {
        #[arg(value_name = "AUDIO_FILE")]
        query_file: PathBuf,

        #[arg(short, long, value_name = "DB_PATH")]
        path: PathBuf,

        #[arg(short, long, default_value = "10")]
        rank: usize,
    },
}

const DEFAULT_DB_PATH: &str = "db.db3";
const DEFAULT_FOLDER_DB_PATH: &str = ".db";

fn main() -> Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Index { path } => {
            let db_folder_path = path.join(DEFAULT_FOLDER_DB_PATH);
            if !db_folder_path.exists() {
                std::fs::create_dir_all(&db_folder_path)?;
            }

            let default_db_path = db_folder_path.join(DEFAULT_DB_PATH);
            index_folder(path, &default_db_path)?;
            Ok(())
        }

        Commands::Search {
            query_file,
            path,
            rank,
        } => {
            let default_db_path = path.join(DEFAULT_FOLDER_DB_PATH).join(DEFAULT_DB_PATH);
            search(query_file, &default_db_path, *rank)?;
            Ok(())
        }
    }
}
