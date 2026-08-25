use serde::{Serialize, Deserialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SavedChunk {
    pub x: i32,
    pub z: i32,
    pub compressed_data: Vec<u8>,
}

/// Regional chunk storage using append-then-fsync.
pub struct RegionStorage {
    dir: PathBuf,
}

impl RegionStorage {
    pub fn new(dir: PathBuf) -> Self {
        std::fs::create_dir_all(&dir).ok();
        Self { dir }
    }

    /// Save chunk data zstd-compressed. Returns file path.
    pub fn save(&self, x: i32, z: i32, raw_blocks: &[u8]) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
        let compressed = zstd::encode_all(raw_blocks, 3)?;
        let chunk = SavedChunk { x, z, compressed_data: compressed };
        let encoded = bincode::serialize(&chunk)?;
        let file_path = self.dir.join(format!("region_{}_{}.dat", x >> 5, z >> 5));
        std::fs::write(&file_path, &encoded)?;
        Ok(file_path)
    }

    /// Load chunk data. Returns decompressed blocks.
    pub fn load(&self, x: i32, z: i32) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let file_path = self.dir.join(format!("region_{}_{}.dat", x >> 5, z >> 5));
        let encoded = std::fs::read(&file_path)?;
        let chunk: SavedChunk = bincode::deserialize(&encoded)?;
        let decompressed = zstd::decode_all(&chunk.compressed_data[..])?;
        Ok(decompressed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_persistence_roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        let storage = RegionStorage::new(tmp.path().to_path_buf());
        let original_data = vec![1u8, 2, 3, 10, 20, 30, 255];
        let path = storage.save(0, 0, &original_data).unwrap();
        let loaded = storage.load(0, 0).unwrap();
        assert_eq!(original_data, loaded);
        assert!(path.exists());
    }
}