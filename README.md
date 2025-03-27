# Shezem-rs
## About
A Rust implementation of a fast audio fingerprinting system inspired by Shazam, for audio recognition and identification. It focuses on speed, efficiency and simplicity. 

## How it works
**Comming soon...**

## Usage
### Build
```bash
# Clone the repository
git clone https://github.com/username/shezem-rs.git
cd shezem-rs

# Build the project
cargo build --release

# The executable will be available at
# ./target/release/shezem-rs
```

The CLI provides two main commands: `index` and `search`

### Indexing Audio Files

To create an index of audio files in a directory:

```bash
shezem-rs index /path/to/audio/folder
```

This will create a `.db` folder in the specified directory and store the database file (`db.db3`) inside it.

### Searching for Similar Audio

To find similar audio files to a query file:

```bash
shezem-rs search /path/to/query.mp3 --path /path/to/indexed/folder
```

By default, this will return the top 10 matches. You can change the number of results with the `--rank` option:

```bash
shezem-rs search /path/to/query.mp3 --path /path/to/indexed/folder --rank 5
```
## Performance 
Performance benchmarks were conducted on a collection of 100 songs totaling approximately 1.1GB, using an AMD Ryzen 5 5600H (12) @ 4.28 GHz processor:

- **Indexing Speed**: Complete folder indexing was accomplished in 35.5 seconds
- **Search Performance**: 
  - 10-second audio sample search: 0.3 seconds
  - 3-minute audio sample search: 1.02 seconds
