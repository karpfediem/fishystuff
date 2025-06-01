use crate::{Context, Error};
use rand::prelude::IndexedRandom;
use rand::Rng;

const PHRASES: &[&str] = &[
    "Eep! Don’t poke the Crio, silly land-walker!",
    "Poke me again and I’ll splash ya, squeak!",
    "You got some nerve, aye? I *like* it!",
    "Is this how you greet fine otters? Rude!",
    "Oof! That tickled my whiskers, squeak!",
    "Hands off the fur, it's maintenance day!",
    "Oi! I'm workin' here, or at least pretendin'!",
    "That’s Crio harassment, that is!",
    "You'll get fined for pokin’ Crio, y’know?",
    "Crio not happy. Crio considering revolution.",
    "Touch me again and I’ll turn you into fish bait!",
    "Poke me one more time and I’ll make you chum!",
    "A sea otter’s patience is deep… but not endless!",
    "I’ve wrestled sea monsters nicer than you!",
    "Even the waves got more manners!",
    "Don’t make me slap you with a mackerel!",
    "You wanna go fish-wrangling with yer nose?",
    "Careful, landlubber. I bite *and* splash.",
    "Fishin’ for trouble, eh?",
    "I’ll tell the tides on you!",
    "Crio is a very important otter. Show respect!",
    "I am *Crio the Unpoked*! Or I *was*...",
    "Do you *know* who I am?! No? Fair.",
    "Pokin’ Crio is a declaration of war!",
    "I’ll report you to the Otter Council!",
    "This is why I prefer seagulls… mostly.",
    "Bananafish pudding! Wait, what?",
    "If I explode, it’s your fault.",
    "Crio feels a great disturbance… in his fur.",
    "Do pokes count as blessings? I forget!",
    "You’ve awakened the forbidden itch!",
    "I smell... treachery. Or herring.",
    "Stop or I’ll summon the ancient barnacle!",
    "I once saw a starfish do a dance. True story!",
    "You poke Crio, Crio poke destiny!",
    "Three pokes and I turn into a tuna. Probably.",
    "How bored *are* you, adventurer?",
    "There’s a whole war goin’ on and *this* is your priority?",
    "Bet you poke all the NPCs. Freak.",
    "Is this your idea of endgame content?",
    "Why don't you poke some *rocks*, huh?",
    "You should be ashamed. But I'm kinda impressed.",
    "Ten pokes unlocks a secret fish! (Or was it 15...?)",
    "Poking NPCs: the *true* Black Desert experience.",
    "You better not be streaming this!",
    "Crio will remember this.",
];

fn respond() -> String {
    let mut rng = rand::rng();

    match rng.random::<f64>() < 0.35 {
        true => String::from("Qweek!"),
        false => PHRASES.choose(&mut rng).unwrap_or(&"Qweek!").to_string(),
    }
}

/// Poke Crio, see what happens
#[poise::command(slash_command, prefix_command)]
pub async fn poke(ctx: Context<'_>) -> Result<(), Error> {
    ctx.say(respond()).await?;
    Ok(())
}
