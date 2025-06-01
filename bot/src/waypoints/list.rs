use crate::paginate::paginate;
use crate::waypoints::zone_names::ZONE_NAMES;
use crate::{Context, Error};

const ZONES_PER_PAGE: usize = 20;

/// List all the currently known Zone names
#[poise::command(slash_command, prefix_command)]
pub async fn waypoints_list(ctx: Context<'_>) -> Result<(), Error> {
    let pages: Vec<String> = ZONE_NAMES
        .chunks(ZONES_PER_PAGE)
        .map(|chunk| chunk.join("\n"))
        .collect();
    paginate(ctx, pages).await?;

    Ok(())
}
