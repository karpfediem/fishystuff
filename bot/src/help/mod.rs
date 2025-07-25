use crate::utils::fuzzy::gen_autocomplete;
use crate::{Context, Error};
use futures::{stream, Stream};
use std::str::FromStr;
use poise::serenity_prelude::{Builder, CreateInteractionResponseFollowup, CreateMessage};
use strum::{Display, EnumIter, EnumString, IntoEnumIterator};

#[derive(EnumIter, EnumString, Display, Debug, Eq, PartialEq, Hash)]
enum HelpTopic {
    #[strum(serialize = "Mystical Fish")]
    Mystical,
    #[strum(serialize = "Durability Reduction Resistance (DRR)")]
    DRR,
    #[strum(serialize = "DRR Pets")]
    DRRPets,
    #[strum(serialize = "Experience")]
    Experience,
    #[strum(serialize = "Gear")]
    Gear,
    #[strum(serialize = "Map")]
    Map,
    #[strum(serialize = "Money")]
    Money,
    #[strum(serialize = "Waypoints")]
    Waypoints,
    #[strum(serialize = "Fishing Zones")]
    Zones,
}

async fn autocomplete_fuzzy_help<'a>(
    _ctx: Context<'_>,
    input: &'a str,
) -> impl Stream<Item = String> + 'a {
    let options: Vec<String> = HelpTopic::iter().map(|key| key.to_string()).collect();
    stream::iter(gen_autocomplete(input, options))
}

/// Quick answers to some Frequently Asked Questions
#[poise::command(prefix_command, slash_command)]
pub async fn help(
    ctx: Context<'_>,
    #[description = "Topic"]
    #[autocomplete = "autocomplete_fuzzy_help"]
    topic: String,
) -> Result<(), Error> {
    match HelpTopic::from_str(topic.as_str()) {
        Ok(topic) => match topic {
            HelpTopic::Mystical => help_mystical(ctx).await,
            HelpTopic::DRR => help_drr(ctx).await,
            HelpTopic::DRRPets => help_drr_pets(ctx).await,
            HelpTopic::Experience => help_experience(ctx).await,
            HelpTopic::Gear => help_gear(ctx).await,
            HelpTopic::Map => help_map(ctx).await,
            HelpTopic::Waypoints => help_waypoints(ctx).await,
            HelpTopic::Money => help_money(ctx).await,
            HelpTopic::Zones => help_zones(ctx).await,
        },
        Err(_e) => help_reject(ctx).await,
    }?;
    Ok(())
}

async fn help_reject(ctx: Context<'_>) -> Result<(), Error> {
    ctx.send(poise::CreateReply::default().content("Qweek! I can't help you with that."))
        .await?;
    Ok(())
}

async fn help_mystical(ctx: Context<'_>) -> Result<(), Error> {
    ctx.send(poise::CreateReply::default().content(
        "https://discord.com/channels/161861855332139008/1377294298869137530/1377294298869137530",
    ))
    .await?;
    Ok(())
}

async fn help_drr(ctx: Context<'_>) -> Result<(), Error> {
    ctx.send(poise::CreateReply::default().content(
        "https://discord.com/channels/161861855332139008/1378355406966886421/1378355406966886421",
    ))
    .await?;
    Ok(())
}

async fn help_drr_pets(ctx: Context<'_>) -> Result<(), Error> {
    ctx.send(poise::CreateReply::default().content(
        "https://cdn.discordapp.com/attachments/1377390705499967548/1378362466991472701/image.png?ex=683ef694&is=683da514&hm=3f65efaa84d170b39fa5f0caa63044e544557198053e41aadcdc27852324939f&\n\
        https://discord.com/channels/161861855332139008/1377390705499967548/1377390705499967548",
    )).await?;
    Ok(())
}

async fn help_experience(ctx: Context<'_>) -> Result<(), Error> {
    ctx.send(poise::CreateReply::default().content(
        "https://discord.com/channels/161861855332139008/1377294657234669598/1377294657234669598",
    ))
    .await?;
    Ok(())
}

async fn help_gear(ctx: Context<'_>) -> Result<(), Error> {
    ctx.send(poise::CreateReply::default().content(
        "https://discord.com/channels/161861855332139008/1378397306394644490/1378397306394644490\n\
        https://discord.com/channels/161861855332139008/1378333117072408628/1378333117072408628",
    ))
    .await?;
    Ok(())
}
async fn help_map(ctx: Context<'_>) -> Result<(), Error> {
    ctx.send(poise::CreateReply::default().content(
        "https://discord.com/channels/161861855332139008/1377401205088583690/1377401205088583690",
    ))
    .await?;
    Ok(())
}

async fn help_waypoints(ctx: Context<'_>) -> Result<(), Error> {
    ctx.send(poise::CreateReply::default().content(
        "https://discord.com/channels/161861855332139008/1377387923326107819/1377387923326107819",
    ))
    .await?;
    Ok(())
}

async fn help_money(ctx: Context<'_>) -> Result<(), Error> {
    ctx.send(poise::CreateReply::default().content(
        "https://discord.com/channels/161861855332139008/1378361389030178836/1378361389030178836",
    ))
    .await?;
    Ok(())
}

async fn help_zones(ctx: Context<'_>) -> Result<(), Error> {
    ctx.send(poise::CreateReply::default().content(
        "https://discord.com/channels/161861855332139008/1378819661763711136/1378819661763711136",
    ))
    .await?;
    Ok(())
}
