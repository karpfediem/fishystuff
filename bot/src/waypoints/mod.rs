use std::cmp::Reverse;
use crate::waypoints::zone_names::ZONE_NAMES;
use crate::{Context, Error};
use futures::{stream, StreamExt};
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use poise::futures_util::Stream;
use poise::serenity_prelude::{CreateActionRow, CreateButton, CreateEmbed};
use std::fs;
use std::path::{Path, PathBuf};

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

async fn autocomplete_fuzzy_zone<'a>(
    _ctx: Context<'_>,
    input: &'a str,
) -> impl Stream<Item = String> + 'a {
    let matcher = SkimMatcherV2::default();
    let input_normalized = normalize(input);

    // Collect all zone names with their scores
    let mut scored_zones: Vec<(String, i64)> = ZONE_NAMES
        .iter()
        .filter_map(|&zone| {
            matcher
                .fuzzy_match(&normalize(zone), &input_normalized)
                .map(|score| (zone.to_string(), score))
        })
        .collect();

    // Sort by descending score and take top 10
    scored_zones.sort_by_key(|&(_, score)| Reverse(score));
    let top_matches = scored_zones.into_iter().take(10).map(|(zone, _)| zone);

    // Return a stream over the top matches
    stream::iter(top_matches)
}

async fn autocomplete_zone<'a>(
    _ctx: Context<'_>,
    partial: &'a str,
) -> impl Stream<Item = String> + 'a {
    futures::stream::iter(ZONE_NAMES)
        .filter(move |name| futures::future::ready(name.starts_with(partial)))
        .map(|name| name.to_string())
}

const BASE_URL: &str =
    "https://github.com/Flockenberger/bdo-fish-waypoints/raw/refs/heads/main/Bookmark/";

fn validate_path(user_path: &Path, base_path: &Path) -> Result<PathBuf, Error> {
    let base = base_path
        .canonicalize()
        .map_err(|e| format!("Base path error: {}", e))?;
    let user = user_path
        .canonicalize()
        .map_err(|e| format!("User path error: {}", e))?;
    if !base.exists() || !user.exists() {
        return Err("Directories don't exist!".into());
    }

    // Check that the full path starts with the base path
    if user.starts_with(&base) {
        Ok(user)
    } else {
        Err("Access denied: path traversal attempt detected.".into())
    }
}

/// Show waypoint preview and XML data for a given zone (fuzzy-matched)
#[poise::command(prefix_command, slash_command)]
pub async fn waypoints(
    ctx: Context<'_>,
    #[description = "Zone Name"]
    #[autocomplete = "autocomplete_fuzzy_zone"]
    zone: String,
) -> Result<(), Error> {
    if !ZONE_NAMES.contains(&&*zone) {
        return Err(format!("{}? Never heard of it. Qweek!", zone).into());
    }

    let base_path = Path::new("./bdo-fish-waypoints/Bookmark");
    let zone_dir = base_path.join(&zone);
    let mut xml_path = zone_dir.join(format!("{zone}.xml"));
    xml_path = validate_path(&xml_path, base_path).map_err(|_| "Could not load XML data!")?;

    // Load XML
    let xml_content =
        fs::read_to_string(&xml_path).unwrap_or_else(|_| "<Failed to read XML>".to_string());

    let zone_encoded = urlencoding::encode(zone.as_str());
    let thumb_url = format!("{}{}/Preview.webp", BASE_URL.to_string(), zone_encoded);

    ctx.send(
        poise::CreateReply::default()
            .embed(
                CreateEmbed::new()
                    .thumbnail(thumb_url)
                    .title(zone.clone()).description("### Usage\n\
                     - If you are unfamiliar with how to use waypoints please check out the [**Tutorial**](https://youtu.be/W-bWmKdv8K8)\n\
                     - Click the **Thumbnail Image** to see a detailed preview of this Zone 🔍 \n\
                     - Your local bookmark file is located under `Documents\\Black Desert\\UserCache\\<Your User ID>\\gamevariable.xml`\n\
                     ")
                    .field("Waypoints XML", format!("```xml\n{}```", xml_content), false),
            )
            .components(vec![CreateActionRow::Buttons(vec![
                CreateButton::new_link("https://youtu.be/W-bWmKdv8K8".to_string())
                    .emoji('❔')
                    .label("Tutorial"),
            ])]),
    )
    .await?;

    Ok(())
}
