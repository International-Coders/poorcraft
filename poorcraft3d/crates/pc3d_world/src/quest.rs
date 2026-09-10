//! The quest authority (owner order, 2026-09-10): deterministic, pure
//! quests derived from the world's own settlement data — no rendering,
//! no RNG beyond the world seed. A quest binds a GIVER (an NPC role at
//! a cell), a KIND with concrete parameters, a STATE machine, and a
//! reward; progress events are submitted by the simulation side
//! (`QuestEvent`) and the authority decides advancement.

use crate::coords::{CellCoord, RegionCoord};
use crate::gen::WorldGen;
use crate::npc::Role;
use crate::settlement_plan::SettlementPlan;

/// Quest kinds — each maps to mechanics that actually exist in the sim.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QuestKind {
    /// Visit a cell (a person, a place).
    Visit { target: CellCoord },
    /// Deliver N building blocks to a site (construction economy).
    Deliver { site: CellCoord, blocks: u8 },
    /// Build a structure of at least N blocks at a site.
    Build { site: CellCoord, blocks: u8 },
    /// Greet N distinct NPCs (social fabric).
    Greet { count: u8 },
    /// Clear N solid cells (excavation/labor).
    Excavate { cells: u8 },
}

impl QuestKind {
    pub fn name(self) -> &'static str {
        match self {
            QuestKind::Visit { .. } => "visit",
            QuestKind::Deliver { .. } => "deliver",
            QuestKind::Build { .. } => "build",
            QuestKind::Greet { .. } => "greet",
            QuestKind::Excavate { .. } => "excavate",
        }
    }

    /// The numeric goal (visits/blocks/greets/cells).
    pub fn goal(self) -> u8 {
        match self {
            QuestKind::Visit { .. } => 1,
            QuestKind::Deliver { blocks, .. } => blocks,
            QuestKind::Build { blocks, .. } => blocks,
            QuestKind::Greet { count } => count,
            QuestKind::Excavate { cells } => cells,
        }
    }
}

/// Quest state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QuestState {
    Offered,
    Active,
    /// Done, awaiting the reward claim.
    Complete,
    /// Reward claimed; the quest is finished forever.
    Claimed,
}

/// One quest: identity + giver + kind + progress + reward.
#[derive(Clone, Debug, PartialEq)]
pub struct Quest {
    pub id: u32,
    pub title: String,
    pub giver_role: Role,
    pub giver_cell: CellCoord,
    pub kind: QuestKind,
    pub state: QuestState,
    pub progress: u8,
    /// Reward in the economy's credit unit.
    pub reward: u32,
}

impl Quest {
    /// Submit a progress event; returns the new state. Pure — same
    /// (quest, event) always yields the same result.
    pub fn advance(&self, event: &QuestEvent) -> Quest {
        let mut q = self.clone();
        if q.state != QuestState::Active {
            return q; // only ACTIVE quests take progress
        }
        let hit = match (&q.kind, event) {
            (QuestKind::Visit { target }, QuestEvent::Visited { cell }) => cell == target,
            (QuestKind::Deliver { site, .. }, QuestEvent::Delivered { site: s, blocks }) => {
                s == site && {
                    q.progress = q.progress.saturating_add(*blocks);
                    false // handled via the counter below
                }
            }
            (QuestKind::Build { site, .. }, QuestEvent::Built { site: s, blocks }) => {
                s == site && {
                    q.progress = q.progress.saturating_add(*blocks);
                    false
                }
            }
            (QuestKind::Greet { .. }, QuestEvent::Greeted { .. }) => {
                q.progress = q.progress.saturating_add(1);
                false
            }
            (QuestKind::Excavate { .. }, QuestEvent::Excavated { cells }) => {
                q.progress = q.progress.saturating_add(*cells);
                false
            }
            _ => false,
        };
        if hit {
            q.progress = q.progress.saturating_add(1);
        }
        if q.progress >= q.kind.goal() {
            q.state = QuestState::Complete;
        }
        q
    }

    /// Accept an OFFERED quest (the player's choice).
    pub fn accept(&self) -> Quest {
        let mut q = self.clone();
        if q.state == QuestState::Offered {
            q.state = QuestState::Active;
        }
        q
    }

    /// Claim the reward of a COMPLETE quest.
    pub fn claim(&self) -> Quest {
        let mut q = self.clone();
        if q.state == QuestState::Complete {
            q.state = QuestState::Claimed;
        }
        q
    }
}

/// Simulation-side progress events (submitted by gameplay; the quest
/// authority decides what counts).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum QuestEvent {
    Visited { cell: CellCoord },
    Delivered { site: CellCoord, blocks: u8 },
    Built { site: CellCoord, blocks: u8 },
    Greeted { npc: CellCoord },
    Excavated { cells: u8 },
}

fn fnv(seed: u64, words: [u64; 3]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in seed.to_le_bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    for word in words {
        for b in word.to_le_bytes() {
            h ^= b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
    }
    h
}

fn unit(seed: u64, words: [u64; 3]) -> f32 {
    ((fnv(seed, words) >> 11) as f32) / ((1u64 << 53) as f32)
}

/// Derive the region's quests deterministically from the settlement
/// plan: anchors become sites, NPC roles become givers, the seed picks
/// kinds and goals. 3..=6 quests per region.
pub fn plan_quests(gen: &WorldGen, plan: &SettlementPlan, region: RegionCoord) -> Vec<Quest> {
    let seed = gen.hash_seed() ^ 0x5145_4553_0000_0001;
    let mut quests = Vec::new();
    let work = plan
        .anchors
        .work_cells
        .first()
        .copied()
        .unwrap_or(CellCoord { x: plan.plaza.x + 4, y: 0, z: plan.plaza.z });
    let bed = plan
        .anchors
        .bed_cells
        .first()
        .copied()
        .unwrap_or(CellCoord { x: plan.plaza.x - 4, y: 0, z: plan.plaza.z - 4 });
    let idle = plan
        .anchors
        .idle_cells
        .first()
        .copied()
        .unwrap_or(CellCoord { x: plan.plaza.x + 8, y: 0, z: plan.plaza.z + 8 });
    let homes: Vec<CellCoord> = plan
        .buildings
        .iter()
        .map(|b| b.cell)
        .take(6)
        .collect();
    let n = 3 + (unit(seed, [region.x as u64, region.z as u64, 0x71]) * 4.0) as usize;
    for i in 0..n {
        let roll = unit(seed, [region.x as u64, region.z as u64, 0x72 + i as u64]);
        let (giver_role, giver_cell) = match i % 3 {
            0 => (Role::Farmer, bed),
            1 => (Role::Builder, work),
            _ => (Role::Guard, idle),
        };
        let kind = match (roll * 5.0) as usize {
            0 => {
                let target = if homes.is_empty() {
                    idle
                } else {
                    homes[i.min(homes.len() - 1)]
                };
                QuestKind::Visit { target }
            }
            1 => QuestKind::Deliver { site: work, blocks: 4 + (roll * 8.0) as u8 },
            2 => QuestKind::Build { site: idle, blocks: 6 + (roll * 10.0) as u8 },
            3 => QuestKind::Greet { count: 2 + (roll * 3.0) as u8 },
            _ => QuestKind::Excavate { cells: 3 + (roll * 6.0) as u8 },
        };
        let title = format!("{} {}", giver_role_name(giver_role), kind.name());
        quests.push(Quest {
            id: (region.x as u32).wrapping_mul(1000).wrapping_add(region.z as u32).wrapping_mul(10).wrapping_add(i as u32),
            title,
            giver_role,
            giver_cell,
            kind,
            state: QuestState::Offered,
            progress: 0,
            reward: 10 + (roll * 30.0) as u32,
        });
    }
    quests
}

fn giver_role_name(r: Role) -> &'static str {
    match r {
        Role::Farmer => "THE FARMER'S",
        Role::Builder => "THE BUILDER'S",
        Role::Guard => "THE GUARD'S",
        _ => "THE",
    }
}

#[cfg(test)]
mod quest_authority_tests {
    use super::*;

    fn plan_at(seed: u64, x: i32, z: i32) -> (WorldGen, SettlementPlan) {
        let gen = WorldGen::new(seed);
        let reg = RegionCoord { x, z };
        (gen, SettlementPlan::plan(&gen, reg))
    }

    #[test]
    fn quests_derive_deterministically_from_the_plan() {
        let (g, p) = plan_at(4242, 3, 3);
        let a = plan_quests(&g, &p, RegionCoord { x: 3, z: 3 });
        let b = plan_quests(&g, &p, RegionCoord { x: 3, z: 3 });
        assert_eq!(a, b, "same region + plan -> identical quests");
        assert!((3..=6).contains(&a.len()), "3..=6 quests ({})", a.len());
    }

    #[test]
    fn quest_lifecycle_visits_offer_accept_complete_claim() {
        let (g, p) = plan_at(7, 0, 0);
        let quests = plan_quests(&g, &p, RegionCoord { x: 0, z: 0 });
        let q = quests.iter().find(|q| matches!(q.kind, QuestKind::Visit { .. })).expect("a visit quest");
        let QuestKind::Visit { target } = q.kind else { unreachable!() };
        let active = q.accept();
        assert_eq!(active.state, QuestState::Active);
        let done = active.advance(&QuestEvent::Visited { cell: target });
        assert_eq!(done.state, QuestState::Complete);
        assert_eq!(done.progress, 1);
        let claimed = done.claim();
        assert_eq!(claimed.state, QuestState::Claimed);
        // Claimed quests ignore further events.
        let after = claimed.advance(&QuestEvent::Visited { cell: target });
        assert_eq!(after.state, QuestState::Claimed);
    }

    #[test]
    fn counters_accumulate_and_wrong_sites_do_not_count() {
        let (g, p) = plan_at(9, 1, 1);
        let quests = plan_quests(&g, &p, RegionCoord { x: 1, z: 1 });
        let q = quests.iter().find(|q| matches!(q.kind, QuestKind::Deliver { .. })).expect("a deliver quest");
        let QuestKind::Deliver { site, blocks } = q.kind else { unreachable!() };
        let mut cur = q.accept();
        cur = cur.advance(&QuestEvent::Delivered { site: CellCoord { x: site.x + 99, y: 0, z: site.z }, blocks: 10 });
        assert_eq!(cur.progress, 0, "wrong site does not count");
        cur = cur.advance(&QuestEvent::Delivered { site, blocks: 2 });
        assert_eq!(cur.progress, 2);
        cur = cur.advance(&QuestEvent::Delivered { site, blocks: 2 });
        assert_eq!(cur.progress, 4);
        if blocks <= 4 {
            assert_eq!(cur.state, QuestState::Complete);
        }
    }

    #[test]
    fn offered_quests_take_no_progress() {
        let (g, p) = plan_at(11, 2, 2);
        let quests = plan_quests(&g, &p, RegionCoord { x: 2, z: 2 });
        let q = &quests[0];
        let ev = QuestEvent::Greeted { npc: q.giver_cell };
        let after = q.advance(&ev);
        assert_eq!(after.state, QuestState::Offered, "only ACTIVE quests advance");
    }
}
