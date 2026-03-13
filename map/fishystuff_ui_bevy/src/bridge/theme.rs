use bevy::color::{Color, Oklaba, Oklcha};

pub fn parse_css_color(value: &str) -> Option<Color> {
    let trimmed = value.trim();
    if let Some(hex) = trimmed.strip_prefix('#') {
        return parse_hex_color(hex);
    }
    parse_rgb_color(trimmed)
        .or_else(|| parse_oklch_color(trimmed))
        .or_else(|| parse_oklab_color(trimmed))
}

fn parse_rgb_color(value: &str) -> Option<Color> {
    let parts = parse_css_function_parts(value, "rgba(")
        .or_else(|| parse_css_function_parts(value, "rgb("))?;
    if parts.len() < 3 {
        return None;
    }
    let r = parse_rgb_channel(parts[0])?;
    let g = parse_rgb_channel(parts[1])?;
    let b = parse_rgb_channel(parts[2])?;
    let a = if parts.len() >= 4 {
        parse_alpha_channel(parts[3])?
    } else {
        1.0
    };
    Some(Color::srgba(
        r as f32 / 255.0,
        g as f32 / 255.0,
        b as f32 / 255.0,
        a,
    ))
}

fn parse_oklch_color(value: &str) -> Option<Color> {
    let parts = parse_css_function_parts(value, "oklch(")?;
    if parts.len() < 3 {
        return None;
    }
    let lightness = parse_unit_interval(parts[0])?;
    let chroma = parse_fractional_channel(parts[1], 0.0, 1.0)?;
    let hue = parse_hue_channel(parts[2])?;
    let alpha = if parts.len() >= 4 {
        parse_alpha_channel(parts[3])?
    } else {
        1.0
    };
    Some(Color::from(Oklcha::new(lightness, chroma, hue, alpha)))
}

fn parse_oklab_color(value: &str) -> Option<Color> {
    let parts = parse_css_function_parts(value, "oklab(")?;
    if parts.len() < 3 {
        return None;
    }
    let lightness = parse_unit_interval(parts[0])?;
    let a = parse_fractional_channel(parts[1], -1.0, 1.0)?;
    let b = parse_fractional_channel(parts[2], -1.0, 1.0)?;
    let alpha = if parts.len() >= 4 {
        parse_alpha_channel(parts[3])?
    } else {
        1.0
    };
    Some(Color::from(Oklaba::new(lightness, a, b, alpha)))
}

fn parse_css_function_parts<'a>(value: &'a str, prefix: &str) -> Option<Vec<&'a str>> {
    let inner = value.strip_prefix(prefix)?.strip_suffix(')')?;
    Some(
        inner
            .split(|ch: char| ch.is_ascii_whitespace() || ch == ',' || ch == '/')
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>(),
    )
}

fn parse_hex_color(hex: &str) -> Option<Color> {
    let (r, g, b, a) = match hex.len() {
        6 => (
            u8::from_str_radix(&hex[0..2], 16).ok()?,
            u8::from_str_radix(&hex[2..4], 16).ok()?,
            u8::from_str_radix(&hex[4..6], 16).ok()?,
            255,
        ),
        8 => (
            u8::from_str_radix(&hex[0..2], 16).ok()?,
            u8::from_str_radix(&hex[2..4], 16).ok()?,
            u8::from_str_radix(&hex[4..6], 16).ok()?,
            u8::from_str_radix(&hex[6..8], 16).ok()?,
        ),
        _ => return None,
    };
    Some(Color::srgba(
        r as f32 / 255.0,
        g as f32 / 255.0,
        b as f32 / 255.0,
        a as f32 / 255.0,
    ))
}

fn parse_rgb_channel(value: &str) -> Option<u8> {
    if let Some(percent) = value.strip_suffix('%') {
        let percent = percent.parse::<f32>().ok()?.clamp(0.0, 100.0);
        return Some(((percent / 100.0) * 255.0).round() as u8);
    }
    let value = value.parse::<f32>().ok()?.clamp(0.0, 255.0);
    Some(value.round() as u8)
}

fn parse_alpha_channel(value: &str) -> Option<f32> {
    if let Some(percent) = value.strip_suffix('%') {
        let percent = percent.parse::<f32>().ok()?.clamp(0.0, 100.0);
        return Some(percent / 100.0);
    }
    Some(value.parse::<f32>().ok()?.clamp(0.0, 1.0))
}

fn parse_unit_interval(value: &str) -> Option<f32> {
    parse_fractional_channel(value, 0.0, 1.0)
}

fn parse_fractional_channel(value: &str, min: f32, max: f32) -> Option<f32> {
    if let Some(percent) = value.strip_suffix('%') {
        let percent = percent.parse::<f32>().ok()?;
        return Some((percent / 100.0).clamp(min, max));
    }
    Some(value.parse::<f32>().ok()?.clamp(min, max))
}

fn parse_hue_channel(value: &str) -> Option<f32> {
    let degrees = if let Some(raw) = value.strip_suffix("deg") {
        raw.parse::<f32>().ok()?
    } else if let Some(raw) = value.strip_suffix("grad") {
        raw.parse::<f32>().ok()? * 0.9
    } else if let Some(raw) = value.strip_suffix("rad") {
        raw.parse::<f32>().ok()? * 180.0 / std::f32::consts::PI
    } else if let Some(raw) = value.strip_suffix("turn") {
        raw.parse::<f32>().ok()? * 360.0
    } else {
        value.parse::<f32>().ok()?
    };
    Some(degrees.rem_euclid(360.0))
}

#[cfg(test)]
mod tests {
    use super::parse_css_color;
    use bevy::color::{Color, Oklaba, Oklcha};

    #[test]
    fn parses_oklch_theme_colors() {
        assert_eq!(
            parse_css_color("oklch(0.253 0.021 274.4)"),
            Some(Color::from(Oklcha::new(0.253, 0.021, 274.4, 1.0)))
        );
    }

    #[test]
    fn parses_oklab_theme_colors() {
        assert_eq!(
            parse_css_color("oklab(0.62 0.08 -0.12 / 75%)"),
            Some(Color::from(Oklaba::new(0.62, 0.08, -0.12, 0.75)))
        );
    }
}
