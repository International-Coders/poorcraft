use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// One compressed chunk entry inside a region file.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SavedChunk {
    pub x: i32,
    pub z: i32,
    pub compressed_data: Vec<u8>,
}

/// A region file holds all chunks with coords in [rx*32, rx*32+32) x [rz*32, rz*32+32).
#[derive(Serialize, Deserialize, Debug, Default)]
struct Region {
    chunks: Vec<SavedChunk>,
}

/// Regional chunk storage: one file per 32x32 chunk region, each holding
/// every saved chunk keyed by (x, z). Writes are atomic (tmp + rename).
pub struct RegionStorage {
    dir: PathBuf,
}

impl RegionStorage {
    pub fn new(dir: PathBuf) -> Self {
        std::fs::create_dir_all(&dir).ok();
        Self { dir }
    }

    fn region_path(&self, rx: i32, rz: i32) -> PathBuf {
        self.dir.join(format!("region_{}_{}.dat", rx, rz))
    }

    fn load_region(&self, rx: i32, rz: i32) -> Region {
        let path = self.region_path(rx, rz);
        if let Ok(encoded) = std::fs::read(&path) {
            if let Ok(region) = bincode::deserialize::<Region>(&encoded) {
                return region;
            }
        }
        Region::default()
    }

    fn store_region(&self, rx: i32, rz: i32, region: &Region) -> Result<(), Box<dyn std::error::Error>> {
        let encoded = bincode::serialize(region)?;
        let path = self.region_path(rx, rz);
        let tmp = path.with_extension("dat.tmp");
        std::fs::write(&tmp, &encoded)?;
        std::fs::rename(&tmp, &path)?;
        Ok(())
    }

    /// Save one chunk's raw block data, zstd-compressed, into its region file.
    pub fn save(&self, x: i32, z: i32, raw_blocks: &[u8]) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
        let (rx, rz) = (x >> 5, z >> 5);
        let mut region = self.load_region(rx, rz);
        let compressed = zstd::encode_all(raw_blocks, 3)?;
        match region.chunks.iter_mut().find(|c| c.x == x && c.z == z) {
            Some(existing) => existing.compressed_data = compressed,
            None => region.chunks.push(SavedChunk { x, z, compressed_data: compressed }),
        }
        self.store_region(rx, rz, &region)?;
        Ok(self.region_path(rx, rz))
    }

    /// Load one chunk's raw block data. Errors if the chunk was never saved.
    pub fn load(&self, x: i32, z: i32) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let (rx, rz) = (x >> 5, z >> 5);
        let region = self.load_region(rx, rz);
        let chunk = region
            .chunks
            .iter()
            .find(|c| c.x == x && c.z == z)
            .ok_or_else(|| format!("chunk ({}, {}) not found in region ({}, {})", x, z, rx, rz))?;
        Ok(zstd::decode_all(&chunk.compressed_data[..])?)
    }

    /// All saved chunk coords, for world listing / load-on-start.
    pub fn list_chunks(&self) -> Vec<(i32, i32)> {
        let mut out = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&self.dir) {
            for entry in entries.flatten() {
                if let Ok(encoded) = std::fs::read(entry.path()) {
                    if let Ok(region) = bincode::deserialize::<Region>(&encoded) {
                        out.extend(region.chunks.iter().map(|c| (c.x, c.z)));
                    }
                }
            }
        }
        out
    }
}

/// Helper for tests and callers that track chunk dirtiness.
pub type ChunkMap = HashMap<(i32, i32), Vec<u8>>;

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

    #[test]
    fn test_neighbor_chunks_same_region_do_not_collide() {
        let tmp = tempfile::tempdir().unwrap();
        let storage = RegionStorage::new(tmp.path().to_path_buf());
        // (0,0), (1,0), (31,5) all live in region (0,0); the old format
        // overwrote the whole file on each save.
        storage.save(0, 0, &[1, 1, 1]).unwrap();
        storage.save(1, 0, &[2, 2, 2]).unwrap();
        storage.save(31, 5, &[3, 3, 3]).unwrap();
        assert_eq!(storage.load(0, 0).unwrap(), vec![1, 1, 1]);
        assert_eq!(storage.load(1, 0).unwrap(), vec![2, 2, 2]);
        assert_eq!(storage.load(31, 5).unwrap(), vec![3, 3, 3]);
        assert_eq!(storage.list_chunks().len(), 3);
    }

    #[test]
    fn test_resave_overwrites_same_chunk() {
        let tmp = tempfile::tempdir().unwrap();
        let storage = RegionStorage::new(tmp.path().to_path_buf());
        storage.save(4, 4, &[1]).unwrap();
        storage.save(4, 4, &[9, 9]).unwrap();
        assert_eq!(storage.load(4, 4).unwrap(), vec![9, 9]);
        assert_eq!(storage.list_chunks(), vec![(4, 4)]);
    }

    #[test]
    fn test_negative_coordinates() {
        let tmp = tempfile::tempdir().unwrap();
        let storage = RegionStorage::new(tmp.path().to_path_buf());
        // (-1,-1) is region (-1,-1); (-33, 0) is region (-2, 0).
        storage.save(-1, -1, &[7]).unwrap();
        storage.save(-33, 0, &[8]).unwrap();
        assert_eq!(storage.load(-1, -1).unwrap(), vec![7]);
        assert_eq!(storage.load(-33, 0).unwrap(), vec![8]);
    }

    #[test]
    fn test_load_missing_chunk_errors() {
        let tmp = tempfile::tempdir().unwrap();
        let storage = RegionStorage::new(tmp.path().to_path_buf());
        assert!(storage.load(99, 99).is_err());
    }
}
