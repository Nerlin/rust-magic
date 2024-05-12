use phf::phf_map;
use regex::Regex;

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq, )]
pub enum Color {
    Colorless,
    White,
    Blue,
    Black,
    Red,
    Green,
}

const COLOR_CODES: phf::Map<char, Color> = phf_map! {
    'R' => Color::Red,
    'W' => Color::White,
    'U' => Color::Blue,
    'B' => Color::Black,
    'G' => Color::Green,
    'C' => Color::Colorless
};

#[derive(Debug, Hash, PartialOrd, PartialEq, Ord, Eq)]
pub struct Mana {
    pub red: u8,
    pub white: u8,
    pub blue: u8,
    pub black: u8,
    pub green: u8,
    pub colorless: u8,
}

impl Mana {
    pub fn new() -> Mana {
        Mana {
            red: 0,
            white: 0,
            blue: 0,
            black: 0,
            green: 0,
            colorless: 0,
        }
    }

    pub fn set(&mut self, color: &Color, amount: u8) {
        match color {
            Color::Colorless => self.colorless = amount,
            Color::White => self.white = amount,
            Color::Blue => self.blue = amount,
            Color::Black => self.black = amount,
            Color::Red => self.red = amount,
            Color::Green => self.green = amount,
        }
    }

    pub fn get(&self, color: &Color) -> u8 {
        match color {
            Color::Colorless => self.colorless,
            Color::White => self.white,
            Color::Blue => self.blue,
            Color::Black => self.black,
            Color::Red => self.red,
            Color::Green => self.green,
        }
    }

    pub fn has(&self, color: &Color) -> bool {
        self.get(color) > 0
    }

    pub fn iter(&self) -> Vec<(Color, u8)> {
        let mut vec = vec![];
        if self.colorless > 0 {
            vec.push((Color::Colorless, self.colorless));
        }
        if self.white > 0 {
            vec.push((Color::White, self.white));
        }
        if self.red > 0 {
            vec.push((Color::Red, self.red));
        }
        if self.green > 0 {
            vec.push((Color::Green, self.green));
        }
        if self.blue > 0 {
            vec.push((Color::Blue, self.blue));
        }
        if self.black > 0 {
            vec.push((Color::Black, self.black));
        }
        vec
    }
}

impl<const N: usize> From<[(Color, u8); N]> for Mana {
    fn from(value: [(Color, u8); N]) -> Self {
        let mut mana = Mana::new();
        for (color, amount) in value.iter() {
            mana.set(color, *amount);
        }
        mana
    }
}

impl From<&str> for Mana {
    fn from(value: &str) -> Self {
        let cmc_regex = Regex::new(r"(?<colorless>\d*)(?<colored>[WUBRG]*)").unwrap();
        match cmc_regex.captures(value) {
            Some(matched) => {
                let (_, [colorless, colored]) = matched.extract();

                let mut mana = Mana::new();
                if let Ok(colorless_amount) = colorless.parse::<u8>() {
                    mana.set(&Color::Colorless, colorless_amount);
                }

                for char in colored.chars().into_iter() {
                    if let Some(color) = COLOR_CODES.get(&char) {
                        let amount = mana.get(color);
                        mana.set(color, amount + 1);
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
    use crate::mana::{Color, Mana};

    #[test]
    fn test_color() {
        assert_eq!(
            Mana::from("RG"),
            Mana::from([
                (Color::Red, 1),
                (Color::Green, 1)
            ])
        );
    }

    #[test]
    fn test_colorless() {
        assert_eq!(
            Mana::from("4"),
            Mana::from([
                (Color::Colorless, 4),
            ])
        );
    }

    #[test]
    fn test_combined() {
        assert_eq!(
            Mana::from("3UU"),
            Mana::from([
                (Color::Colorless, 3),
                (Color::Blue, 2)
            ])
        );
    }

    #[test]
    fn test_multicolor() {
        assert_eq!(
            Mana::from("2BBWW"),
            Mana::from([
                (Color::Colorless, 2),
                (Color::Black, 2),
                (Color::White, 2)
            ])
        );
    }
}