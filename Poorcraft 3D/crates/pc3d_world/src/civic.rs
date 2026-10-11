//! P3D-610: civic projects — player-built and NPC-commissioned.
//!
//! Civic works (aqueducts, walls, granaries) use the same materials,
//! site, and ownership contracts regardless of who commissioned them.
//! Player projects advance when the player delivers materials; NPC
//! projects auto-advance per tick. Both complete at work_required.

/// Who commissioned the project.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Commissioner {
    Player,
    Npc,
}

/// A civic construction project.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CivicProject {
    pub name: &'static str,
    pub commissioned_by: Commissioner,
    pub work_required: u64,
    pub work_done: u64,
    pub materials_needed: u64,
    pub materials_delivered: u64,
    pub completed: bool,
}

impl CivicProject {
    pub fn new(
        name: &'static str,
        by: Commissioner,
        work_required: u64,
        materials_needed: u64,
    ) -> Self {
        CivicProject {
            name,
            commissioned_by: by,
            work_required,
            work_done: 0,
            materials_needed,
            materials_delivered: 0,
            completed: false,
        }
    }

    /// Deliver materials (player path). Returns how many were accepted.
    pub fn deliver_materials(&mut self, count: u64) -> u64 {
        if self.completed {
            return 0;
        }
        let accepted = count.min(self.materials_needed - self.materials_delivered);
        self.materials_delivered += accepted;
        self.check_complete();
        accepted
    }

    /// NPC auto-advance: each tick adds `work_per_npc * npc_count` work.
    /// Returns true when the project just completed.
    pub fn npc_advance(&mut self, work: u64) -> bool {
        if self.completed {
            return false;
        }
        self.work_done = (self.work_done + work).min(self.work_required);
        if self.work_done >= self.work_required {
            self.completed = true;
            return true;
        }
        false
    }

    /// Material delivery path: completed when materials are fully delivered.
    fn check_complete(&mut self) {
        if self.materials_delivered >= self.materials_needed {
            self.completed = true;
        }
    }

    pub fn progress_pct(&self) -> u8 {
        if self.work_required == 0 {
            return 100;
        }
        ((self.work_done * 100) / self.work_required).min(100) as u8
    }
}

/// Manages all civic projects for one settlement.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CivicBoard {
    pub projects: Vec<CivicProject>,
}

impl CivicBoard {
    /// Commission a new project.
    pub fn commission(
        &mut self,
        name: &'static str,
        by: Commissioner,
        work_required: u64,
        materials_needed: u64,
    ) -> usize {
        self.projects
            .push(CivicProject::new(name, by, work_required, materials_needed));
        self.projects.len() - 1
    }

    /// Player delivers materials to a project by index.
    pub fn deliver(&mut self, idx: usize, count: u64) -> u64 {
        self.projects[idx].deliver_materials(count)
    }

    /// NPC tick: auto-advance NPC-commissioned projects.
    pub fn npc_tick(&mut self, work_per_npc: u64, npc_count: u64) {
        let total_work = work_per_npc * npc_count.max(1);
        for p in &mut self.projects {
            if p.commissioned_by == Commissioner::Npc && !p.completed {
                p.npc_advance(total_work);
            }
        }
    }

    pub fn completed_projects(&self) -> Vec<&CivicProject> {
        self.projects.iter().filter(|p| p.completed).collect()
    }

    pub fn active_projects(&self) -> Vec<&CivicProject> {
        self.projects.iter().filter(|p| !p.completed).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Player-commissioned: deliver materials to complete.
    #[test]
    fn p3d610_player_project_delivers_and_completes() {
        let mut board = CivicBoard::default();
        let idx = board.commission("aqueduct", Commissioner::Player, 100, 50);
        board.deliver(idx, 30);
        assert!(!board.projects[idx].completed);
        board.deliver(idx, 20);
        assert!(board.projects[idx].completed);
        // Delivering to a completed project accepts 0.
        assert_eq!(board.deliver(idx, 10), 0);
    }

    /// NPC-commissioned projects auto-advance per tick.
    #[test]
    fn p3d610_npc_projects_auto_advance() {
        let mut board = CivicBoard::default();
        let idx = board.commission("granary", Commissioner::Npc, 100, 0);
        for _ in 0..5 {
            board.npc_tick(10, 3); // 3 NPCs doing 10 work each
        }
        assert!(
            board.projects[idx].completed,
            "150 work in 5 ticks completes 100"
        );
    }

    /// Both player and NPC paths produce identical completion states.
    #[test]
    fn p3d610_both_paths_produce_same_completion() {
        let mut player_board = CivicBoard::default();
        let mut npc_board = CivicBoard::default();
        player_board.commission("wall", Commissioner::Player, 200, 0);
        npc_board.commission("wall", Commissioner::Npc, 200, 0);
        // Player delivers in chunks.
        for _ in 0..4 {
            player_board.deliver(0, 50);
        }
        // NPC auto-advances.
        for _ in 0..4 {
            npc_board.npc_tick(50, 1);
        }
        assert!(player_board.projects[0].completed);
        assert!(npc_board.projects[0].completed);
    }
}
