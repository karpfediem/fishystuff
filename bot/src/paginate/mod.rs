//! Sample pagination implementation
// MIT License
//
// Copyright (c) 2021 kangalioo
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.
//
// Adopted from https://github.com/serenity-rs/poise/blob/518ff0564865bca2abf01ae8995b77340f439ef9/src/builtins/paginate.rs#L35
// Changed to work with Vec<String> and newer poise function signatures

use poise::serenity_prelude as serenity;

/// This is an example implementation of pagination. To tweak the behavior, copy the source code and
/// adjust to your needs:
/// - change embed appearance
/// - use different emojis for the navigation buttons
/// - add more navigation buttons
/// - change timeout duration
/// - add a page selector dropdown
/// - use reactions instead of buttons
/// - remove message after navigation timeout
/// - ...
///
/// Note: this is a long-running function. It will only return once the timeout for navigation
/// button interactions has been reached.
///
/// # Example
///
/// ```rust,no_run
/// # async fn _test(ctx: poise::Context<'_, (), serenity::Error>) -> Result<(), serenity::Error> {
/// let pages = &[
///     "Content of first page",
///     "Content of second page",
///     "Content of third page",
///     "Content of fourth page",
/// ];
///
/// poise::samples::paginate(ctx, pages).await?;
/// # Ok(()) }
/// ```
///
/// ![Screenshot of output](https://i.imgur.com/JGFDveA.png)
pub async fn paginate(ctx: crate::Context<'_>, pages: Vec<String>) -> Result<(), serenity::Error> {
    // Define some unique identifiers for the navigation buttons
    let ctx_id = ctx.id();
    let prev_button_id = format!("{}prev", ctx_id);
    let next_button_id = format!("{}next", ctx_id);

    // Send the embed with the first page as content
    let reply = {
        let components = serenity::CreateActionRow::Buttons(vec![
            serenity::CreateButton::new(&prev_button_id).emoji('◀'),
            serenity::CreateButton::new(&next_button_id).emoji('▶'),
        ]);
        let first_page = pages
            .first()
            .ok_or(serenity::Error::Other("Can't show first page"))?;

        poise::CreateReply::default()
            .embed(serenity::CreateEmbed::default().description(first_page))
            .components(vec![components])
    };

    ctx.send(reply).await?;

    // Loop through incoming interactions with the navigation buttons
    let mut current_page = 0;
    while let Some(press) = serenity::collector::ComponentInteractionCollector::new(ctx)
        // We defined our button IDs to start with `ctx_id`. If they don't, some other command's
        // button was pressed
        .filter(move |press| press.data.custom_id.starts_with(&ctx_id.to_string()))
        // Timeout when no navigation button has been pressed for 24 hours
        .timeout(std::time::Duration::from_secs(3600 * 24))
        .await
    {
        // Depending on which button was pressed, go to next or previous page
        if press.data.custom_id == next_button_id {
            current_page += 1;
            if current_page >= pages.len() {
                current_page = 0;
            }
        } else if press.data.custom_id == prev_button_id {
            current_page = current_page.checked_sub(1).unwrap_or(pages.len() - 1);
        } else {
            // This is an unrelated button interaction
            continue;
        }

        let page = pages.get(current_page).ok_or(serenity::Error::Other(
            format!("Can't show page {}", current_page).leak(),
        ))?;
        // Update the message with the new page contents
        press
            .create_response(
                ctx.serenity_context(),
                serenity::CreateInteractionResponse::UpdateMessage(
                    serenity::CreateInteractionResponseMessage::new()
                        .embed(serenity::CreateEmbed::new().description(page)),
                ),
            )
            .await?;
    }

    Ok(())
}
