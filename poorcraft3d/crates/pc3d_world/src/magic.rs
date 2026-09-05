//! P3D-504: the first magic path — learnable runes, mana, world-facing
//! casts.
//!
//! A rune is LEARNED (progression), cast for MANA (a regenerating pool
//! on the fixed clock), and produces a world-facing EFFECT as an edit
//! plan (cells to mark/clear) composed by callers through the P3D-204
//! edit path. Deterministic: costs, regen, and effects are pure.

use crate::coords::CellCoord;
use std::collections::BTreeSet;

/// The learnable runes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Rune {
    /// Places a light: marks the target cell + the 6 neighbors as lit.
    Lumen,
    /// Clears a 3×3×1 disc at the target cell (earth-magic dig).
    Delve,
}

impl Rune {
    pub fn mana_cost(self) -> i32 {
        match self {
            Rune::Lumen => 10,
            Rune::Delve => 18,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Rune::Lumen => "lumen",
            Rune::Delve => "delve",
        }
    }
}

/// The mage's mana pool: regenerates per tick on the fixed clock.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Mana {
    pub current: i32,
    pub max: i32,
}

pub const MANA_REGEN_PER_TICK: i32 = 1;
pub const MANA_MAX: i32 = 100;

impl Default for Mana {
    fn default() -> Self {
        Mana { current: MANA_MAX, max: MANA_MAX }
    }
}

impl Mana {
    /// Regen one tick (capped at max). Casting this same tick BLOCKS
    /// regen — call `spend` after `tick`.
    pub fn tick(&mut self) {
        self.current = (self.current + MANA_REGEN_PER_TICK).min(self.max);
    }

    /// Deduct mana; returns false (unchanged) when insufficient.
    pub fn spend(&mut self, cost: i32) -> bool {
        if self.current >= cost {
            self.current -= cost;
            true
        } else {
            false
        }
    }
}

/// The effect a cast produces, as cells the caller applies through the
/// edit path.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CastEffect {
    /// Cells to turn into light markers.
    Light(Vec<CellCoord>),
    /// Cells to clear.
    Dig(Vec<CellCoord>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CastError {
    NotLearned,
    NotEnoughMana,
}

/// The mage: learned runes + mana pool.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Mage {
    learned: BTreeSet<Rune>,
    pub mana: Mana,
}

impl Mage {
    pub fn new() -> Self {
        Mage { learned: BTreeSet::new(), mana: Mana::default() }
    }

    /// Learn a rune (progression; idempotent).
    pub fn learn(&mut self, rune: Rune) -> bool {
        self.learned.insert(rune)
    }

    pub fn knows(&self, rune: Rune) -> bool {
        self.learned.contains(&rune)
    }

    pub fn tick(&mut self) {
        self.mana.tick();
    }

    /// Cast a learned rune at a target cell: deducts mana and returns the
    /// effect plan. Refuses unlearned runes and insufficient mana WITHOUT
    /// changing anything.
    pub fn cast(&mut self, rune: Rune, target: CellCoord) -> Result<CastEffect, CastError> {
        if !self.knows(rune) {
            return Err(CastError::NotLearned);
        }
        if !self.mana.spend(rune.mana_cost()) {
            return Err(CastError::NotEnoughMana);
        }
        Ok(match rune {
            Rune::Lumen => CastEffect::Light({
                let (x, y, z) = (target.x, target.y, target.z);
                vec![
                    target,
                    CellCoord { x: x + 1, y, z },
                    CellCoord { x: x - 1, y, z },
                    CellCoord { x, y: y + 1, z },
                    CellCoord { x, y: y - 1, z },
                    CellCoord { x, y, z: z + 1 },
                    CellCoord { x, y, z: z - 1 },
                ]
            }),
            Rune::Delve => CastEffect::Dig({
                let (x, y, z) = (target.x, target.y, target.z);
                (0..3)
                    .map(|dx| {
                        (0..3).map(move |dz| CellCoord { x: x + dx - 1, y, z: z + dz - 1 })
                    })
                    .flatten()
                    .collect()
            }),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Learning gates casting: an unlearned rune refuses and nothing
    /// (mana included) changes.
    #[test]
    fn p3d504_unlearned_runes_refuse_without_side_effects() {
        let mut mage = Mage::new();
        let mana_before = mage.mana.current;
        let target = CellCoord { x: 5, y: 5, z: 5 };
        assert_eq!(mage.cast(Rune::Lumen, target), Err(CastError::NotLearned));
        assert_eq!(mage.mana.current, mana_before, "failed cast must not drain mana");
        mage.learn(Rune::Lumen);
        assert!(mage.knows(Rune::Lumen));
        assert!(mage.cast(Rune::Lumen, target).is_ok());
    }

    /// Casting deducts the rune's mana cost; mana regenerates per tick
    /// capped at max; casting on an empty pool refuses.
    #[test]
    fn p3d504_mana_is_spent_and_regenerates() {
        let mut mage = Mage::new();
        mage.learn(Rune::Lumen);
        mage.learn(Rune::Delve);
        mage.mana.current = 20;
        mage.tick();
        assert_eq!(mage.mana.current, 21, "regen +1/tick");
        // Lumen costs 10: castable.
        assert!(mage.cast(Rune::Lumen, CellCoord::default()).is_ok());
        assert_eq!(mage.mana.current, 11);
        // Delve costs 18: insufficient now.
        assert_eq!(
            mage.cast(Rune::Delve, CellCoord::default()),
            Err(CastError::NotEnoughMana)
        );
        assert_eq!(mage.mana.current, 11, "refused cast must not drain");
    }

    /// Effects are world-facing: Lumen marks the target + 6 neighbors;
    /// Delve marks the 3×3 disc.
    #[test]
    fn p3d504_effects_cover_the_right_cells() {
        let mut mage = Mage::new();
        mage.learn(Rune::Lumen);
        mage.learn(Rune::Delve);
        let t = CellCoord { x: 10, y: 5, z: 10 };

        let CastEffect::Light(cells) = mage.cast(Rune::Lumen, t).unwrap() else {
            panic!("lumen must yield light");
        };
        assert_eq!(cells.len(), 7, "target + 6 neighbors");
        assert!(cells.contains(&t));
        assert!(cells.contains(&CellCoord { x: 11, y: 5, z: 10 }));

        let CastEffect::Dig(cells) = mage.cast(Rune::Delve, t).unwrap() else {
            panic!("delve must yield dig");
        };
        assert_eq!(cells.len(), 9, "3x3 disc");
        assert!(cells.contains(&t));
        assert!(cells.contains(&CellCoord { x: 9, y: 5, z: 11 }));
    }

    /// Determinism: same learned set + mana → identical cast results.
    #[test]
    fn p3d504_casts_are_deterministic() {
        let build = || {
            let mut m = Mage::new();
            m.learn(Rune::Lumen);
            m.learn(Rune::Delve);
            m.mana.current = 50;
            m
        };
        let mut a = build();
        let mut b = build();
        let t = CellCoord { x: 3, y: 4, z: 5 };
        let la = a.cast(Rune::Delve, t);
        let lb = b.cast(Rune::Delve, t);
        assert_eq!(la, lb);
        assert_eq!(a.mana, b.mana);
    }
}
