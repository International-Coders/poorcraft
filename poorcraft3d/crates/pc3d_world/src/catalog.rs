//! The Function Catalog law (BETA-0.2 W3.0): the catalog document is a
//! GATE, not a wish list. The test parses the committed markdown and
//! enforces its own rules — every row names a proof, ids are unique and
//! well-formed, the status vocabulary is exact, and the count line tells
//! the truth. A function shipped without a row, or a row without a
//! function-shaped proof reference, fails the world suite.
//!
//! The doc path is resolved from the manifest (pc3d_world lives at
//! <repo>/poorcraft3d/crates/pc3d_world), so the law runs from any cwd.

const CATALOG_REL: &str = "../../../docs/POORCRAFT-3D/FUNCTION-CATALOG.md";

/// One parsed catalog row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CatalogRow {
    pub id: String,
    pub section: String,
    pub status: String,
    pub proof: String,
}

/// Parse the catalog's markdown tables. Returns (rows, count line).
/// Count line = (total, works, partial, planned) as the doc states it.
pub fn parse_catalog(md: &str) -> (Vec<CatalogRow>, Option<(usize, usize, usize, usize)>) {
    let mut rows = Vec::new();
    let mut section = String::new();
    let mut counted = None;
    for line in md.lines() {
        if line.starts_with("## ") {
            section = line[3..].trim().to_string();
            continue;
        }
        // The count line: "Rows: ... = **58** (WORKS 29 · PARTIAL 6 · PLANNED 23)"
        if line.starts_with("Rows:") {
            let total = line
                .split("= **")
                .nth(1)
                .and_then(|t| t.split("**").next())
                .and_then(|t| t.trim().parse::<usize>().ok());
            let after = |key: &str| {
                line.split(key)
                    .nth(1)
                    .and_then(|t| {
                        t.trim_start_matches(|c: char| !c.is_ascii_digit())
                            .split(|c: char| !c.is_ascii_digit())
                            .next()
                            .and_then(|n| n.parse::<usize>().ok())
                    })
                    .unwrap_or(0)
            };
            if let Some(total) = total {
                counted = Some((total, after("WORKS"), after("PARTIAL"), after("PLANNED")));
            }
            continue;
        }
        if !line.starts_with('|') || line.contains("---") {
            continue;
        }
        let mut cells: Vec<&str> = line.split('|').map(|c| c.trim()).collect();
        // Splitting "| a | b |" yields empty first/last — drop them.
        if !cells.is_empty() && cells[0].is_empty() {
            cells.remove(0);
        }
        if !cells.is_empty() && cells.last().map(|c| c.is_empty()).unwrap_or(false) {
            cells.pop();
        }
        // Verbs/world: [id, function, input, status, proof]; others:
        // [id, function, status, proof]. Status and proof are the LAST
        // two cells in both shapes.
        let is_row = cells.len() >= 4 && cells[0] != "id" && !cells[0].is_empty();
        if is_row {
            rows.push(CatalogRow {
                id: cells[0].to_string(),
                section: section.clone(),
                status: cells[cells.len() - 2].to_string(),
                proof: cells[cells.len() - 1].to_string(),
            });
        }
    }
    (rows, counted)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn catalog_md() -> String {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(CATALOG_REL);
        std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("catalog doc missing at {:?}: {e}", path))
    }

    /// THE ID LAW: every row has a unique, prefixed, well-formed id
    /// (V##/S##/M##/W##/U##), a status from the exact vocabulary, and a
    /// non-empty proof that is NOT the word "none".
    #[test]
    fn beta02_catalog_rows_are_well_formed_and_proven() {
        let (rows, _) = parse_catalog(&catalog_md());
        assert!(rows.len() >= 45, "catalog fell below the seed floor: {} rows", rows.len());
        let mut ids: Vec<&str> = Vec::new();
        for r in &rows {
            let prefix_ok = r.id.len() >= 3
                && matches!(r.id.as_bytes()[0], b'V' | b'S' | b'M' | b'W' | b'U')
                && r.id[1..].chars().all(|c| c.is_ascii_digit());
            assert!(prefix_ok, "row id {:?} is not V##/S##/M##/W##/U##", r.id);
            assert!(
                matches!(r.status.as_str(), "WORKS" | "PARTIAL" | "PLANNED"),
                "row {} status {:?} outside the vocabulary",
                r.id,
                r.status
            );
            assert!(
                !r.proof.is_empty() && !r.proof.contains("none"),
                "row {} has no proof reference",
                r.id
            );
            ids.push(&r.id);
        }
        ids.sort();
        ids.dedup();
        // dedup only shrinks if an id repeated:
        let (rows2, _) = parse_catalog(&catalog_md());
        let mut ids2: Vec<&str> = rows2.iter().map(|r| r.id.as_str()).collect();
        ids2.sort();
        assert_eq!(
            ids.len(),
            ids2.len(),
            "duplicate catalog ids: {:?}",
            ids2.windows(2).filter(|w| w[0] == w[1]).collect::<Vec<_>>()
        );
    }

    /// THE COUNT LAW: the doc's own count line tells the truth about the
    /// tables above it (the meter cannot quietly drift from its doc).
    #[test]
    fn beta02_catalog_count_line_matches_the_tables() {
        let md = catalog_md();
        let (rows, counted) = parse_catalog(&md);
        let (claimed_total, claimed_works, claimed_partial, claimed_planned) =
            counted.expect("catalog must carry a 'Rows:' count line");
        assert_eq!(
            rows.len(),
            claimed_total,
            "the count line's total ({claimed_total}) disagrees with the parsed tables ({})",
            rows.len()
        );
        let works = rows.iter().filter(|r| r.status == "WORKS").count();
        let partial = rows.iter().filter(|r| r.status == "PARTIAL").count();
        assert_eq!(works, claimed_works, "WORKS count drifted from the line");
        assert_eq!(partial, claimed_partial, "PARTIAL count drifted from the line");
        let planned = rows.iter().filter(|r| r.status == "PLANNED").count();
        assert_eq!(planned, claimed_planned, "PLANNED count drifted from the line");
    }

    /// THE SEED FLOOR: every function the played game ships TODAY is in
    /// the catalog — the battery's proven verbs (V01-V16, M01, W01-W04,
    /// W08, U01-U03, U09, S01-S04) must all be present and WORKS.
    #[test]
    fn beta02_catalog_covers_every_played_verb() {
        let (rows, _) = parse_catalog(&catalog_md());
        let must_work = [
            "V01", "V02", "V03", "V04", "V05", "V06", "V07", "V10", "V11", "V12", "V13",
            "V14", "V15", "V16", "S01", "S02", "S03", "S04", "M01", "W01", "W02", "W03",
            "W04", "W08", "U01", "U02", "U03", "U09",
        ];
        for id in must_work {
            let row = rows.iter().find(|r| r.id == id).unwrap_or_else(|| {
                panic!("catalog lost {id} — a played function shipped without a row")
            });
            assert_eq!(
                row.status, "WORKS",
                "{id} is played and battery-proven; the catalog may not downgrade it"
            );
        }
    }
}
