use crate::core::models::ThemeWeights;

pub fn get_theme_preset(system_raw: &str) -> ThemeWeights {
    let system = system_raw.to_lowercase();

    // Default weights (all 0.0 except the primary which is decided below)
    let mut weights = ThemeWeights::default();

    // Exact or partial matches
    if system.contains("d&d") || system.contains("dnd") || system.contains("5e") || system.contains("pathfinder") {
        weights.fantasy = 1.0;
        // Reset others (ThemeWeights::default sets fantasy=1.0)
    } else if system.contains("call of cthulhu") || system.contains("coc") || system.contains("vaesen") {
        weights.fantasy = 0.0;
        weights.cosmic = 1.0;
    } else if system.contains("kult") {
        weights.fantasy = 0.0;
        weights.cosmic = 0.8;
        weights.noir = 0.2;
    } else if system.contains("delta green") {
        weights.fantasy = 0.0;
        weights.noir = 0.6;
        weights.cosmic = 0.4;
    } else if system.contains("night's black agents") || system.contains("nba") {
        weights.fantasy = 0.0;
        weights.noir = 0.8;
        weights.terminal = 0.2;
    } else if system.contains("mothership") || system.contains("alien") {
        weights.fantasy = 0.0;
        weights.terminal = 1.0;
    } else if system.contains("traveller") || system.contains("stars without number") || system.contains("swn") {
        weights.fantasy = 0.0;
        weights.terminal = 0.9;
        weights.neon = 0.1;
    } else if system.contains("cyberpunk") || system.contains("the sprawl") {
        weights.fantasy = 0.0;
        weights.neon = 1.0;
    } else if system.contains("shadowrun") {
        weights.fantasy = 0.2;
        weights.neon = 0.8;
    } else if system.contains("blade runner") {
        weights.fantasy = 0.0;
        weights.neon = 0.6;
        weights.noir = 0.4;
    } else {
        // Default persistence of fantasy = 1.0 from default()
    }

    weights
}
