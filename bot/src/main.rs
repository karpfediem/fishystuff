//! main.rs

use crate::moderation::index::RecentIndex;
use std::sync::Arc;
mod help;
mod paginate;
mod poke;
mod talk;
mod utils;
mod waypoints;
mod zones;

mod moderation;

use crate::help::help;
use crate::poke::poke;
use crate::talk::talk;
use crate::waypoints::waypoints;
use crate::zones::list::zones;

use poise::serenity_prelude as serenity;
use poise::serenity_prelude::Settings;

use crate::moderation::actions::notify::{ModInfoConfig, SerenityPoster};
use crate::moderation::actions::purge::SerenityPurger;
use crate::moderation::actions::ModeratorActions;
use crate::moderation::handler::burst_guard::BurstState;
use crate::moderation::handler::{burst_event_handler, trap_event_handler};
use crate::moderation::index::DashRecentIndex;

struct Data {
    /// Rolling, in-memory message index (storage & lookup only).
    index: Arc<DashRecentIndex>,
    /// Cooldown/config state for the burst guard.
    burst: BurstState,
    /// Orchestrates: summary → thread → evidence → purge (side-effects).
    actions: ModeratorActions,
}

type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let token = std::env::var("DISCORD_TOKEN").expect("missing DISCORD_TOKEN");

    let intents = serenity::GatewayIntents::non_privileged()
        | serenity::GatewayIntents::GUILDS
        | serenity::GatewayIntents::GUILD_MEMBERS
        | serenity::GatewayIntents::MESSAGE_CONTENT;

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![help(), waypoints(), zones(), poke(), talk()],
            event_handler: |ctx, event, _framework, data| {
                Box::pin(async move {
                    // 1) Record all incoming messages once into the rolling index.
                    if let serenity::FullEvent::Message { new_message } = event.clone() {
                        data.index.record(&new_message).await;
                    }

                    // Trap handler: kick/timeout + reply + summary→thread→evidence→purge
                    // (Trap does not need index reads, only actions.)
                    trap_event_handler(ctx, event, &data.actions).await?;

                    // Burst guard: detection via index reads; enforcement via actions
                    burst_event_handler(ctx, event, &data.burst, &*data.index, &data.actions)
                        .await?;

                    Ok::<(), Error>(())
                })
            },
            ..Default::default()
        })
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                // Register current slash commands
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                
                let index = Arc::new(DashRecentIndex::new(180)); // shared
                let burst = BurstState::new();

                let mod_info_id = std::env::var("MOD_INFO_CHANNEL_ID")
                    .ok()
                    .and_then(|s| s.parse::<u64>().ok())
                    .unwrap_or(211092999835222017);
                let poster = SerenityPoster::new(ModInfoConfig::new(mod_info_id));
                let purger = SerenityPurger::new();

                let actions = ModeratorActions::new(Arc::clone(&index), poster, purger);


                Ok(Data {
                    index,
                    burst,
                    actions,
                })
            })
        })
        .build();

    let mut cache_settings = Settings::default();
    cache_settings.max_messages = 10;

    let client = serenity::ClientBuilder::new(token, intents)
        .framework(framework)
        .cache_settings(cache_settings)
        .await;

    client.unwrap().start().await.unwrap();
}
