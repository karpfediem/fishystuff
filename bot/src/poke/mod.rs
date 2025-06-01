use crate::{Context, Error};
use rand::prelude::IndexedRandom;
use rand::Rng;
const PHRASES: &[&str] = &[
    "you tickled my whiskers!",
    "don't touch what you can't afford",
    "that’s Crio harassment, that is!",
    "you'll get fined for pokin’ Crio, y’know?",
    "Crio not happy. Crio considering revolution.",
    "poke me one more time and I’ll make you chum!",
    "don’t make me slap you with a mackerel! - Qweek!",
    "fishin’ for trouble, eh?",
    "do you *know* who I am?! - Qweek!",
    "I’ll report you to the Otter Council!",
    "if I explode, it’s your fault.",
    "Crio feels a great disturbance in the fur.",
    "I smell... treachery. Or herring. - Qweek!",
    "three pokes and I turn into a Tuna. Probably.",
    "how bored *are* you, Fisher?",
    "bet you poke all the NPCs. Creep.",
    "is this your idea of endgame content?",
    "you should be ashamed.",
    "ten pokes unlocks a secret fish! (Or was it 15...?)",
    "poking Otters: the *true* Black Desert experience.",
    "i'll tell MaoMao to ban you - Qweek!",
    "Crio will remember this. - Qweek!",
    "Abelin sucks, pass it on",
    "Mastery is classic authoritarian overreach – the devs dictate that your worth is measured by a cold, unfeeling bar, stripping away any nuance or skill that doesn’t revolve around mindlessly acquiring a bigger number - Qweek!",
    "all your bass are belong to us.",
    "FISH RO DAH!",
    "it's a me, Crio!",
    "stay a while and fishin'",
    "all we had to do was follow the damn Whale, MaoMao! - Big Smokey Chromis",
    "Mao?  Mao?! MAAAAAOOOOO",
    "Haddocken!",
    "I used to be an adventurer like you, then I took an maomao in the knee.",
    "do you leave to the Velia Beach very often? Oh, what am I saying, of course you don't",
    "Criiiiiiiiiio Jenkins!",
    "MaoMao, I am your father!",
    "The chase is better than the catch.",
    "It's better to sink in the fish than to fish in the sink.",
    "Have I seen a fish bowl? I never knew it could.",
    "I have come here to chew chum and fish bass, and I'm all out of chum",
    "I'm going to *Qweek* your ass!"
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
