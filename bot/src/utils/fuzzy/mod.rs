use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use poise::futures_util::Stream;
use std::cmp::Reverse;

/// Normalize a string for comparison (lowercase, trimmed)
fn normalize(s: &str) -> String {
    s.trim().to_lowercase()
}

pub fn gen_autocomplete<T>(input: &str, options: T) -> impl Iterator<Item = String>
where
    T: IntoIterator,
    T::Item: AsRef<str>,
{
    let matcher = SkimMatcherV2::default();
    let input_normalized = normalize(input);

    // Collect all option names with their scores
    let mut scored_options: Vec<(String, i64)> = options
        .into_iter()
        .filter_map(|o| {
            matcher
                .fuzzy_match(&normalize(o.as_ref()), &input_normalized)
                .map(|score| (o.as_ref().to_string(), score))
        })
        .collect();

    // Sort by descending score and take top 10
    scored_options.sort_by_key(|&(_, score)| Reverse(score));
    scored_options.into_iter().take(10).map(|(o, _)| o)
}
