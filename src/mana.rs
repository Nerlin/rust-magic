use std::ops::{self, AddAssign, SubAssign};

use phf::phf_map;
use regex::Regex;

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
pub enum Color {
    Colorless,
    White,
    Blue,
    Black,
    Red,
    Green,
    Any,
}

const COLOR_CODES: phf::Map<char, Color> = phf_map! {
    'R' => Color::Red,
    'W' => Color::White,
    'U' => Color::Blue,
    'B' => Color::Black,
    'G' => Color::Green,
    'C' => Color::Colorless,
};

#[derive(Debug, Clone, Copy, Hash, PartialOrd, PartialEq, Ord, Eq)]
pub struct Mana {
    pub red: u8,
    pub white: u8,
    pub blue: u8,
    pub black: u8,
    pub green: u8,
    pub colorless: u8,
    pub any: u8,
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
            any: 0,
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
            Color::Any => self.any = amount,
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
            Color::Any => self.any,
        }
    }

    pub fn has(&self, color: &Color) -> bool {
        self.get(color) > 0
    }

    /// Returns the converted mana cost.
    pub fn cmc(&self) -> u8 {
        self.iter().iter().map(|(_, amount)| amount).sum()
    }

    /// Determines whether this mana is enough for paying the specified mana cost.
    pub fn enough(&self, mana: &Mana) -> bool {
        let mut remainder = self.clone();
        for (color, amount) in mana.iter() {
            match color {
                Color::Colorless | Color::Any => {
                    let colorless = remainder.get(&Color::Colorless);
                    if colorless >= amount {
                        remainder.set(&Color::Colorless, colorless - amount);
                    } else {
                        remainder.set(&Color::Colorless, 0);
                        return remainder.pick_any(amount - colorless).is_some();
                    }
                }
                Color::White | Color::Blue | Color::Black | Color::Green | Color::Red => {
                    let current = remainder.get(&color);
                    if current >= amount {
                        remainder.set(&color, current - amount);
                    } else {
                        return false;
                    }
                }
            }
        }
        true
    }

    /// Picks any amount of available mana and creates a new instance with the selection.
    pub fn pick_any(&self, amount: u8) -> Option<Mana> {
        let mut pick = Mana::new();
        let mut remainder = amount;

        for (color, current) in self.iter() {
            if current == 0 {
                continue;
            } else if current > remainder {
                pick.set(&color, remainder);
                return Some(pick);
            } else {
                pick.set(&color, current);
                remainder -= current;
                if remainder == 0 {
                    return Some(pick);
                }
            }
        }
        None
    }

    pub fn iter(&self) -> Vec<(Color, u8)> {
        let mut vec = vec![];
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
        if self.colorless > 0 {
            vec.push((Color::Colorless, self.colorless));
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
        let cmc_regex = Regex::new(r"(?<colorless>\d*)(?<any>[*]*)(?<colored>[WUBRG]*)").unwrap();
        match cmc_regex.captures(value) {
            Some(matched) => {
                let (_, [colorless, any, colored]) = matched.extract();

                let mut mana = Mana::new();
                mana.any = any.len() as u8;

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
            }
            None => Mana::new(),
        }
    }
}

impl ops::Add<Mana> for Mana {
    type Output = Mana;

    fn add(self, rhs: Mana) -> Self::Output {
        let mut result = self.clone();
        result.add_assign(rhs);
        result
    }
}

impl ops::AddAssign<Mana> for Mana {
    fn add_assign(&mut self, rhs: Mana) {
        for (color, amount) in rhs.iter() {
            let current = self.get(&color);
            self.set(&color, current.saturating_add(amount));
        }
    }
}

impl ops::Sub<Mana> for Mana {
    type Output = Mana;

    fn sub(self, rhs: Mana) -> Self::Output {
        let mut result = self.clone();
        result.sub_assign(rhs);
        result
    }
}

impl ops::SubAssign<Mana> for Mana {
    fn sub_assign(&mut self, rhs: Mana) {
        for (color, amount) in rhs.iter() {
            let current = self.get(&color);
            self.set(&color, current.saturating_sub(amount));
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
            Mana::from([(Color::Red, 1), (Color::Green, 1)])
        );
    }

    #[test]
    fn test_colorless() {
        assert_eq!(Mana::from("4"), Mana::from([(Color::Colorless, 4),]));
    }

    #[test]
    fn test_any() {
        assert_eq!(Mana::from("*"), Mana::from([(Color::Any, 1)]));
        assert_eq!(Mana::from("***"), Mana::from([(Color::Any, 3)]));
    }

    #[test]
    fn test_combined() {
        assert_eq!(
            Mana::from("3UU"),
            Mana::from([(Color::Colorless, 3), (Color::Blue, 2)])
        );
    }

    #[test]
    fn test_multicolor() {
        assert_eq!(
            Mana::from("2BBWW"),
            Mana::from([(Color::Colorless, 2), (Color::Black, 2), (Color::White, 2)])
        );
    }

    #[test]
    fn test_enough() {
        assert!(Mana::from("UURR").enough(&Mana::from("UR")));
    }

    #[test]
    fn test_enough_colorless() {
        assert!(Mana::from("5").enough(&Mana::from("2")));
    }

    #[test]
    fn test_enough_colored_as_colorless() {
        assert!(Mana::from("UURR").enough(&Mana::from("3")));
    }

    #[test]
    fn test_cmc() {
        assert_eq!(Mana::from("3").cmc(), 3);
        assert_eq!(Mana::from("R").cmc(), 1);
        assert_eq!(Mana::from("UW").cmc(), 2);
        assert_eq!(Mana::from("1U").cmc(), 2);
        assert_eq!(Mana::from("2WWBB").cmc(), 6);
    }
}
