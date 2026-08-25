use serde::{Serialize, Deserialize};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToolMaterial {
    Wood,
    Flint,
    Copper,
    Bronze,
    Iron,
    Steel,
    Mythril,
    Adamantine,
}

impl ToolMaterial {
    pub fn harvest_level(self) -> u8 {
        match self {
            ToolMaterial::Wood => 0,
            ToolMaterial::Flint => 1,
            ToolMaterial::Copper => 1,
            ToolMaterial::Bronze => 2,
            ToolMaterial::Iron => 2,
            ToolMaterial::Steel => 3,
            ToolMaterial::Mythril => 4,
            ToolMaterial::Adamantine => 5,
        }
    }

    pub fn speed(self) -> f32 {
        match self {
            ToolMaterial::Wood => 1.0,
            ToolMaterial::Flint => 1.2,
            ToolMaterial::Copper => 1.4,
            ToolMaterial::Bronze => 1.6,
            ToolMaterial::Iron => 1.8,
            ToolMaterial::Steel => 2.2,
            ToolMaterial::Mythril => 3.0,
            ToolMaterial::Adamantine => 2.6,
        }
    }

    pub fn durability(self) -> u32 {
        match self {
            ToolMaterial::Wood => 60,
            ToolMaterial::Flint => 90,
            ToolMaterial::Copper => 180,
            ToolMaterial::Bronze => 320,
            ToolMaterial::Iron => 500,
            ToolMaterial::Steel => 900,
            ToolMaterial::Mythril => 1400,
            ToolMaterial::Adamantine => 2400,
        }
    }

    pub fn damage(self) -> f32 {
        match self {
            ToolMaterial::Wood => 0.0,
            ToolMaterial::Flint => 0.5,
            ToolMaterial::Copper => 1.0,
            ToolMaterial::Bronze => 1.5,
            ToolMaterial::Iron => 2.0,
            ToolMaterial::Steel => 3.0,
            ToolMaterial::Mythril => 4.0,
            ToolMaterial::Adamantine => 5.0,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PartType {
    Head,
    Haft,
    Binding,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolPart {
    pub part_type: PartType,
    pub material: ToolMaterial,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CustomTool {
    pub name: String,
    pub head: ToolPart,
    pub haft: ToolPart,
    pub binding: ToolPart,
    pub durability: u32,
    pub max_durability: u32,
    pub mining_speed: f32,
    pub attack_damage: f32,
    pub rune: Option<String>,
}

impl CustomTool {
    pub fn assemble(name: &str, head: ToolMaterial, haft: ToolMaterial, binding: ToolMaterial) -> Self {
        let durability = (head.durability() + haft.durability() + binding.durability()) / 3;
        let mining_speed = (head.speed() * 0.6) + (haft.speed() * 0.3) + (binding.speed() * 0.1);
        let attack_damage = head.damage() + haft.damage() * 0.2;
        Self {
            name: name.to_string(),
            head: ToolPart { part_type: PartType::Head, material: head },
            haft: ToolPart { part_type: PartType::Haft, material: haft },
            binding: ToolPart { part_type: PartType::Binding, material: binding },
            durability,
            max_durability: durability,
            mining_speed,
            attack_damage,
            rune: None,
        }
    }
}

pub struct ForgeMinigame {
    pub temperature: f32, // 0..100, target orange zone 60..80
    pub strikes_completed: u32,
    pub target_strikes: u32,
}

impl ForgeMinigame {
    pub fn new(target_strikes: u32) -> Self {
        Self { temperature: 50.0, strikes_completed: 0, target_strikes }
    }

    pub fn bellows(&mut self, amount: f32) {
        self.temperature = (self.temperature + amount).clamp(0.0, 100.0);
    }

    pub fn strike(&mut self) -> bool {
        if (60.0..=80.0).contains(&self.temperature) {
            self.strikes_completed += 1;
        }
        self.strikes_completed >= self.target_strikes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_material_stats() {
        assert_eq!(ToolMaterial::Wood.harvest_level(), 0);
        assert_eq!(ToolMaterial::Adamantine.harvest_level(), 5);
        assert!(ToolMaterial::Mythril.speed() > ToolMaterial::Wood.speed());
    }

    #[test]
    fn test_tool_assembly() {
        let pick = CustomTool::assemble("Mythril Pickaxe", ToolMaterial::Mythril, ToolMaterial::Iron, ToolMaterial::Bronze);
        assert!(pick.mining_speed > 2.0);
        assert_eq!(pick.max_durability, (1400 + 500 + 320) / 3);
    }

    #[test]
    fn test_forge_minigame() {
        let mut forge = ForgeMinigame::new(3);
        forge.bellows(25.0); // temp 75.0 (orange zone)
        assert!(!forge.strike());
        assert!(!forge.strike());
        assert!(forge.strike());
        assert_eq!(forge.strikes_completed, 3);
    }
}
