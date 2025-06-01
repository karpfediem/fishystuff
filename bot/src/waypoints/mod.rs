use crate::waypoints::zone_names::ZONE_NAMES;
use crate::{Context, Error};
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use poise::serenity_prelude::CreateAttachment;
use std::fs;
use std::path::Path;

pub mod list;
mod zone_names;

/// Normalize a string for comparison (lowercase, trimmed)
fn normalize(s: &str) -> String {
    s.trim().to_lowercase()
}

/// Find the closest matching zone name from input
fn find_closest_zone(input: &str) -> Option<String> {
    let matcher = SkimMatcherV2::default();
    let input_normalized = normalize(input);

    ZONE_NAMES
        .iter()
        .filter_map(|&zone| {
            matcher
                .fuzzy_match(&normalize(zone), &input_normalized)
                .map(|score| (zone, score))
        })
        .max_by_key(|&(_, score)| score)
        .map(|(zone, _)| zone.to_string())
}

/// Show waypoint preview and XML data for a given zone (fuzzy-matched)
#[poise::command(prefix_command, slash_command)]
pub async fn waypoints(
    ctx: Context<'_>,
    #[description = "Zone name (partial, case-insensitive)"] search: String,
) -> Result<(), Error> {
    let Some(matched_zone) = find_closest_zone(&search) else {
        ctx.say(format!("No such zone matching `{}`", search))
            .await?;
        return Ok(());
    };

    let base_path = Path::new("./bdo-fish-waypoints/Bookmark");
    let zone_dir = base_path.join(&matched_zone);
    let xml_path = zone_dir.join(format!("{matched_zone}.xml"));
    let image_path = zone_dir.join("Preview.webp");

    if !xml_path.exists() || !image_path.exists() {
        ctx.say(format!(
            "Zone `{}` found, but required files are missing.",
            matched_zone
        ))
        .await?;
        return Ok(());
    }

    // Load XML
    let xml_content =
        fs::read_to_string(&xml_path).unwrap_or_else(|_| "<Failed to read XML>".to_string());

    let attachment = CreateAttachment::path(image_path).await?;
    // Send both as one message
    ctx.send(
        poise::CreateReply::default()
            .content(format!(
                "**Matched Zone: `{}`**\n```xml\n{}```",
                matched_zone, xml_content
            ))
            .attachment(attachment),
    )
    .await?;

    Ok(())
}
