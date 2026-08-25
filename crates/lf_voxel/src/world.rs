use std::collections::HashSet;
use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::persistence::RegionStorage;
use crate::registry;
use crate::{BlockState, VoxelSection};
use crate::meshing::{self, MeshData, Vertex};

pub const SECTION_COUNT: usize = 16; // 16 sections of 16 -> world height 256

/// A 16x256x16 chunk column made of 16 vertical sections.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ChunkColumn {
    pub sections: Vec<VoxelSection>,
}

impl ChunkColumn {
    pub fn empty() -> Self {
        Self {
            sections: (0..SECTION_COUNT).map(|_| VoxelSection::new_empty()).collect(),
        }
    }

    /// Block at local coords. x/z must be 0..16; y clamps to the world height.
    pub fn get(&self, x: usize, y: usize, z: usize) -> BlockState {
        if y >= SECTION_COUNT * 16 {
            return BlockState::AIR;
        }
        self.sections[y / 16].get(x, y % 16, z)
    }

    pub fn set(&mut self, x: usize, y: usize, z: usize, block: BlockState) {
        if y < SECTION_COUNT * 16 && x < 16 && z < 16 {
            self.sections[y / 16].set(x, y % 16, z, block);
        }
    }
}

/// A set of chunk columns keyed by chunk coords. Missing chunks read as AIR.
#[derive(Clone, Debug, Default)]
pub struct World {
    pub chunks: HashMap<(i32, i32), ChunkColumn>,
}

impl World {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn chunk(&self, cx: i32, cz: i32) -> Option<&ChunkColumn> {
        self.chunks.get(&(cx, cz))
    }

    pub fn ensure_chunk(&mut self, cx: i32, cz: i32) -> &mut ChunkColumn {
        self.chunks.entry((cx, cz)).or_insert_with(ChunkColumn::empty)
    }

    /// Block at world coords.
    pub fn get_block(&self, x: i32, y: i32, z: i32) -> BlockState {
        if y < 0 || y >= (SECTION_COUNT * 16) as i32 {
            return BlockState::AIR;
        }
        let (cx, lx) = (x.div_euclid(16), x.rem_euclid(16) as usize);
        let (cz, lz) = (z.div_euclid(16), z.rem_euclid(16) as usize);
        match self.chunks.get(&(cx, cz)) {
            Some(col) => col.get(lx, y as usize, lz),
            None => BlockState::AIR,
        }
    }

    /// Set block at world coords; does nothing for missing chunks/out of range.
    /// Returns the chunk coords owning the block (for remeshing).
    pub fn set_block(&mut self, x: i32, y: i32, z: i32, block: BlockState) -> Option<(i32, i32)> {
        if y < 0 || y >= (SECTION_COUNT * 16) as i32 {
            return None;
        }
        let (cx, lx) = (x.div_euclid(16), x.rem_euclid(16) as usize);
        let (cz, lz) = (z.div_euclid(16), z.rem_euclid(16) as usize);
        self.chunks.get_mut(&(cx, cz))?.set(lx, y as usize, lz, block);
        Some((cx, cz))
    }

    /// Solid for physics (uses the block registry: water is not solid).
    pub fn is_solid(&self, x: i32, y: i32, z: i32) -> bool {
        registry::is_solid(self.get_block(x, y, z))
    }

    /// Highest targetable block at a column (world coords), for spawn
    /// placement. Skips water so spawns land on the shore/floor surface.
    pub fn surface_height(&self, x: i32, z: i32) -> i32 {
        for y in (0..(SECTION_COUNT * 16) as i32).rev() {
            if registry::is_targetable(self.get_block(x, y, z)) {
                return y + 1;
            }
        }
        0
    }

    /// Mesh one chunk column into world-space vertices (origin at chunk min).
    pub fn mesh_column(&self, cx: i32, cz: i32, tex_of: &dyn Fn(BlockState) -> u32) -> MeshData {
        let mut vertices: Vec<Vertex> = Vec::new();
        let mut indices: Vec<u32> = Vec::new();
        let col = match self.chunks.get(&(cx, cz)) {
            Some(c) => c,
            None => return MeshData { vertices, indices },
        };
        for (sy, section) in col.sections.iter().enumerate() {
            let neighbor_px = self.chunks.get(&(cx + 1, cz)).map(|c| &c.sections[sy]);
            let neighbor_nx = self.chunks.get(&(cx - 1, cz)).map(|c| &c.sections[sy]);
            let neighbor_pz = self.chunks.get(&(cx, cz + 1)).map(|c| &c.sections[sy]);
            let neighbor_nz = self.chunks.get(&(cx, cz - 1)).map(|c| &c.sections[sy]);
            let neighbor_py = col.sections.get(sy + 1);
            let neighbor_ny = if sy > 0 { col.sections.get(sy - 1) } else { None };
            let mesh = meshing::mesh_section(
                section,
                neighbor_px, neighbor_nx, neighbor_py, neighbor_ny, neighbor_pz, neighbor_nz,
                tex_of,
            );
            let base = vertices.len() as u32;
            let oy = (sy * 16) as f32;
            for v in mesh.vertices {
                vertices.push(Vertex {
                    position: [v.position[0], v.position[1] + oy, v.position[2]],
                    ..v
                });
            }
            indices.extend(mesh.indices.iter().map(|i| i + base));
        }
        // Offset to world space.
        let ox = (cx * 16) as f32;
        let oz = (cz * 16) as f32;
        for v in &mut vertices {
            v.position[0] += ox;
            v.position[2] += oz;
        }
        MeshData { vertices, indices }
    }
}


/// World persistence: chunk columns via RegionStorage plus a small player
/// state blob. A "world" is a directory with region files and player.dat.
pub struct WorldStorage {
    regions: RegionStorage,
    dir: std::path::PathBuf,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PlayerSave {
    pub position: [f32; 3],
    pub yaw: f32,
    pub pitch: f32,
}

impl WorldStorage {
    pub fn open(dir: &Path) -> Self {
        let regions = RegionStorage::new(dir.join("region"));
        Self { regions, dir: dir.to_path_buf() }
    }

    pub fn save_chunk(&self, cx: i32, cz: i32, col: &ChunkColumn) -> Result<(), Box<dyn std::error::Error>> {
        let bytes = bincode::serialize(col)?;
        self.regions.save(cx, cz, &bytes)?;
        Ok(())
    }

    pub fn load_chunk(&self, cx: i32, cz: i32) -> Option<ChunkColumn> {
        let bytes = self.regions.load(cx, cz).ok()?;
        bincode::deserialize(&bytes).ok()
    }

    pub fn saved_chunks(&self) -> HashSet<(i32, i32)> {
        self.regions.list_chunks().into_iter().collect()
    }

    pub fn save_player(&self, player: &PlayerSave) -> Result<(), Box<dyn std::error::Error>> {
        let bytes = bincode::serialize(player)?;
        std::fs::write(self.dir.join("player.dat"), bytes)?;
        Ok(())
    }

    pub fn load_player(&self) -> Option<PlayerSave> {
        let bytes = std::fs::read(self.dir.join("player.dat")).ok()?;
        bincode::deserialize(&bytes).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn world_block_roundtrip_across_chunk_borders() {
        let mut w = World::new();
        w.ensure_chunk(0, 0);
        w.ensure_chunk(-1, -1);
        w.set_block(0, 5, 0, BlockState::STONE).unwrap();
        w.set_block(-1, 70, -1, BlockState::DIRT).unwrap(); // chunk (-1,-1), local (15,70,15)
        assert_eq!(w.get_block(0, 5, 0), BlockState::STONE);
        assert_eq!(w.get_block(-1, 70, -1), BlockState::DIRT);
        assert_eq!(w.get_block(-1, 5, -1), BlockState::AIR); // other block in same chunk
        assert_eq!(w.get_block(999, 5, 999), BlockState::AIR); // missing chunk
        assert!(w.is_solid(0, 5, 0));
        assert!(!w.is_solid(999, 5, 999));
    }

    #[test]
    fn y_bounds_and_surface_height() {
        let mut w = World::new();
        w.ensure_chunk(3, 4);
        w.set_block(48, 0, 64, BlockState::STONE).unwrap();
        w.set_block(48, 10, 64, BlockState::GRASS).unwrap();
        assert_eq!(w.surface_height(48, 64), 11);
        assert!(w.set_block(48, 256, 64, BlockState::STONE).is_none());
        assert!(w.set_block(48, -1, 64, BlockState::STONE).is_none());
        assert_eq!(w.get_block(48, 255, 64), BlockState::AIR);
    }

    #[test]
    fn mesh_column_offsets_into_world_space() {
        let mut w = World::new();
        w.ensure_chunk(2, 3);
        w.set_block(32 + 5, 0, 48 + 7, BlockState::GRASS).unwrap(); // chunk (2,3) local (5,0,7)
        let mesh = w.mesh_column(2, 3, &|_| 0);
        assert!(!mesh.vertices.is_empty());
        // every vertex lies within chunk (2,3): x 32..48, z 48..64
        for v in &mesh.vertices {
            assert!((32.0..=48.0).contains(&v.position[0]), "x out of chunk: {}", v.position[0]);
            assert!((48.0..=64.0).contains(&v.position[2]), "z out of chunk: {}", v.position[2]);
        }
    }

    #[test]
    fn mesh_column_culls_border_faces_with_neighbors() {
        let mut w = World::new();
        w.ensure_chunk(0, 0);
        w.ensure_chunk(1, 0);
        // solid wall at x=15 in chunk (0,0) and x=16 (=local 0 of chunk (1,0)): shared faces hidden
        for y in 0..4 {
            for z in 0..16 {
                w.set_block(15, y, z, BlockState::STONE).unwrap();
                w.set_block(16, y, z, BlockState::STONE).unwrap();
            }
        }
        let mesh = w.mesh_column(0, 0, &|_| 0);
        // with the neighbor present, no +X faces at x=16 plane: max vertex x should be exactly 16 (face boundary) but no face AT x==16 facing +X... simpler: fewer vertices than without neighbor
        let mut w2 = World::new();
        w2.ensure_chunk(0, 0);
        for y in 0..4 {
            for z in 0..16 {
                w2.set_block(15, y, z, BlockState::STONE).unwrap();
            }
        }
        let mesh2 = w2.mesh_column(0, 0, &|_| 0);
        assert!(mesh.vertices.len() < mesh2.vertices.len(), "neighbor culling failed");
    }

    #[test]
    fn world_storage_roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        let storage = WorldStorage::open(tmp.path());
        let mut world = World::new();
        world.ensure_chunk(3, -2);
        world.set_block(3 * 16 + 4, 40, -2 * 16 + 5, BlockState(7)).unwrap();
        storage.save_chunk(3, -2, world.chunk(3, -2).unwrap()).unwrap();
        let player = PlayerSave { position: [1.0, 70.0, 2.0], yaw: 0.5, pitch: -0.1 };
        storage.save_player(&player).unwrap();

        let loaded = storage.load_chunk(3, -2).unwrap();
        assert_eq!(loaded.get(4, 40, 5), BlockState(7));
        assert_eq!(storage.saved_chunks(), [(3, -2)].into_iter().collect());
        let p = storage.load_player().unwrap();
        assert_eq!(p.position, [1.0, 70.0, 2.0]);
        assert_eq!(p.yaw, 0.5);
    }
}
