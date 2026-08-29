use std::cell::RefCell;
use std::collections::HashSet;
use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::light::{compute_column_light, ColumnLight};
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
    /// Flood-filled light per column, invalidated on edits. Interior
    /// mutability so meshing can stay `&self`.
    light_cache: RefCell<HashMap<(i32, i32), ColumnLight>>,
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
        // Edited column + the 8 surrounding columns relight next mesh:
        // light travels up to 15 blocks, so a mid-column torch edit can
        // change any neighbor's light (P28 cross-column lighting).
        let mut cache = self.light_cache.borrow_mut();
        for dx in -1..=1 {
            for dz in -1..=1 {
                cache.remove(&(cx + dx, cz + dz));
            }
        }
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
    /// Opaque and water faces are separated; light is flood-filled per column.
    /// `tex_of` selects the atlas layer per block AND face (per-face
    /// materials: grass top/side/bottom, log rings, ...).
    pub fn mesh_column(&self, cx: i32, cz: i32, tex_of: &dyn Fn(BlockState, crate::meshing::Face) -> u32) -> ColumnMesh {
        let mut opaque = MeshData::default();
        let mut water = MeshData::default();
        let col = match self.chunks.get(&(cx, cz)) {
            Some(c) => c,
            None => return ColumnMesh { opaque, water },
        };
        let key = (cx, cz);
        let light = {
            let mut cache = self.light_cache.borrow_mut();
            match cache.get(&key) {
                Some(l) => l.clone(),
                None => {
                    let l = compute_column_light(self, cx, cz, col);
                    cache.insert(key, l.clone());
                    l
                }
            }
        };
        // light_of receives SECTION-LOCAL coords (y in 0..16 per section);
        // the column light arrays are indexed by world y, so each section
        // translates by its own origin.
        let light_of_section = |oy: usize, x: i32, y: i32, z: i32| -> u32 {
            let world_y = y + oy as i32;
            if world_y < 0 {
                return 0; // below the world: dark
            }
            if world_y > 255 {
                return 0xF0; // above the world: full sky
            }
            let lx = x.clamp(0, 15) as usize;
            let lz = z.clamp(0, 15) as usize;
            let sky = light.sky_at(lx, world_y as usize, lz);
            let block_l = light.block_at(lx, world_y as usize, lz);
            ((sky as u32) << 4) | block_l as u32
        };
        for (sy, section) in col.sections.iter().enumerate() {
            let neighbor_px = self.chunks.get(&(cx + 1, cz)).map(|c| &c.sections[sy]);
            let neighbor_nx = self.chunks.get(&(cx - 1, cz)).map(|c| &c.sections[sy]);
            let neighbor_pz = self.chunks.get(&(cx, cz + 1)).map(|c| &c.sections[sy]);
            let neighbor_nz = self.chunks.get(&(cx, cz - 1)).map(|c| &c.sections[sy]);
            // Section E: diagonal sections so CTM bitmasks at section
            // corners still see their diagonal neighbours
            let diag_px_pz = self.chunks.get(&(cx + 1, cz + 1)).map(|c| &c.sections[sy]);
            let diag_px_nz = self.chunks.get(&(cx + 1, cz - 1)).map(|c| &c.sections[sy]);
            let diag_nx_pz = self.chunks.get(&(cx - 1, cz + 1)).map(|c| &c.sections[sy]);
            let diag_nx_nz = self.chunks.get(&(cx - 1, cz - 1)).map(|c| &c.sections[sy]);
            let neighbor_py = col.sections.get(sy + 1);
            let neighbor_ny = if sy > 0 { col.sections.get(sy - 1) } else { None };
            let oy_us = sy * 16;
            let light_of = |x: i32, y: i32, z: i32| light_of_section(oy_us, x, y, z);
            let mesh = meshing::mesh_section(
                section,
                neighbor_px, neighbor_nx, neighbor_py, neighbor_ny, neighbor_pz, neighbor_nz,
                diag_px_pz, diag_px_nz, diag_nx_pz, diag_nx_nz,
                tex_of,
                &light_of,
            );
            let oy = oy_us as f32;
            // Route each vertex into its channel and remember the new index.
            let mut remap = vec![0u32; mesh.vertices.len()];
            for (vi, v) in mesh.vertices.iter().enumerate() {
                let world_v = Vertex {
                    position: [v.position[0], v.position[1] + oy, v.position[2]],
                    ..*v
                };
                if meshing::is_water_layer(v.tex_index) {
                    remap[vi] = water.vertices.len() as u32;
                    water.vertices.push(world_v);
                } else {
                    remap[vi] = opaque.vertices.len() as u32;
                    opaque.vertices.push(world_v);
                }
            }
            for i in mesh.indices {
                let vi = i as usize;
                if meshing::is_water_layer(mesh.vertices[vi].tex_index) {
                    water.indices.push(remap[vi]);
                } else {
                    opaque.indices.push(remap[vi]);
                }
            }
        }
        // Offset to world space.
        let ox = (cx * 16) as f32;
        let oz = (cz * 16) as f32;
        for channel in [&mut opaque, &mut water] {
            for v in &mut channel.vertices {
                v.position[0] += ox;
                v.position[2] += oz;
            }
        }
        ColumnMesh { opaque, water }
    }
}

/// Texture atlas layer used for water faces (see lf_assets).
pub const WATER_TEX_LAYER: u32 = 10;

/// A column's mesh split by render pass.
#[derive(Default)]
pub struct ColumnMesh {
    pub opaque: MeshData,
    pub water: MeshData,
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

    /// Persist the world seed beside the player data. Every world owns its
    /// seed; generated fresh (OS entropy) when absent.
    pub fn save_seed(&self, seed: u64) -> Result<(), Box<dyn std::error::Error>> {
        let bytes = bincode::serialize(&seed)?;
        std::fs::write(self.dir.join("seed.dat"), bytes)?;
        Ok(())
    }

    pub fn load_seed(&self) -> Option<u64> {
        let bytes = std::fs::read(self.dir.join("seed.dat")).ok()?;
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
        let mesh = w.mesh_column(2, 3, &|_, _| 0);
        assert!(!mesh.opaque.vertices.is_empty());
        // every vertex lies within chunk (2,3): x 32..48, z 48..64
        for v in &mesh.opaque.vertices {
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
        let mesh = w.mesh_column(0, 0, &|_, _| 0);
        // with the neighbor present, no +X faces at x=16 plane: max vertex x should be exactly 16 (face boundary) but no face AT x==16 facing +X... simpler: fewer vertices than without neighbor
        let mut w2 = World::new();
        w2.ensure_chunk(0, 0);
        for y in 0..4 {
            for z in 0..16 {
                w2.set_block(15, y, z, BlockState::STONE).unwrap();
            }
        }
        let mesh2 = w2.mesh_column(0, 0, &|_, _| 0);
        assert!(mesh.opaque.vertices.len() < mesh2.opaque.vertices.len(), "neighbor culling failed");
    }

    #[test]
    fn light_cache_invalidates_on_edits() {
        let mut w = World::new();
        w.ensure_chunk(0, 0);
        for lx in 0..16 {
            for lz in 0..16 {
                w.set_block(lx, 0, lz, crate::BlockState(registry::block::STONE)).unwrap();
            }
        }
        // mesh once to fill the cache (light is sky-lit)
        let lit = w.mesh_column(0, 0, &|_, _| 0);
        // cap: verify light attribute is sky-lit high up (sky nibble = 15)
        assert!(lit.opaque.vertices.iter().any(|v| (v.light >> 4) == 15));
        // place a torch and remesh: block light must appear
        w.set_block(8, 100, 8, crate::BlockState(registry::block::TORCH)).unwrap();
        let relit = w.mesh_column(0, 0, &|_, _| 0);
        assert!(relit.opaque.vertices.iter().any(|v| (v.light & 0xF) > 0),
            "torch light must be visible after the edit (cache invalidated)");
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
