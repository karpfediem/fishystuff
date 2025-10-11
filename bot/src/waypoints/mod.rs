use crate::utils::fuzzy::gen_autocomplete;
use crate::zones::fish_names::FISH_NAMES;
use crate::zones::zone_names::ZONE_NAMES;
use crate::{Context, Error};
use futures::stream;
use poise::futures_util::Stream;
use poise::serenity_prelude::{CreateActionRow, CreateButton, CreateEmbed};
use poise::CreateReply;
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

const BASE_URL: &str = "https://github.com/Flockenberger/bdo-fish-waypoints/raw/refs/heads/main/";

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
    let names = ZONE_NAMES;
    let mut name = zone;
    let base_path = Path::new("./bdo-fish-waypoints/Bookmark");
    if !names.contains(&&*name) {
        // If user didn't use the autocomplete, try to match again
        if let Some(matching) = gen_autocomplete(&name, names).next() {
            name = matching;
        } else {
            return Err("Never heard of it. Qweek!".into());
        }
    }

    let zone_dir = base_path.join(&name);
    let mut xml_path = zone_dir.join(format!("{name}.xml"));
    xml_path = validate_path(&xml_path, base_path).map_err(|_| "Could not load XML data!")?;

    // Load XML
    let xml_content =
        fs::read_to_string(&xml_path).unwrap_or_else(|_| "<Failed to read XML>".to_string());

    let name_encoded = urlencoding::encode(name.as_str());
    let thumb_url = format!(
        "{}Bookmark/{}/Preview.webp",
        BASE_URL.to_string(),
        name_encoded
    );
    let waypoint_readme_url = format!("{}Bookmark/{}/", BASE_URL.to_string(), name_encoded);

    ctx.send(create_waypoint_reply(
        name,
        xml_content,
        thumb_url,
        waypoint_readme_url,
    ))
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
            return Err("Never heard of it. Qweek!".into());
        }
    }

    let base_path = Path::new("./bdo-fish-waypoints/FishBookmark");
    let zone_dir = base_path.join(&name);
    let mut xml_path = zone_dir.join(format!("{name}.xml"));
    xml_path = validate_path(&xml_path, base_path).map_err(|_| "Could not load XML data!")?;

    // Load XML
    let xml_content =
        fs::read_to_string(&xml_path).unwrap_or_else(|_| "<Failed to read XML>".to_string());

    let name_encoded = urlencoding::encode(name.as_str());
    let thumb_url = format!(
        "{}FishBookmark/{}/{}_0_Preview.webp",
        BASE_URL.to_string(),
        name_encoded,
        name
    );
    let waypoint_readme_url = format!("{}FishBookmark/{}/", BASE_URL.to_string(), name_encoded);

    ctx.send(create_waypoint_reply(
        name,
        xml_content,
        thumb_url,
        waypoint_readme_url,
    ))
    .await?;

    Ok(())
}

fn create_waypoint_reply(
    zone: String,
    xml_content: String,
    thumb_url: String,
    waypoint_readme_url: String,
) -> CreateReply {
    poise::CreateReply::default()
        .embed(
            CreateEmbed::new()
                .thumbnail(thumb_url.clone())
                .title("Usage").description(format!("\
                     - If you are unfamiliar with how to use waypoints please check out the [**Tutorial**](https://youtu.be/W-bWmKdv8K8)\n\
                     - Click the [**Thumbnail Image**]({}) to see a detailed preview 🔍 \n\
                     - Your local bookmark file is located under `Documents\\Black Desert\\UserCache\\<Your User ID>\\gamevariable.xml`\n\
                     ", thumb_url))
        ).content(format!("## {}\n```xml\n{}```", zone, xml_content))
        .components(vec![CreateActionRow::Buttons(vec![
            CreateButton::new_link("https://youtu.be/W-bWmKdv8K8".to_string())
                .emoji('❔')
                .label("Tutorial"),
        ]), CreateActionRow::Buttons(vec![
            CreateButton::new_link(waypoint_readme_url)
                .emoji('📄')
                .label("Waypoint README"),
        ])])
}
