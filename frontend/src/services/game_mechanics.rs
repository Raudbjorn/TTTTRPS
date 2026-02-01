
pub const STANDARD_CONDITIONS: &[&str] = &[
    "Blinded", "Charmed", "Deafened", "Exhaustion", "Frightened", "Grappled",
    "Incapacitated", "Invisible", "Paralyzed", "Petrified", "Poisoned",
    "Prone", "Restrained", "Stunned", "Unconscious",
];

pub fn get_condition_description(name: &str) -> Option<&'static str> {
    match name.to_lowercase().as_str() {
        "blinded" => Some("Can't see. Auto-fails sight checks. Attacks have advantage against, disadvantage on attacks."),
        "charmed" => Some("Can't attack charmer. Charmer has advantage on social checks."),
        "deafened" => Some("Can't hear. Auto-fails hearing checks."),
        "exhaustion" => Some("Cumulative levels with increasing penalties."),
        "frightened" => Some("Disadvantage on checks/attacks while fear source visible. Can't move closer."),
        "grappled" => Some("Speed 0. Ends if grappler incapacitated or removed from reach."),
        "incapacitated" => Some("Can't take actions or reactions."),
        "invisible" => Some("Can't be seen. Attacks against have disadvantage, attacks have advantage."),
        "paralyzed" => Some("Incapacitated, can't move/speak. Auto-fail STR/DEX saves. Attacks have advantage, crits in 5ft."),
        "petrified" => Some("Transformed to stone. Incapacitated, resistant to damage, immune to poison/disease."),
        "poisoned" => Some("Disadvantage on attacks and ability checks."),
        "prone" => Some("Can only crawl. Disadvantage on attacks. Advantage/disadvantage based on distance."),
        "restrained" => Some("Speed 0. Attacks against have advantage. Disadvantage on attacks and DEX saves."),
        "stunned" => Some("Incapacitated, can't move. Auto-fail STR/DEX saves. Attacks have advantage."),
        "unconscious" => Some("Incapacitated, drops items, falls prone. Auto-fail STR/DEX. Attacks have advantage, crits in 5ft."),
        _ => None,
    }
}
