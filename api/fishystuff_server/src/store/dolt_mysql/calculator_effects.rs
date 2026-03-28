#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(super) struct CalculatorItemEffectValues {
    pub(super) afr: Option<f32>,
    pub(super) bonus_rare: Option<f32>,
    pub(super) bonus_big: Option<f32>,
    pub(super) drr: Option<f32>,
    pub(super) exp_fish: Option<f32>,
    pub(super) exp_life: Option<f32>,
}

fn add_effect_value(slot: &mut Option<f32>, value: Option<f32>) {
    let Some(value) = value else {
        return;
    };
    *slot = Some(slot.unwrap_or(0.0) + value);
}

pub(super) fn extract_first_number(text: &str) -> Option<f32> {
    let chars: Vec<char> = text.chars().collect();
    let mut idx = 0;
    while idx < chars.len() {
        if chars[idx] == '+' || chars[idx] == '-' || chars[idx].is_ascii_digit() {
            let start = idx;
            idx += 1;
            let mut seen_digit = chars[start].is_ascii_digit();
            while idx < chars.len() && (chars[idx].is_ascii_digit() || chars[idx] == '.') {
                seen_digit |= chars[idx].is_ascii_digit();
                idx += 1;
            }
            if seen_digit {
                let candidate = chars[start..idx].iter().collect::<String>();
                if let Ok(value) = candidate.parse::<f32>() {
                    return Some(value);
                }
            }
        } else {
            idx += 1;
        }
    }
    None
}

fn extract_percent_ratio(text: &str) -> Option<f32> {
    extract_first_number(text).map(|value| value.abs() / 100.0)
}

fn parse_calculator_effect_line(values: &mut CalculatorItemEffectValues, line: &str) {
    let line = line.trim();
    if line.is_empty() {
        return;
    }
    if line.contains("자동 낚시") {
        add_effect_value(&mut values.afr, extract_percent_ratio(line));
    }
    if line.contains("희귀 어종") {
        add_effect_value(&mut values.bonus_rare, extract_percent_ratio(line));
    }
    if line.contains("대형 어종") {
        add_effect_value(&mut values.bonus_big, extract_percent_ratio(line));
    }
    if line.contains("내구도 소모 감소 저항") {
        add_effect_value(&mut values.drr, extract_percent_ratio(line));
    }
    if line.contains("낚시 경험치") {
        add_effect_value(&mut values.exp_fish, extract_percent_ratio(line));
    }
    if line.contains("생활 경험치") {
        add_effect_value(&mut values.exp_life, extract_percent_ratio(line));
    }
}

pub(super) fn parse_calculator_effect_text(values: &mut CalculatorItemEffectValues, text: &str) {
    for line in text.lines() {
        parse_calculator_effect_line(values, line);
    }
}

#[cfg(test)]
mod tests {
    use super::{extract_first_number, parse_calculator_effect_text, CalculatorItemEffectValues};

    #[test]
    fn extract_first_number_handles_signed_percent_lines() {
        assert_eq!(extract_first_number("자동 낚시 시간 -15%"), Some(-15.0));
        assert_eq!(extract_first_number("낚시 경험치 획득량 +10%"), Some(10.0));
        assert_eq!(extract_first_number("생활 숙련도 +20"), Some(20.0));
        assert_eq!(extract_first_number("효과 없음"), None);
    }

    #[test]
    fn calculator_effect_text_parses_balacs_style_lines() {
        let mut values = CalculatorItemEffectValues::default();
        parse_calculator_effect_text(
            &mut values,
            "자동 낚시 시간 감소 7%\n낚시 경험치 획득량 +10%",
        );

        assert_eq!(
            values,
            CalculatorItemEffectValues {
                afr: Some(0.07),
                exp_fish: Some(0.10),
                ..CalculatorItemEffectValues::default()
            }
        );
    }

    #[test]
    fn calculator_effect_text_parses_event_food_and_housekeeper_lines() {
        let mut values = CalculatorItemEffectValues::default();
        parse_calculator_effect_text(&mut values, "생활 숙련도 +50\n생활 경험치 획득량 +20%");

        assert_eq!(
            values,
            CalculatorItemEffectValues {
                exp_life: Some(0.20),
                ..CalculatorItemEffectValues::default()
            }
        );

        let mut event_food = CalculatorItemEffectValues::default();
        parse_calculator_effect_text(
            &mut event_food,
            "자동 낚시 시간 -10%\n생활 경험치 획득량 +50%\n생활 숙련도 +100",
        );

        assert_eq!(
            event_food,
            CalculatorItemEffectValues {
                afr: Some(0.10),
                exp_life: Some(0.50),
                ..CalculatorItemEffectValues::default()
            }
        );
    }
}
