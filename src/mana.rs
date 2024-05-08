use std::collections::HashMap;
use phf::phf_map;

use regex::Regex;

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq, )]
pub enum Color {
    Colorless,
    White,
    Blue,
    Black,
    Red,
    Green
}

const COLOR_CODES: phf::Map<char, Color> = phf_map! {
    'R' => Color::Red,
    'W' => Color::White,
    'U' => Color::Blue,
    'B' => Color::Black,
    'G' => Color::Green,
    'C' => Color::Colorless
};

pub type Mana = HashMap<Color, u8>;

// Converted Mana Cost
pub struct CMC {
    pub value: String
}

impl CMC {
    pub fn new(value: &str) -> CMC {
        CMC { value: String::from(value) }
    }

    pub fn to_mana(&self) -> Mana {
        let cmc_regex = Regex::new(r"(?<colorless>\d*)(?<colored>[WUBRG]*)").unwrap();
        match cmc_regex.captures(&self.value) {
            Some(matched) => {
                let (_, [colorless, colored]) = matched.extract();

                let mut mana = Mana::new();
                if let Ok(colorless_amount) = colorless.parse::<u8>() {
                    mana.insert(Color::Colorless, colorless_amount);
                }

                for char in colored.chars().into_iter() {
                    if let Some(color) = COLOR_CODES.get(&char) {
                        let amount = *mana.get(color).unwrap_or(&0);
                        mana.insert(color.clone(), amount + 1);
                    }
                }

                mana
            },
            None => {
                Mana::new()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::mana::{CMC, Color, Mana};

    #[test]
    fn test_color() {
        assert_eq!(
            CMC::new("RG").to_mana(),
            Mana::from([
                (Color::Red, 1),
                (Color::Green, 1)
            ])
        );
    }

    #[test]
    fn test_colorless() {
        assert_eq!(
            CMC::new("4").to_mana(),
            Mana::from([
                (Color::Colorless, 4),
            ])
        );
    }

    #[test]
    fn test_combined() {
        assert_eq!(
            CMC::new("3UU").to_mana(),
            Mana::from([
                (Color::Colorless, 3),
                (Color::Blue, 2)
            ])
        );
    }

    #[test]
    fn test_multicolor() {
        assert_eq!(
            CMC::new("2BBWW").to_mana(),
            Mana::from([
                (Color::Colorless, 2),
                (Color::Black, 2),
                (Color::White, 2)
            ])
        );
    }
}