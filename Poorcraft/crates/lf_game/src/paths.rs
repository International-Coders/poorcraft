//! P37 paths & specialization: four non-decaying, non-locking standings
//! (Engineer / Architect / Battlemage / Artisan) accrued by play, plus
//! the generalized gate era-vs-path and the respec sink.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Path {
    Engineer,
    Architect,
    Battlemage,
    Artisan,
}

impl Path {
    pub const ALL: [Path; 4] = [Path::Engineer, Path::Architect, Path::Battlemage, Path::Artisan];

    pub fn name(self) -> &'static str {
        match self {
            Path::Engineer => "Engineer",
            Path::Architect => "Architect",
            Path::Battlemage => "Battlemage",
            Path::Artisan => "Artisan",
        }
    }

    pub fn desc(self) -> &'static str {
        match self {
            Path::Engineer => "machines hum under your hands",
            Path::Architect => "the valley takes your shape",
            Path::Battlemage => "spell and steel, both yours",
            Path::Artisan => "everything you make is better",
        }
    }
}

/// What play accrues (P37 doc 07): machines run, blocks placed, spells
/// cast and bosses felled, items crafted and enchanted.
#[derive(Clone, Copy, Debug)]
pub enum PathEvent {
    MachineRan,
    BlockPlaced,
    SpellCast,
    BossSlain,
    ItemCrafted,
    ItemEnchanted,
}

impl PathEvent {
    fn path(self) -> Path {
        match self {
            PathEvent::MachineRan => Path::Engineer,
            PathEvent::BlockPlaced => Path::Architect,
            PathEvent::SpellCast | PathEvent::BossSlain => Path::Battlemage,
            PathEvent::ItemCrafted | PathEvent::ItemEnchanted => Path::Artisan,
        }
    }

    fn weight(self) -> u32 {
        match self {
            PathEvent::MachineRan | PathEvent::BlockPlaced
            | PathEvent::SpellCast | PathEvent::ItemCrafted => 1,
            PathEvent::ItemEnchanted => 3,
            PathEvent::BossSlain => 10,
        }
    }
}

/// Points per standing tier.
pub const TIER_STEP: u32 = 25;
/// The respec sink: pay this, standings reset, future gains for the
/// chosen focus accrue double.
pub const RESPEC_COST: [(&'static str, u8); 2] = [("iron_ingot", 8), ("null_shard", 1)];

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Paths {
    #[serde(default)]
    points: [u32; 4],
    /// The post-respec focus: its accrual doubles until the next respec.
    #[serde(default)]
    pub focus: Option<Path>,
}

impl Paths {
    pub fn standing(&self, path: Path) -> u32 {
        self.points[path as usize]
    }

    pub fn tier(&self, path: Path) -> u32 {
        self.standing(path) / TIER_STEP
    }

    /// Accrue an event. Returns the tier that was CROSSED (milestones
    /// for the chronicle), if any.
    pub fn accrue(&mut self, event: PathEvent) -> Option<(Path, u32)> {
        let path = event.path();
        let gain = event.weight()
            * if self.focus == Some(path) { 2 } else { 1 };
        let before = self.tier(path);
        self.points[path as usize] = self.points[path as usize].saturating_add(gain);
        let after = self.tier(path);
        (after > before).then_some((path, after))
    }

    /// Pay the sink, reset standings, focus future gains. The caller
    /// consumes the cost; this only checks the cooldown-free state.
    pub fn respec(&mut self, focus: Path) {
        self.points = [0, 0, 0, 0];
        self.focus = Some(focus);
    }
}

/// The generalized gate (P37 doc 08): recipes are gated by era OR by
/// path standing. `Open` = no gate.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Gate {
    Era(crate::research::Era),
    Path(Path, u32),
    Open,
}

impl Gate {
    /// The unlock verdict against the live research + standings.
    pub fn passes(self, research: &crate::research::ResearchState, paths: &Paths) -> bool {
        match self {
            Gate::Era(e) => research.unlocked(e),
            Gate::Path(p, standing) => paths.standing(p) >= standing,
            Gate::Open => true,
        }
    }

    /// What to show the locked player.
    pub fn label(self) -> String {
        match self {
            Gate::Era(e) => format!("needs {}", e.name()),
            Gate::Path(p, s) => format!("needs {} {} standing", p.name(), s),
            Gate::Open => String::new(),
        }
    }
}

/// The full gate table: era gates (via Era::required_for, now honoring
/// branch eras through unlocked()) + the professional-tier ornate path
/// gates (P37 doc 08: 1-2 per path).
pub fn gate_for(item_id: &str) -> Gate {
    match item_id {
        "precision_gear" => Gate::Path(Path::Engineer, TIER_STEP),
        "master_blueprint" => Gate::Path(Path::Architect, TIER_STEP),
        "battlestaff" => Gate::Path(Path::Battlemage, TIER_STEP),
        "master_chisel" => Gate::Path(Path::Artisan, TIER_STEP),
        other => Gate::Era(crate::research::Era::required_for(other)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn events_accrue_to_the_right_paths_and_tiers() {
        let mut p = Paths::default();
        assert_eq!(p.accrue(PathEvent::MachineRan), None, "one tick is not a tier");
        for _ in 0..23 {
            p.accrue(PathEvent::MachineRan);
        }
        assert_eq!(p.accrue(PathEvent::MachineRan), Some((Path::Engineer, 1)), "25 ticks cross tier 1");
        assert_eq!(p.standing(Path::Engineer), 25);
        assert_eq!(p.standing(Path::Artisan), 0, "no cross-pollution");
        assert_eq!(p.accrue(PathEvent::BossSlain), None, "a boss is 10 — not yet a tier");
        assert_eq!(p.accrue(PathEvent::ItemEnchanted), None, "3 < 25");
        for _ in 0..3 {
            p.accrue(PathEvent::BossSlain);
        }
        assert!(p.tier(Path::Battlemage) >= 1, "repeated boss kills cross the tier");
    }

    #[test]
    fn respec_redirects_future_gains() {
        let mut p = Paths::default();
        for _ in 0..30 {
            p.accrue(PathEvent::BlockPlaced);
        }
        assert_eq!(p.standing(Path::Architect), 30);
        p.respec(Path::Engineer);
        assert_eq!(p.standing(Path::Architect), 0, "the sink resets standings");
        assert_eq!(p.accrue(PathEvent::MachineRan), None, "2x but 25 needed");
        assert_eq!(p.standing(Path::Engineer), 2, "focus doubles the gain");
        assert_eq!(p.standing(Path::Architect), 0, "unfocused paths accrue normally — none here yet");
        p.accrue(PathEvent::BlockPlaced);
        assert_eq!(p.standing(Path::Architect), 1);
    }

    /// The generalized gate covers both kinds and fixes the branch-era
    /// bench bug (Steam items must pass with Steam unlocked, even though
    /// the mainline era is lower).
    #[test]
    fn gates_cover_eras_and_paths() {
        let mut research = crate::research::ResearchState::default();
        let paths = Paths::default();
        assert!(!gate_for("boiler").passes(&research, &paths), "nothing unlocked yet");
        research.era = crate::research::Era::Electrical;
        research.branches.push(crate::research::Era::Steam);
        assert!(gate_for("boiler").passes(&research, &paths),
            "Steam unlocked passes even though the MAINLINE era is Electrical");
        assert!(!gate_for("battlestaff").passes(&research, &paths), "path gates need standing");
        let mut pro = Paths::default();
        for _ in 0..25 {
            pro.accrue(PathEvent::SpellCast);
        }
        assert!(gate_for("battlestaff").passes(&research, &pro));
        assert!(gate_for("stone_pickaxe").passes(&research, &paths), "primitive items are open");
    }
}
