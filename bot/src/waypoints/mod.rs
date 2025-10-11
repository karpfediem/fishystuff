use crate::utils::fuzzy::gen_autocomplete;
use crate::zones::fish_names::FISH_NAMES;
use crate::zones::zone_names::ZONE_NAMES;
use crate::{Context, Error};
use futures::stream;
use poise::futures_util::Stream;
use poise::serenity_prelude::{CreateActionRow, CreateButton, CreateEmbed};
use std::fs;
use std::path::{Path, PathBuf};

async fn autocomplete_fuzzy_zone<'a>(
    _ctx: Context<'_>,
    input: &'a str,
) -> impl Stream<Item = String> + 'a {
    stream::iter(gen_autocomplete(input, ZONE_NAMES))
}

async fn autocomplete_fuzzy_fish<'a>(
    _ctx: Context<'_>,
    input: &'a str,
) -> impl Stream<Item = String> + 'a {
    stream::iter(gen_autocomplete(input, FISH_NAMES))
}

const BASE_URL: &str =
    "https://github.com/Flockenberger/bdo-fish-waypoints/raw/refs/heads/main/";

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

/// Show waypoint preview and XML data for a given zone
#[poise::command(prefix_command, slash_command)]
pub async fn waypoints(
    ctx: Context<'_>,
    #[description = "Zone Name"]
    #[autocomplete = "autocomplete_fuzzy_zone"]
    zone: String,
) -> Result<(), Error> {
    let mut zone = zone;
    if !ZONE_NAMES.contains(&&*zone) {
        // If user didn't use the autocomplete, try to match again
        if let Some(matching) = gen_autocomplete(&zone, ZONE_NAMES).next() {
            zone = matching;
        } else {
            return Err(format!("{}? Never heard of it. Qweek!", zone).into());
        }
    }

    let base_path = Path::new("./bdo-fish-waypoints/Bookmark");
    let zone_dir = base_path.join(&zone);
    let mut xml_path = zone_dir.join(format!("{zone}.xml"));
    xml_path = validate_path(&xml_path, base_path).map_err(|_| "Could not load XML data!")?;

    // Load XML
    let xml_content =
        fs::read_to_string(&xml_path).unwrap_or_else(|_| "<Failed to read XML>".to_string());

    let zone_encoded = urlencoding::encode(zone.as_str());
    let thumb_url = format!("{}Bookmark/{}/Preview.webp", BASE_URL.to_string(), zone_encoded);

    ctx.send(
        poise::CreateReply::default()
            .embed(
                CreateEmbed::new()
                    .thumbnail(thumb_url.clone())
                    .title(zone.clone()).description(format!("### Usage\n\
                     - If you are unfamiliar with how to use waypoints please check out the [**Tutorial**](https://youtu.be/W-bWmKdv8K8)\n\
                     - Click the [**Thumbnail Image**]({}) to see a detailed preview of this Zone 🔍 \n\
                     - Your local bookmark file is located under `Documents\\Black Desert\\UserCache\\<Your User ID>\\gamevariable.xml`\n\
                     ", thumb_url))
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

/// Show waypoint preview and XML data for a given fish
#[poise::command(prefix_command, slash_command)]
pub async fn fish(
    ctx: Context<'_>,
    #[description = "Fish Name"]
    #[autocomplete = "autocomplete_fuzzy_fish"]
    fish: String,
) -> Result<(), Error> {
    let names = FISH_NAMES;
    let mut name = fish;
    if !names.contains(&&*name) {
        // If user didn't use the autocomplete, try to match again
        if let Some(matching) = gen_autocomplete(&name, names).next() {
            name = matching;
        } else {
            return Err(format!("{}? Never heard of it. Qweek!", name).into());
        }
    }

    let base_path = Path::new("./bdo-fish-waypoints/FishBookmark");
    let zone_dir = base_path.join(&name);
    let mut xml_path = zone_dir.join(format!("{name}.xml"));
    xml_path = validate_path(&xml_path, base_path).map_err(|_| "Could not load XML data!")?;

    // Load XML
    let xml_content =
        fs::read_to_string(&xml_path).unwrap_or_else(|_| "<Failed to read XML>".to_string());

    let encoded = urlencoding::encode(name.as_str());
    let thumb_url = format!("{}FishBookmark/{}/{}_0_Preview.webp", BASE_URL.to_string(), encoded, name);

    ctx.send(
        poise::CreateReply::default()
            .embed(
                CreateEmbed::new()
                    .thumbnail(thumb_url.clone())
                    .title(name.clone()).description(format!("### Usage\n\
                     - If you are unfamiliar with how to use waypoints please check out the [**Tutorial**](https://youtu.be/W-bWmKdv8K8)\n\
                     - Click the [**Thumbnail Image**]({}) to see a detailed preview of this Zone 🔍 \n\
                     - Your local bookmark file is located under `Documents\\Black Desert\\UserCache\\<Your User ID>\\gamevariable.xml`\n\
                     ", thumb_url))
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
