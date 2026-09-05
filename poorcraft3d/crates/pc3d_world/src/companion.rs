//! P3D-406: companion follow/wait/assist/recovery.
//!
//! A companion trails the player: FOLLOW keeps it within
//! [`FOLLOW_DISTANCE`] by pathing behind the player whenever the player's
//! cell changes (paths recompute from the companion's ACTUAL position, so
//! being left behind self-heals without teleports); WAIT holds;
//! ASSIST paths to a target cell and holds there. Path caching keyed by
//! (target cell) avoids re-pathing every tick.

use crate::coords::CellCoord;
use crate::nav::NavPatch;

/// How close the companion tries to stay behind the player (cells).
pub const FOLLOW_DISTANCE: i32 = 2;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompanionCommand {
    Follow,
    Wait,
    Assist,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Companion {
    pub entity: crate::entities::EntityId,
    pub pos: CellCoord,
    pub command: CompanionCommand,
    /// Cached path and current leg.
    path: Vec<CellCoord>,
    leg: usize,
    /// The player cell the cached path was computed for.
    path_for: Option<CellCoord>,
    /// The assist target when in Assist mode.
    assist_target: Option<CellCoord>,
}

impl Companion {
    pub fn new(entity: crate::entities::EntityId, pos: CellCoord) -> Self {
        Companion {
            entity,
            pos,
            command: CompanionCommand::Follow,
            path: Vec::new(),
            leg: 0,
            path_for: None,
            assist_target: None,
        }
    }

    pub fn set_command(&mut self, cmd: CompanionCommand, assist_target: Option<CellCoord>) {
        self.command = cmd;
        self.assist_target = assist_target;
        self.path.clear();
        self.leg = 0;
        self.path_for = None;
    }

    /// Arrival check: x/z only (y is terrain height).
    pub fn at_target(&self, target: CellCoord) -> bool {
        self.pos.x == target.x && self.pos.z == target.z
    }

    pub fn path_remaining(&self) -> usize {
        self.path.len().saturating_sub(self.leg)
    }

    /// One deterministic tick on the companion's nav patch.
    pub fn step(&mut self, nav: &NavPatch, player_cell: CellCoord) {
        match self.command {
            CompanionCommand::Wait => return, // hold position, ignore paths
            CompanionCommand::Assist => {
                let Some(target) = self.assist_target else { return };
                if self.pos == target {
                    return; // holding the assist position
                }
                self.ensure_path(nav, target);
            }
            CompanionCommand::Follow => {
                // Stay within FOLLOW_DISTANCE: if the player moved to a new
                // cell (or the path is exhausted and we're still far),
                // recompute a path to a trailing cell behind the player.
                let dx = (self.pos.x - player_cell.x).abs();
                let dz = (self.pos.z - player_cell.z).abs();
                let cheb = dx.max(dz);
                let path_stale = self.path_for != Some(player_cell);
                if cheb > FOLLOW_DISTANCE && (path_stale || self.leg >= self.path.len()) {
                    let trailing = CellCoord {
                        x: player_cell.x - (player_cell.x - self.pos.x).signum(),
                        y: 0,
                        z: player_cell.z - (player_cell.z - self.pos.z).signum(),
                    };
                    self.ensure_path(nav, trailing);
                    self.path_for = Some(player_cell);
                }
            }
        }
        // Consume one path leg per tick.
        if self.leg < self.path.len() {
            self.pos = self.path[self.leg];
            self.leg += 1;
        }
    }

    fn ensure_path(&mut self, nav: &NavPatch, target: CellCoord) {
        if self.leg < self.path.len() {
            return; // cached path still valid
        }
        if let Some(path) = nav.path(self.pos, target) {
            if path.len() >= 2 {
                self.path = path[1..].to_vec(); // skip the current cell
            } else {
                self.path = path;
            }
            self.leg = 0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gen::WorldGen;
    use crate::coords::PatchCoord;
    use crate::entities::EntityId;

    fn nav_hills() -> NavPatch {
        // A NavPatch containing the origin cells the tests move through.
        let gen = WorldGen::new(3);
        NavPatch::from_gen(&gen, PatchCoord { x: 0, y: 0, z: 0 })
    }

    fn eid() -> EntityId {
        EntityId(1)
    }

    /// Follow: the player walks away; the companion trails within
    /// FOLLOW_DISTANCE + path slack once the player stops, WITHOUT
    /// teleporting (its positions pass through intermediate cells).
    #[test]
    fn p3d406_follow_trails_without_teleport() {
        let nav = nav_hills();
        let mut c = Companion::new(eid(), CellCoord { x: 0, y: 0, z: 0 });
        assert_eq!(c.command, CompanionCommand::Follow);
        // Player walks away along +x, one cell per tick, for 10 ticks
        // (staying inside the nav patch).
        let mut player = CellCoord { x: 2, y: 0, z: 0 };
        let mut last = c.pos;
        let mut jumped = false;
        for _ in 0..10 {
            player.x += 1;
            c.step(&nav, player);
            let d = (c.pos.x - last.x).abs() + (c.pos.z - last.z).abs();
            if d > 2 {
                jumped = true;
            }
            last = c.pos;
        }
        assert!(!jumped, "companion teleported");
        // Player stops; companion catches up to within distance+slack.
        for _ in 0..60 {
            c.step(&nav, player);
        }
        let cheb = (c.pos.x - player.x).abs().max((c.pos.z - player.z).abs());
        assert!(
            cheb <= FOLLOW_DISTANCE + 2,
            "companion too far after catching up: {cheb}"
        );
    }

    /// Wait holds position even as the player walks away; switching back
    /// to Follow resumes trailing.
    #[test]
    fn p3d406_wait_holds_and_follow_resumes() {
        let nav = nav_hills();
        let mut c = Companion::new(eid(), CellCoord { x: 0, y: 0, z: 0 });
        c.set_command(CompanionCommand::Wait, None);
        let held = c.pos;
        let mut player = CellCoord { x: 5, y: 0, z: 5 };
        for _ in 0..8 {
            player.x += 1;
            c.step(&nav, player);
        }
        assert_eq!(c.pos, held, "wait must hold position");
        c.set_command(CompanionCommand::Follow, None);
        for _ in 0..120 {
            c.step(&nav, player);
        }
        let cheb = (c.pos.x - player.x).abs().max((c.pos.z - player.z).abs());
        assert!(cheb <= FOLLOW_DISTANCE + 2, "did not resume following");
    }

    /// Assist: pathing to a target cell and holding it.
    #[test]
    fn p3d406_assist_paths_to_target_and_holds() {
        let nav = nav_hills();
        let mut c = Companion::new(eid(), CellCoord { x: 0, y: 0, z: 0 });
        let target = CellCoord { x: 12, y: 0, z: 4 };
        c.set_command(CompanionCommand::Assist, Some(target));
        for _ in 0..80 {
            c.step(&nav, CellCoord { x: 0, y: 0, z: 0 });
        }
        // Arrival compares x/z: y follows the nav terrain heights.
        assert!(
            c.pos.x == target.x && c.pos.z == target.z,
            "assist at {:?}, target {:?}",
            c.pos,
            target
        );
        // Holding: further ticks keep it there.
        for _ in 0..10 {
            c.step(&nav, CellCoord { x: 0, y: 0, z: 0 });
        }
        assert!(c.pos.x == target.x && c.pos.z == target.z);
    }

    /// Determinism: identical command/cell histories produce identical
    /// companion positions.
    #[test]
    fn p3d406_companion_is_deterministic() {
        let nav = nav_hills();
        let mut a = Companion::new(eid(), CellCoord { x: 0, y: 0, z: 0 });
        let mut b = Companion::new(eid(), CellCoord { x: 0, y: 0, z: 0 });
        let mut player = CellCoord { x: 1, y: 0, z: 1 };
        for t in 0..200i32 {
            player.x += if t % 3 == 0 { 1 } else { 0 };
            player.z += if t % 5 == 0 { 1 } else { 0 };
            a.step(&nav, player);
            b.step(&nav, player);
        }
        assert_eq!(a.pos, b.pos);
    }
}
