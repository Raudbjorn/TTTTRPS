use crate::bindings::get_timeline_summary;

pub async fn generate_session_summary(session_id: String) -> Result<String, String> {
    let summary = get_timeline_summary(session_id.clone()).await?;

    let mut narrative = String::new();
    narrative.push_str(&format!(
        "Session Summary (Duration: {} minutes, {} events)

",
        summary.duration_minutes, summary.total_events
    ));

    if !summary.key_moments.is_empty() {
        narrative.push_str(
            "KEY MOMENTS:
",
        );
        for moment in &summary.key_moments {
            narrative.push_str(&format!(
                "- [{:?}] {} - {}
",
                moment.severity, moment.title, moment.description
            ));
        }
        narrative.push('\n');
    }

    if summary.combat.encounters > 0 {
        narrative.push_str(&format!(
            "COMBAT: {} encounter(s), {} total rounds",
            summary.combat.encounters, summary.combat.total_rounds
        ));
        if let Some(damage) = summary.combat.damage_dealt {
            narrative.push_str(&format!(", {} damage dealt", damage));
        }
        if let Some(healing) = summary.combat.healing_done {
            narrative.push_str(&format!(", {} healing done", healing));
        }
        if summary.combat.deaths > 0 {
            narrative.push_str(&format!(", {} death(s)", summary.combat.deaths));
        }
        narrative.push_str(
            "

",
        );
    }

    if !summary.npcs_encountered.is_empty() {
        narrative.push_str("NPCs ENCOUNTERED: ");
        let names: Vec<&str> = summary
            .npcs_encountered
            .iter()
            .map(|n| n.name.as_str())
            .collect();
        narrative.push_str(&names.join(", "));
        narrative.push_str(
            "

",
        );
    }

    if !summary.locations_visited.is_empty() {
        narrative.push_str("LOCATIONS VISITED: ");
        let names: Vec<&str> = summary
            .locations_visited
            .iter()
            .map(|l| l.name.as_str())
            .collect();
        narrative.push_str(&names.join(", "));
        narrative.push_str(
            "

",
        );
    }

    if !summary.items_acquired.is_empty() {
        narrative.push_str("ITEMS ACQUIRED: ");
        narrative.push_str(&summary.items_acquired.join(", "));
        narrative.push_str(
            "

",
        );
    }

    Ok(narrative)
}
