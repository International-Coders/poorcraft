//! P3D-302: flow-table persistence through the header law.

use crate::framing::{frame, unframe, FrameError};
use crate::store::{write_atomic, LoadError};
use pc3d_core::{FormatHeader, SupportedVersions};
use pc3d_world::flow::{FlowRecord, FlowTable};
use std::fs;
use std::path::Path;

/// Fixed-width record (48 bytes): region_x,region_z i32 | direction u8 |
/// slope i32 | discharge u64 | capacity u64 | revision u64 | pad 7.
fn encode_table(t: &FlowTable) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&(t.revision as u64).to_le_bytes());
    out.extend_from_slice(&(t.records.len() as u64).to_le_bytes());
    for r in t.records.values() {
        out.extend_from_slice(&r.region_x.to_le_bytes());
        out.extend_from_slice(&r.region_z.to_le_bytes());
        out.push(r.direction);
        out.extend_from_slice(&r.slope_per_mille.to_le_bytes());
        out.extend_from_slice(&[0, 0, 0]); // alignment pad
        out.extend_from_slice(&r.discharge.to_le_bytes());
        out.extend_from_slice(&r.capacity.to_le_bytes());
        out.extend_from_slice(&r.revision.to_le_bytes());
        out.extend_from_slice(&[0u8; 8]);
    }
    out
}

fn decode_table(bytes: &[u8]) -> Result<FlowTable, LoadError> {
    if bytes.len() < 16 {
        return Err(LoadError::Framing(FrameError::TooShort));
    }
    let revision = u64::from_le_bytes(bytes[..8].try_into().unwrap());
    let count = u64::from_le_bytes(bytes[8..16].try_into().unwrap()) as usize;
    if bytes.len() != 16 + count * 48 {
        return Err(LoadError::Framing(FrameError::LengthMismatch {
            declared: (16 + count * 48) as u64,
            actual: bytes.len(),
        }));
    }
    let mut records = std::collections::BTreeMap::new();
    for i in 0..count {
        let off = 16 + i * 48;
        let rec = &bytes[off..off + 48];
        let region_x = i32::from_le_bytes(rec[0..4].try_into().unwrap());
        let region_z = i32::from_le_bytes(rec[4..8].try_into().unwrap());
        let direction = rec[8];
        let slope_per_mille = i32::from_le_bytes(rec[9..13].try_into().unwrap());
        let discharge = u64::from_le_bytes(rec[16..24].try_into().unwrap());
        let capacity = u64::from_le_bytes(rec[24..32].try_into().unwrap());
        let revision = u64::from_le_bytes(rec[32..40].try_into().unwrap());
        records.insert(
            (region_x, region_z),
            FlowRecord {
                region_x,
                region_z,
                direction,
                slope_per_mille,
                discharge,
                capacity,
                revision,
            },
        );
    }
    Ok(FlowTable { revision, records })
}

/// Atomically persist a flow table.
pub fn save_flow_table(
    save_root: &Path,
    world_name: &str,
    table: &FlowTable,
    supported: &SupportedVersions,
) -> Result<(), LoadError> {
    let header = FormatHeader { save: supported.save, ..FormatHeader::current() };
    let bytes = frame(&header, &encode_table(table));
    let path = crate::paths::world_root(save_root, world_name)
        .join(std::path::PathBuf::from("water/flow.p3d"));
    write_atomic(&path, &bytes)?;
    Ok(())
}

/// Load a flow table through the refusal law.
pub fn load_flow_table(
    save_root: &Path,
    world_name: &str,
    supported: &SupportedVersions,
) -> Result<FlowTable, LoadError> {
    let path = crate::paths::world_root(save_root, world_name)
        .join(std::path::PathBuf::from("water/flow.p3d"));
    let bytes = fs::read(path)?;
    Ok(decode_table(&unframe(&bytes, supported)?)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pc3d_world::gen::WorldGen;
    use pc3d_world::hydro::RiverGraph;

    const SUP: SupportedVersions = SupportedVersions::epoch1();

    /// Flow tables round-trip through disk; revision bumps persist; the
    /// refusal law applies.
    #[test]
    fn p3d302_flow_table_round_trips_and_refuses() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let graph = RiverGraph::new(&WorldGen::new(7), 8);
        let mut table = FlowTable::from_graph(&graph);
        save_flow_table(root, "w", &table, &SUP).unwrap();
        let loaded = load_flow_table(root, "w", &SUP).unwrap();
        assert_eq!(loaded, table);

        // Bump + re-save: the revision survives.
        table.bump_revision();
        save_flow_table(root, "w", &table, &SUP).unwrap();
        assert_eq!(load_flow_table(root, "w", &SUP).unwrap().revision(), 2);

        // Foreign file refused.
        let path = crate::paths::world_root(root, "w")
            .join(std::path::PathBuf::from("water/flow.p3d"));
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, b"garbage bytes that are long enough to pass the magic check").unwrap();
        assert!(matches!(
            load_flow_table(root, "w", &SUP),
            Err(LoadError::Framing(FrameError::ForeignFormat))
        ));
    }
}
