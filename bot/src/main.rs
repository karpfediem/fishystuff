use poise::serenity_prelude as serenity;

struct Data {} // User data, which is stored and accessible in all command invocations
type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

/// Displays your or another user's account creation date
#[poise::command(slash_command, prefix_command)]
async fn age(
    ctx: Context<'_>,
    #[description = "Selected user"] user: Option<serenity::User>,
) -> Result<(), Error> {
    let u = user.as_ref().unwrap_or_else(|| ctx.author());
    let response = format!("{}'s account was created at {}", u.name, u.created_at());
    ctx.say(response).await?;
    Ok(())
}

/// A command with two subcommands: `child1` and `child2`
///
/// Running this function directly, without any subcommand, is only supported in prefix commands.
/// Discord doesn't permit invoking the root command of a slash command if it has subcommands.
#[poise::command(prefix_command, slash_command, subcommands("help_drr", "help_mystical"))]
pub async fn help(ctx: Context<'_>) -> Result<(), Error> {
    ctx.say("What can I help you with? *qweek*").await?;
    Ok(())
}

/// A subcommand of `help`
#[poise::command(rename = "drr", prefix_command, slash_command)]
pub async fn help_drr(ctx: Context<'_>) -> Result<(), Error> {
    ctx.say("DRR").await?;
    Ok(())
}

/// Another subcommand of `help`
#[poise::command(rename = "mystical", prefix_command, slash_command)]
pub async fn help_mystical(ctx: Context<'_>) -> Result<(), Error> {
    ctx.say("Stinky Fish").await?;
    Ok(())
}
#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let token = std::env::var("DISCORD_TOKEN").expect("missing DISCORD_TOKEN");
    let intents =
        serenity::GatewayIntents::non_privileged();

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![age(), help()],
            ..Default::default()
        })
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                Ok(Data {})
            })
        })
        .build();

    let client = serenity::ClientBuilder::new(token, intents)
        .framework(framework)
        .await;
    client.unwrap().start().await.unwrap();
}
