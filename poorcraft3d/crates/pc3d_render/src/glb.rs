//! GLB (glTF 2.0 binary) asset loading (NWR-002).
//!
//! A deliberately small reader for exactly what the asset factory emits
//! (and what the milestone needs): nodes named `lod0/lod1/lod2` carrying
//! meshes of POSITION/NORMAL f32 accessors + u32 indices, materials with
//! `baseColorFactor` (linear RGB — our shader's space), and empty
//! `socket.*` nodes with translations. Anything malformed fails with a
//! named error; LOD selection is by camera distance with fallback to the
//! coarsest available LOD.

use crate::scene::SceneVertex;
use serde_json::Value;

#[derive(Debug)]
pub enum GlbError {
    NotGlb,
    Truncated(&'static str),
    NoJsonChunk,
    Json(String),
    Missing(&'static str),
}

impl std::fmt::Display for GlbError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GlbError::NotGlb => write!(f, "not a GLB (bad magic)"),
            GlbError::Truncated(what) => write!(f, "truncated GLB: {what}"),
            GlbError::NoJsonChunk => write!(f, "no JSON chunk"),
            GlbError::Json(e) => write!(f, "glTF json: {e}"),
            GlbError::Missing(what) => write!(f, "missing {what}"),
        }
    }
}

struct Document {
    json: Value,
    bin: Vec<u8>,
}

fn parse_glb(bytes: &[u8]) -> Result<Document, GlbError> {
    if bytes.len() < 12 || &bytes[0..4] != b"glTF" {
        return Err(GlbError::NotGlb);
    }
    let version = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
    if version != 2 {
        return Err(GlbError::NotGlb);
    }
    let total = u32::from_le_bytes(bytes[8..12].try_into().unwrap()) as usize;
    if total != bytes.len() {
        return Err(GlbError::Truncated("declared length != file length"));
    }
    let mut pos = 12;
    let mut json: Option<Value> = None;
    let mut bin = Vec::new();
    while pos + 8 <= bytes.len() {
        let len = u32::from_le_bytes(bytes[pos..pos + 4].try_into().unwrap()) as usize;
        let kind = &bytes[pos + 4..pos + 8];
        let start = pos + 8;
        let end = start.checked_add(len).ok_or(GlbError::Truncated("chunk length"))?;
        if end > bytes.len() {
            return Err(GlbError::Truncated("chunk exceeds file"));
        }
        if kind == b"JSON" {
            let s = std::str::from_utf8(&bytes[start..end])
                .map_err(|_| GlbError::Json("utf8".into()))?;
            json = Some(serde_json::from_str(s.trim_end()).map_err(|e| GlbError::Json(e.to_string()))?);
        } else if kind == b"BIN\x00" {
            bin = bytes[start..end].to_vec();
        }
        pos = end;
    }
    Ok(Document {
        json: json.ok_or(GlbError::NoJsonChunk)?,
        bin,
    })
}

fn read_f32_vec3(doc: &Document, accessor: &Value) -> Result<Vec<[f32; 3]>, GlbError> {
    let view_i = accessor["bufferView"].as_u64().ok_or(GlbError::Missing("bufferView"))? as usize;
    let views = doc.json["bufferViews"].as_array().ok_or(GlbError::Missing("bufferViews"))?;
    let view = views.get(view_i).ok_or(GlbError::Missing("bufferView entry"))?;
    let off = view["byteOffset"].as_u64().unwrap_or(0) as usize;
    let len = view["byteLength"].as_u64().ok_or(GlbError::Missing("byteLength"))? as usize;
    let count = accessor["count"].as_u64().ok_or(GlbError::Missing("count"))? as usize;
    let mut out = Vec::with_capacity(count);
    for i in 0..count {
        let b = off + i * 12;
        if b + 12 > doc.bin.len() || b + 12 > off + len {
            return Err(GlbError::Truncated("accessor data"));
        }
        let f = |k: usize| {
            f32::from_le_bytes(doc.bin[b + k..b + k + 4].try_into().unwrap())
        };
        out.push([f(0), f(4), f(8)]);
    }
    Ok(out)
}

fn read_u32_indices(doc: &Document, accessor: &Value) -> Result<Vec<u32>, GlbError> {
    let view_i = accessor["bufferView"].as_u64().ok_or(GlbError::Missing("bufferView"))? as usize;
    let views = doc.json["bufferViews"].as_array().ok_or(GlbError::Missing("bufferViews"))?;
    let view = views.get(view_i).ok_or(GlbError::Missing("bufferView entry"))?;
    let off = view["byteOffset"].as_u64().unwrap_or(0) as usize;
    let len = view["byteLength"].as_u64().ok_or(GlbError::Missing("byteLength"))? as usize;
    let count = accessor["count"].as_u64().ok_or(GlbError::Missing("count"))? as usize;
    let mut out = Vec::with_capacity(count);
    for i in 0..count {
        let b = off + i * 4;
        if b + 4 > doc.bin.len() || b + 4 > off + len {
            return Err(GlbError::Truncated("index data"));
        }
        out.push(u32::from_le_bytes(doc.bin[b..b + 4].try_into().unwrap()));
    }
    Ok(out)
}

/// One LOD of a loaded asset: a ready mesh (pos/normal/color) in the
/// asset's local space (pivot at ground, meters, +Y up).
#[derive(Clone, Default)]
pub struct LodMesh {
    pub name: String,
    pub vertices: Vec<SceneVertex>,
    pub indices: Vec<u32>,
}

impl LodMesh {
    pub fn triangles(&self) -> usize {
        self.indices.len() / 3
    }
    /// Bakes a world translation (meters) into a copy of the mesh.
    pub fn translated(&self, t: [f32; 3]) -> LodMesh {
        let mut out = self.clone();
        for v in &mut out.vertices {
            v.pos[0] += t[0];
            v.pos[1] += t[1];
            v.pos[2] += t[2];
        }
        out
    }
}

/// A parsed asset: LOD meshes by name + named sockets (world-local).
#[derive(Clone, Default)]
pub struct Asset {
    pub lods: Vec<LodMesh>,
    pub sockets: Vec<(String, [f32; 3])>,
}

impl Asset {
    /// The LOD to draw at a given camera distance: lod0 < 40 m,
    /// lod1 < 120 m, lod2 beyond — falling back to the coarsest LOD the
    /// asset actually has (never fails).
    pub fn lod_for(&self, distance_m: f32) -> &LodMesh {
        let want = if distance_m < 40.0 {
            "lod0"
        } else if distance_m < 120.0 {
            "lod1"
        } else {
            "lod2"
        };
        if let Some(l) = self.lods.iter().find(|l| l.name == want) {
            return l;
        }
        // Fallback: the coarsest present (last), or any.
        self.lods.last().expect("asset with no LODs")
    }

    /// The socket's local translation, if present.
    pub fn socket(&self, name: &str) -> Option<[f32; 3]> {
        self.sockets
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, p)| *p)
    }
}

/// Parses an asset GLB from bytes.
pub fn load_asset(bytes: &[u8]) -> Result<Asset, GlbError> {
    let doc = parse_glb(bytes)?;
    let meshes = doc.json["meshes"].as_array().ok_or(GlbError::Missing("meshes"))?;
    let nodes = doc.json["nodes"].as_array().ok_or(GlbError::Missing("nodes"))?;
    let accessors = doc.json["accessors"].as_array().ok_or(GlbError::Missing("accessors"))?;
    let materials = doc.json["materials"].as_array().ok_or(GlbError::Missing("materials"))?;

    let mut asset = Asset::default();
    for node in nodes {
        let name = node["name"].as_str().unwrap_or("");
        if let Some(mesh_i) = node["mesh"].as_u64() {
            let mesh = meshes.get(mesh_i as usize).ok_or(GlbError::Missing("mesh entry"))?;
            let mut verts: Vec<SceneVertex> = Vec::new();
            let mut idx: Vec<u32> = Vec::new();
            let prims = mesh["primitives"].as_array().ok_or(GlbError::Missing("primitives"))?;
            for prim in prims {
                let pos = read_f32_vec3(
                    &doc,
                    accessors
                        .get(prim["attributes"]["POSITION"].as_u64().ok_or(GlbError::Missing("POSITION"))? as usize)
                        .ok_or(GlbError::Missing("POSITION accessor"))?,
                )?;
                let nrm = read_f32_vec3(
                    &doc,
                    accessors
                        .get(prim["attributes"]["NORMAL"].as_u64().ok_or(GlbError::Missing("NORMAL"))? as usize)
                        .ok_or(GlbError::Missing("NORMAL accessor"))?,
                )?;
                let indices = read_u32_indices(
                    &doc,
                    accessors
                        .get(prim["indices"].as_u64().ok_or(GlbError::Missing("indices"))? as usize)
                        .ok_or(GlbError::Missing("indices accessor"))?,
                )?;
                let mat_i = prim["material"].as_u64().unwrap_or(0) as usize;
                let factor = materials
                    .get(mat_i)
                    .and_then(|m| m["pbrMetallicRoughness"]["baseColorFactor"].as_array());
                let color = factor
                    .map(|f| {
                        [
                            f.first().and_then(|v| v.as_f64()).unwrap_or(1.0) as f32,
                            f.get(1).and_then(|v| v.as_f64()).unwrap_or(1.0) as f32,
                            f.get(2).and_then(|v| v.as_f64()).unwrap_or(1.0) as f32,
                        ]
                    })
                    .unwrap_or([0.7, 0.7, 0.7]);
                let base = verts.len() as u32;
                for (p, n) in pos.iter().zip(nrm.iter()) {
                    verts.push(SceneVertex {
                        pos: *p,
                        normal: *n,
                        color,
                    });
                }
                idx.extend(indices.iter().map(|i| i + base));
            }
            asset.lods.push(LodMesh {
                name: name.to_string(),
                vertices: verts,
                indices: idx,
            });
        } else if let Some(t) = node["translation"].as_array() {
            if let Some(name) = name.strip_prefix("socket.") {
                asset.sockets.push((
                    name.to_string(),
                    [
                        t.first().and_then(|v| v.as_f64()).unwrap_or(0.0) as f32,
                        t.get(1).and_then(|v| v.as_f64()).unwrap_or(0.0) as f32,
                        t.get(2).and_then(|v| v.as_f64()).unwrap_or(0.0) as f32,
                    ],
                ));
            }
        }
    }
    if asset.lods.is_empty() {
        return Err(GlbError::Missing("lod meshes"));
    }
    Ok(asset)
}

pub fn load_asset_file(path: &std::path::Path) -> Result<Asset, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    load_asset(&bytes).map_err(|e| format!("{}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repo() -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../..")
            .canonicalize()
            .unwrap()
    }

    fn asset_path(rel: &str) -> std::path::PathBuf {
        repo().join("poorcraft3d/assets/compiled").join(rel)
    }

    #[test]
    fn loads_all_three_factory_assets_with_lods_and_budgets() {
        // Tree: 3 LODs under the pack budgets; sockets absent.
        let tree = load_asset_file(&asset_path("prop/tree_ash.glb")).expect("tree");
        let names: Vec<&str> = tree.lods.iter().map(|l| l.name.as_str()).collect();
        assert_eq!(names, vec!["lod0", "lod1", "lod2"]);
        assert!(tree.lods[0].triangles() <= 1800, "{} > 1800", tree.lods[0].triangles());
        assert!(tree.lods[1].triangles() <= 700);
        assert!(tree.lods[2].triangles() <= 180);
        // Every vertex sits above ground (pivot at feet) and within a sane
        // bounding box (a ~6-7 m tree).
        let max_y = tree.lods[0].vertices.iter().map(|v| v.pos[1]).fold(f32::MIN, f32::max);
        assert!(max_y > 5.0 && max_y < 8.5, "tree height {max_y}");
        // Rock: 2 LODs.
        let rock = load_asset_file(&asset_path("prop/rock_granite.glb")).expect("rock");
        assert!(rock.lods[0].triangles() <= 600);
        assert!(rock.lods[1].triangles() <= 180);
        // House: 3 LODs + the four named sockets from the pack manifest.
        let house = load_asset_file(&asset_path("module/house_croft.glb")).expect("house");
        assert!(house.lods[0].triangles() <= 2500);
        for socket in ["door_front", "road_front", "roof_smoke", "build_base"] {
            assert!(house.socket(socket).is_some(), "missing socket {socket}");
        }
        // The door socket sits on the front (-Z) wall at ground level.
        let d = house.socket("door_front").unwrap();
        assert!(d[2] < -2.0 && d[1].abs() < 0.01, "door socket {d:?}");
    }

    #[test]
    fn lod_selection_and_fallback() {
        let tree = load_asset_file(&asset_path("prop/tree_ash.glb")).unwrap();
        assert_eq!(tree.lod_for(5.0).name, "lod0");
        assert_eq!(tree.lod_for(80.0).name, "lod1");
        assert_eq!(tree.lod_for(500.0).name, "lod2");
        // Rock has no lod2: falls back to the coarsest present (lod1).
        let rock = load_asset_file(&asset_path("prop/rock_granite.glb")).unwrap();
        assert_eq!(rock.lod_for(500.0).name, "lod1");
    }

    #[test]
    fn malformed_and_missing_inputs_fail_named() {
        assert!(load_asset(b"not a glb").is_err());
        assert!(load_asset(b"glTF\x02\x00\x00\x00\x0c\x00\x00\x00").is_err());
        assert!(load_asset_file(std::path::Path::new("/nonexistent/x.glb")).is_err());
        // A GLB with no lod meshes fails.
        let doc = serde_json::json!({
            "asset": {"version": "2.0"},
            "scenes": [{"nodes": []}],
            "nodes": [],
            "meshes": [],
            "accessors": [],
            "bufferViews": [],
        });
        let mut json = serde_json::to_vec(&doc).unwrap();
        while json.len() % 4 != 0 { json.push(b' '); }
        let mut f = Vec::new();
        f.extend_from_slice(b"glTF");
        f.extend_from_slice(&2u32.to_le_bytes());
        let total = 12 + 8 + json.len();
        f.extend_from_slice(&(total as u32).to_le_bytes());
        f.extend_from_slice(&(json.len() as u32).to_le_bytes());
        f.extend_from_slice(b"JSON");
        f.extend_from_slice(&json);
        assert!(matches!(load_asset(&f), Err(GlbError::Missing(_))));
    }
}

#[cfg(test)]
mod gpu_tests {
    use super::*;
    use crate::camera::CameraPose;
    use crate::scene::{project_ndc, to_srgb4, Probe};

    /// GPU proof: all three factory assets render in one windowed scene at
    /// correct scale with material binding (leaf green, granite grey,
    /// thatch roof) and depth; probes are placed on projected asset
    /// points; a control render without assets proves each probe pixel
    /// belongs to an asset.
    #[test]
    fn factory_assets_render_with_materials() {
        let (seed, coord) = pc3d_world::terrain::SceneSpec::SmoothHills.patch();
        let gen = pc3d_world::gen::WorldGen::new(seed);
        let tree = load_asset_file(&asset_path("prop/tree_ash.glb")).unwrap();
        let rock = load_asset_file(&asset_path("prop/rock_granite.glb")).unwrap();
        let house = load_asset_file(&asset_path("module/house_croft.glb")).unwrap();

        // Placements on the patch (terrain heights come from the ground).
        let ground = |x: f32, z: f32| {
            gen.effective_surface_mm((x * 1000.0) as i64, (z * 1000.0) as i64) as f32 / 1000.0
        };
        let t_pos = [coord.origin().x as f32 / 1000.0 + 6.0, 0.0, coord.origin().z as f32 / 1000.0 + 6.0];
        let r_pos = [t_pos[0] + 5.0, 0.0, t_pos[2] + 2.0];
        let h_pos = [t_pos[0] - 2.0, 0.0, t_pos[2] + 10.0];
        let (tp, rp, hp) = (
            [t_pos[0], ground(t_pos[0], t_pos[2]), t_pos[2]],
            [r_pos[0], ground(r_pos[0], r_pos[2]), r_pos[2]],
            [h_pos[0], ground(h_pos[0], h_pos[2]), h_pos[2]],
        );

        let mut r = crate::renderer::Renderer::offscreen(384, 288);
        r.set_placeholder_scene(false);
        // A small terrain band so the assets sit ON the world.
        let patch = pc3d_world::coords::PatchCoord {
            x: (t_pos[0] as i32).div_euclid(16),
            y: 1,
            z: (t_pos[2] as i32).div_euclid(16),
        };
        let mut patches = Vec::new();
        for dx in -1..=1i32 {
            for dz in -1..=1i32 {
                patches.push(pc3d_world::coords::PatchCoord { x: patch.x + dx, y: patch.y, z: patch.z + dz });
            }
        }
        r.load_terrain(&gen, &patches);
        // Near tree lod0, far tree lod1 (LOD variety in one scene), rock
        // lod0, house lod0.
        let tris = [
            r.load_asset(&tree, "lod0", tp),
            r.load_asset(&tree, "lod1", [tp[0] + 12.0, ground(tp[0] + 12.0, tp[2] - 8.0), tp[2] - 8.0]),
            r.load_asset(&rock, "lod0", rp),
            r.load_asset(&house, "lod0", hp),
        ];
        let total: usize = tris.iter().sum();
        assert!(total > 500, "a real scene: {total} tris");
        println!("asset scene: {tris:?} = {total} tris, {} draws", 4);

        // Vantage: south of the pair, looking north.
        let eye = [tp[0] - 2.0, tp[1] + 3.2, tp[2] + 14.0];
        let aim = [tp[0] + 1.0, tp[1] + 1.8, tp[2]];
        let d = [aim[0] - eye[0], aim[1] - eye[1], aim[2] - eye[2]];
        let pose = CameraPose::new(
            eye,
            (-d[0]).atan2(-d[2]),
            (d[1] / (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt()).asin(),
        );
        r.set_pose(pose);
        let aspect = 384.0 / 288.0;

        // Control (no assets) via a fresh renderer with the same scene.
        let mut ctrl = crate::renderer::Renderer::offscreen(384, 288);
        ctrl.set_placeholder_scene(false);
        ctrl.load_terrain(&gen, &patches);
        ctrl.set_pose(pose);

        let leaf = [0.23, 0.40, 0.20];
        let granite = [0.46, 0.48, 0.50];
        let thatch = [0.55, 0.43, 0.24];
        // Probe points on the assets (world meters): tree crown, rock
        // flank, house roof slope. Their exact pixel colors vary with the
        // facet normals — so the proof is CONTROL-DIFFERENCE (the pixel
        // changed because the asset is there) plus a hue-class check for
        // the leaf probe (green channel dominant).
        let probes: [(&str, [f32; 3]); 3] = [
            ("tree_crown", [tp[0], tp[1] + 6.3, tp[2]]),
            ("rock_flank", [rp[0] - 0.9, rp[1] + 0.4, rp[2]]),
            ("house_roof", [hp[0], hp[1] + 3.5, hp[2] + 1.2]),
        ];
        let (report, rgba) = r.capture_png(
            &std::env::temp_dir().join("pc3d_assets.png"),
            &[Probe {
                name: "sky",
                ndc: (0.0, 0.85),
                expected: to_srgb4(crate::scene::sky_color_linear(
                    crate::scene::dir_from_ndc(pose, (0.0, 0.85), aspect),
                    crate::scene::SUN_DIR,
                )),
                tol: 0.05,
            }],
        );
        assert!(report.passes_with(12), "{:?}", report.failed_probes());
        let (_, ctrl_rgba) = ctrl.capture_png(&std::env::temp_dir().join("pc3d_assets_ctrl.png"), &[]);
        for (name, point) in probes {
            let ndc = project_ndc(pose, aspect, point);
            let with = crate::scene::sample_ndc(&rgba, 384, 288, ndc);
            let without = crate::scene::sample_ndc(&ctrl_rgba, 384, 288, ndc);
            let delta: f32 = (0..3).map(|i| (with[i] - without[i]).abs()).sum();
            assert!(
                delta > 0.05,
                "{name} not visible (delta {delta}: {with:?} vs {without:?})"
            );
            println!("{name}: visible (delta {delta:.2})");
        }
        // Material association: SOME pixel in the crown region must read
        // as FOLIAGE — green-dominant and different from the control
        // (clusters are discrete; a single projected point can fall
        // between them onto sky).
        let mut foliage_pixels = 0usize;
        for h in [4.6f32, 5.1, 5.6, 6.1, 6.4] {
            for dx in [-0.5f32, 0.0, 0.5] {
                let pt = [tp[0] + dx, tp[1] + h, tp[2]];
                let ndc = project_ndc(pose, aspect, pt);
                if ndc.0 <= -0.99 || ndc.0 >= 0.99 || ndc.1 <= -0.99 || ndc.1 >= 0.99 {
                    continue;
                }
                let with = crate::scene::sample_ndc(&rgba, 384, 288, ndc);
                let without = crate::scene::sample_ndc(&ctrl_rgba, 384, 288, ndc);
                let changed = (0..3).map(|i| (with[i] - without[i]).abs()).sum::<f32>() > 0.05;
                if changed && with[1] > with[0] && with[1] > with[2] {
                    foliage_pixels += 1;
                }
            }
        }
        assert!(
            foliage_pixels >= 2,
            "crown region must contain foliage-colored pixels ({foliage_pixels})"
        );
    }

    fn asset_path(rel: &str) -> std::path::PathBuf {
        repo().join("poorcraft3d/assets/compiled").join(rel)
    }
    fn repo() -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../..")
            .canonicalize()
            .unwrap()
    }
}
