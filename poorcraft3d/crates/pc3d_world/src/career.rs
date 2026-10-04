//! The career fork (BETA-0.2 W3.1 — journey step 7, the declared
//! next_task): two deep paths as a player-facing choice at the world's
//! forge. ENGINEERING rules the machine line (cogs, boilers, mills);
//! MYSTERIES rule the ley line (sigils, wards, the mana it opens).
//! Thin but REAL in Beta 0.2: each path = a gated recipe family + one
//! signature craft + a live stat (mana) for Mysteries.
//!
//! THE FORK LAWS:
//! - CHOOSE ONCE: a picked career binds (the choice means something;
//!   switching is post-0.2 work with its own price law).
//! - UNLOCK BY PATH: gated recipes refuse without the career — the
//!   catalog check names the missing path, never a vague refusal.
//! - BOTH PATHS STAY HYBRIDIZABLE LATER (the constitution: the choice
//!   unlocks decisions, it does not wall the player off).

use pc3d_core::SeedStreams;

/// The two Beta 0.2 careers. `code` is the stable on-disk byte.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Career {
    Engineering,
    Mysteries,
}

impl Career {
    pub fn code(self) -> u8 {
        match self {
            Career::Engineering => 1,
            Career::Mysteries => 2,
        }
    }

    pub fn from_code(c: u8) -> Option<Self> {
        match c {
            1 => Some(Career::Engineering),
            2 => Some(Career::Mysteries),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Career::Engineering => "Engineering",
            Career::Mysteries => "Mysteries",
        }
    }

    /// The flavor line the panel shows (player language, per the laws).
    pub fn promise(self) -> &'static str {
        match self {
            Career::Engineering => "MACHINES ANSWER TO YOU: COGS, BOILERS, MILLS",
            Career::Mysteries => "THE LEY ANSWERS TO YOU: SIGILS, WARDS, MANA",
        }
    }
}

/// The chosen-path state. Persists in the session (pc3d_save).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CareerState {
    pub chosen: Option<Career>,
}

impl CareerState {
    /// THE CHOOSE-ONCE LAW: the first choice binds and returns true; a
    /// re-choice of the SAME career is an idempotent true (the panel
    /// may re-confirm); SWITCHING refuses and returns false (the fork
    /// means something — a switch law arrives post-0.2 with its price).
    pub fn choose(&mut self, c: Career) -> bool {
        match self.chosen {
            None => {
                self.chosen = Some(c);
                true
            }
            Some(existing) => existing == c,
        }
    }

    /// The recipes this state unlocks (empty before the fork).
    pub fn unlocked_recipes(&self) -> &'static [u16] {
        match self.chosen {
            Some(Career::Engineering) => ENGINEERING_RECIPES,
            Some(Career::Mysteries) => MYSTERIES_RECIPES,
            None => &[],
        }
    }

    /// THE UNLOCK LAW: a gated recipe is craftable only with its path.
    /// Ungated recipes are everyone's.
    pub fn unlocks(&self, recipe_code: u16) -> bool {
        if GATED_RECIPES.contains(&recipe_code) {
            return self.unlocked_recipes().contains(&recipe_code);
        }
        true
    }

    /// The refusal line for a gated, locked recipe (player language).
    pub fn lock_line(&self, recipe_code: u16) -> Option<&'static str> {
        if self.unlocks(recipe_code) {
            return None;
        }
        match recipe_for_career(recipe_code) {
            Some(Career::Engineering) => Some("REQUIRES THE ENGINEERING PATH (X TO CHOOSE)"),
            Some(Career::Mysteries) => Some("REQUIRES THE MYSTERIES PATH (X TO CHOOSE)"),
            None => Some("REQUIRES A PATH (X TO CHOOSE)"),
        }
    }

    /// Mysteries opens the mana stat (S06): the reserved bar goes live.
    pub fn mana_live(&self) -> bool {
        self.chosen == Some(Career::Mysteries)
    }
}

/// The gated recipe codes (the catalog's path families).
pub const ENGINEERING_RECIPES: &[u16] = &[7, 8];
pub const MYSTERIES_RECIPES: &[u16] = &[9, 10];
/// Every gated code, for the unlock law's membership check.
pub const GATED_RECIPES: &[u16] = &[7, 8, 9, 10];

/// Which career a gated recipe belongs to.
pub fn recipe_for_career(recipe_code: u16) -> Option<Career> {
    if ENGINEERING_RECIPES.contains(&recipe_code) {
        Some(Career::Engineering)
    } else if MYSTERIES_RECIPES.contains(&recipe_code) {
        Some(Career::Mysteries)
    } else {
        None
    }
}

/// The seed-derived hint the panel shows BEFORE the choice: which path
/// the world leans toward near the player's spawn (a deterministic
/// flavor nudge — rivers and machine sites lean Engineering, ley pockets
/// lean Mysteries). Pure per (seed, region).
pub fn world_lean(gen: &crate::gen::WorldGen, region: crate::coords::RegionCoord) -> Career {
    let s = SeedStreams::new(gen.seed()).stream_seed("career_lean");
    let river_near = crate::hydro::RiverGraph::new(gen, 4)
        .discharge(region)
        .max(1);
    // Hash the region into a 0..1 tilt, then let real rivers tip it.
    let mut h = s ^ ((region.x as u64) << 32) ^ region.z as u64;
    h ^= h >> 33;
    h = h.wrapping_mul(0xff51afd7ed558ccd);
    h ^= h >> 33;
    let tilt = (h % 100) as i64 + (river_near.min(8) as i64 * 4);
    if tilt >= 50 {
        Career::Engineering
    } else {
        Career::Mysteries
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// THE CHOOSE-ONCE LAW: first choice binds, same-career re-choice is
    /// idempotent, switching refuses without erasing the original.
    #[test]
    fn beta02_career_choose_once_binds() {
        let mut st = CareerState::default();
        assert!(st.chosen.is_none());
        assert!(st.choose(Career::Engineering));
        assert_eq!(st.chosen, Some(Career::Engineering));
        // Same career: idempotent (the panel may re-confirm).
        assert!(st.choose(Career::Engineering));
        assert_eq!(st.chosen, Some(Career::Engineering));
        // Switching refuses AND does not erase.
        assert!(!st.choose(Career::Mysteries));
        assert_eq!(st.chosen, Some(Career::Engineering));
    }

    /// THE UNLOCK LAW: gated recipes refuse without their path (with a
    /// named refusal), unlock with it, and ungated recipes are free.
    #[test]
    fn beta02_career_unlocks_gate_by_path() {
        let mut st = CareerState::default();
        assert!(!st.unlocks(7), "gated before any choice");
        assert!(st.lock_line(7).is_some());
        assert!(st.unlocks(1), "ungated recipes are everyone's");
        st.choose(Career::Engineering);
        assert!(st.unlocks(7) && st.unlocks(8));
        assert!(!st.unlocks(9) && !st.unlocks(10), "the other path stays locked");
        assert!(st.lock_line(9).unwrap().contains("MYSTERIES"));
        assert!(!st.mana_live(), "mana is Mysteries' stat");
        let mut m = CareerState::default();
        m.choose(Career::Mysteries);
        assert!(m.unlocks(9) && m.unlocks(10));
        assert!(!m.unlocks(7));
        assert!(m.mana_live());
    }

    /// The wire law: career codes round-trip and unknown codes refuse.
    #[test]
    fn beta02_career_codes_round_trip() {
        for c in [Career::Engineering, Career::Mysteries] {
            assert_eq!(Career::from_code(c.code()), Some(c));
        }
        assert!(Career::from_code(0).is_none());
        assert!(Career::from_code(3).is_none());
    }

    /// The lean law: deterministic per (seed, region), rivers pull
    /// toward Engineering.
    #[test]
    fn beta02_career_world_lean_is_deterministic() {
        let gen = crate::gen::WorldGen::new(4242);
        let a = world_lean(&gen, crate::coords::RegionCoord { x: 0, z: 0 });
        let b = world_lean(&gen, crate::coords::RegionCoord { x: 0, z: 0 });
        assert_eq!(a, b, "the lean must replay");
        // Across a band both answers occur (a one-sided lean is a bias).
        let mut engineering = 0;
        for x in -8..=8 {
            for z in -8..=8 {
                if matches!(
                    world_lean(&gen, crate::coords::RegionCoord { x, z }),
                    Career::Engineering
                ) {
                    engineering += 1;
                }
            }
        }
        assert!(engineering > 40 && engineering < 300, "lean collapsed: {engineering}/289");
    }
}
