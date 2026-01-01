use crate::bindings::ThemeWeights;

pub fn get_dominant_theme(weights: &ThemeWeights) -> String {
    let mut max = weights.fantasy;
    let mut theme = "theme-fantasy";

    if weights.cosmic > max { max = weights.cosmic; theme = "theme-cosmic"; }
    if weights.terminal > max { max = weights.terminal; theme = "theme-terminal"; }
    if weights.noir > max { max = weights.noir; theme = "theme-noir"; }
    if weights.neon > max { max = weights.neon; theme = "theme-neon"; }

    theme.to_string()
}
