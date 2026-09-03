use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

pub const REQUIRED_FILES: &[&str] = &[
    "README.md",
    "00-ZCODE-GOAL.md",
    "01-CURRENT-REALITY.md",
    "02-BETA-DEFINITION.md",
    "03-HUD-AND-CRAFTING.md",
    "04-WORLDGEN-SEEDS-AND-BIOMES.md",
    "05-CASTLES-FACTIONS-AND-STRATEGY.md",
    "06-NPC-AI-REPUTATION-AND-LIFE.md",
    "07-ASSET-BIBLE-AND-MANIFEST.md",
    "08-ZAI-VISION-AND-DEEP-TESTS.md",
    "09-PERFORMANCE-AND-REPO-HYGIENE.md",
    "10-OVERNIGHT-JOB-QUEUE.md",
    "11-DATA-CONTRACTS.md",
    "12-RELEASE-RECOVERY-AND-HANDOFF.md",
];

#[derive(Debug, PartialEq, Eq)]
pub struct PlanStats {
    pub documents: usize,
    pub jobs: usize,
    pub bytes: usize,
    pub links_checked: usize,
}

fn markdown_links(text: &str) -> impl Iterator<Item = &str> {
    text.split("](")
        .skip(1)
        .filter_map(|tail| tail.split(')').next())
}

fn local_markdown_target(doc: &Path, target: &str) -> Option<PathBuf> {
    let clean = target.split('#').next().unwrap_or(target);
    if clean.is_empty()
        || clean.starts_with("http://")
        || clean.starts_with("https://")
        || !clean.ends_with(".md")
    {
        return None;
    }
    Some(doc.parent().unwrap_or_else(|| Path::new(".")).join(clean))
}

fn parse_job_ids(text: &str) -> Result<Vec<u8>, String> {
    let mut ids = Vec::new();
    for line in text.lines() {
        let Some(rest) = line.strip_prefix("## N") else {
            continue;
        };
        let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
        if digits.len() != 2 {
            return Err(format!("malformed job heading: {line}"));
        }
        ids.push(
            digits
                .parse::<u8>()
                .map_err(|_| format!("invalid job number in heading: {line}"))?,
        );
    }
    Ok(ids)
}

/// Validate the nightly alpha-to-beta pack as an executable contract rather
/// than a loose pile of Markdown. This intentionally checks structure and
/// entry-point continuity; gameplay acceptance remains enforced by each job.
pub fn validate(root: &Path) -> Result<PlanStats, String> {
    if !root.is_dir() {
        return Err(format!("plan directory does not exist: {}", root.display()));
    }

    let mut contents = Vec::new();
    let mut bytes = 0usize;
    let mut links_checked = 0usize;
    for name in REQUIRED_FILES {
        let path = root.join(name);
        let text = std::fs::read_to_string(&path)
            .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
        if !text.starts_with("# ") {
            return Err(format!("{} must start with one H1 heading", path.display()));
        }
        if text.len() < 500 {
            return Err(format!(
                "{} is unexpectedly thin ({} bytes)",
                path.display(),
                text.len()
            ));
        }
        for target in markdown_links(&text) {
            if let Some(local) = local_markdown_target(&path, target) {
                links_checked += 1;
                if !local.is_file() {
                    return Err(format!(
                        "broken local Markdown link in {}: {}",
                        path.display(),
                        target
                    ));
                }
            }
        }
        bytes += text.len();
        contents.push((name, text));
    }

    let readme = &contents[0].1;
    for name in REQUIRED_FILES.iter().skip(1) {
        if !readme.contains(name) {
            return Err(format!("README.md does not link or name {name}"));
        }
    }

    let goal = contents
        .iter()
        .find(|(name, _)| **name == "00-ZCODE-GOAL.md")
        .map(|(_, text)| text.as_str())
        .unwrap_or_default();
    for marker in ["/goal", "make night-plan-check", "git push github HEAD"] {
        if !goal.contains(marker) {
            return Err(format!(
                "00-ZCODE-GOAL.md is missing required marker {marker:?}"
            ));
        }
    }

    let queue = contents
        .iter()
        .find(|(name, _)| **name == "10-OVERNIGHT-JOB-QUEUE.md")
        .map(|(_, text)| text.as_str())
        .unwrap_or_default();
    let ids = parse_job_ids(queue)?;
    let unique: BTreeSet<_> = ids.iter().copied().collect();
    let expected: Vec<u8> = (1..=24).collect();
    if ids != expected || unique.len() != expected.len() {
        return Err(format!(
            "job queue must contain exactly one ordered N01-N24 sequence; found {ids:?}"
        ));
    }

    let all = contents
        .iter()
        .map(|(_, text)| text.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    for marker in [
        "gravebound_court",
        "cinder_host",
        "confidence_0_to_1",
        "WorldIdentity",
        "Morning report template",
    ] {
        if !all.contains(marker) {
            return Err(format!(
                "nightly pack is missing required contract marker {marker:?}"
            ));
        }
    }

    Ok(PlanStats {
        documents: REQUIRED_FILES.len(),
        jobs: ids.len(),
        bytes,
        links_checked,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shipped_night_plan_is_complete_and_linked() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("docs/NIGHTLY-BETA");
        let stats = validate(&root).expect("the checked-in nightly goal pack must stay valid");
        assert_eq!(stats.documents, 14);
        assert_eq!(stats.jobs, 24);
        assert!(
            stats.bytes > 20_000,
            "goal pack unexpectedly lost substantial content"
        );
        assert!(
            stats.links_checked >= 12,
            "README should link the complete pack"
        );
    }

    #[test]
    fn job_parser_exposes_gaps_and_duplicates_to_validation() {
        assert_eq!(
            parse_job_ids("## N01 — one\n## N03 — three").unwrap(),
            vec![1, 3]
        );
        assert!(parse_job_ids("## N1 — malformed").is_err());
    }
}
