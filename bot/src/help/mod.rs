use crate::utils::fuzzy::gen_autocomplete;
use crate::{Context, Error};
use futures::{stream, Stream};
use std::str::FromStr;
use strum::{Display, EnumIter, EnumString, IntoEnumIterator};

#[derive(EnumIter, EnumString, Display, Debug, Eq, PartialEq, Hash)]
enum HelpTopic {
    #[strum(serialize = "Mystical Fish")]
    Mystical,
    #[strum(serialize = "Durability Reduction Resistance (DRR)")]
    DRR,
    #[strum(serialize = "Experience")]
    Experience,
    #[strum(serialize = "Money")]
    Money,
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
            _ => help_reject(ctx).await,
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
    ctx.send(poise::CreateReply::default().content("Mystical Fish! :)"))
        .await?;
    Ok(())
}
