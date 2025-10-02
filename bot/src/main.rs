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

use crate::moderation::purge::UserRecentIndex;
use crate::moderation::trap_event_handler;
use crate::moderation::{burst_event_handler, BurstState};
use poise::serenity_prelude as serenity;
use poise::serenity_prelude::Settings;

struct Data {
    recent: UserRecentIndex,
    burst: BurstState,
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
                    // 1) Record all messages into the rolling index once
                    if let serenity::FullEvent::Message { new_message } = event.clone() {
                        data.recent.record(&new_message).await;
                    }

                    // 2) Run trap logic (reply → delete → purge via index)
                    trap_event_handler(ctx, event, &data.recent).await?;

                    // 3) Run burst detection (guild-wide, no heuristics beyond timing/spread)
                    burst_event_handler(ctx, event, &data.burst, &data.recent).await?;

                    Ok::<(), Error>(())
                })
            },
            ..Default::default()
        })
        .setup(|_ctx, _ready, _framework| {
            Box::pin(async move {
                let recent = UserRecentIndex::new(180); // keep slightly > purge window
                let burst = BurstState::new();
                Ok(Data { recent, burst })
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
