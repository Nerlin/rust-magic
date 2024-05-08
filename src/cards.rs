use std::cell::RefCell;
use std::rc::Rc;

use crate::abilities::{Abilities, Activate, Cost};
use crate::effects::ManaEffect;
use crate::mana::{Color, Mana};
use crate::player::Player;
use crate::zones::Zone;

pub struct Card {
    pub name: String,
    pub typ: CardType,
    pub cost: Mana,
    pub flavor: Option<String>,
    pub color: Vec<Color>,
    pub zone: Zone,
}

pub enum CardType {
    Land,
    Creature,
    Sorcery,
    Instant,
    Enchantment,
    Artifact,
}

pub struct Permanent {
    pub tapped: bool,
}

impl Permanent {
    pub fn new() -> Permanent {
        Permanent { tapped: false }
    }

    pub fn tap(&mut self) -> bool {
        return if !self.tapped {
            self.tapped = true;
            true
        } else {
            false
        };
    }

    pub fn untap(&mut self) -> bool {
        return if self.tapped {
            self.tapped = false;
            true
        } else {
            false
        };
    }
}

pub struct Tap {
    pub target: Rc<RefCell<Permanent>>,
}

impl Cost for Tap {
    fn pay(&mut self) -> bool {
        self.target.borrow_mut().tap()
    }
}

pub struct Land {
    pub card: Card,
    pub permanent: Rc<RefCell<Permanent>>,
    pub abilities: Abilities,
}

impl Land {
    pub fn basic(name: &str, owner: Rc<RefCell<Player>>, mana: Mana) -> Land {
        let mut land = Land {
            card: Card {
                name: String::from(name),
                typ: CardType::Land,
                flavor: None,
                cost: Mana::new(),

                // Lands are colorless
                color: vec![Color::Colorless],
                zone: Zone::None,
            },
            permanent: Rc::new(RefCell::new(Permanent::new())),
            abilities: Abilities::new(),
        };

        let tap = Tap { target: land.permanent.clone() };
        let effect = ManaEffect { player: owner, mana };
        let mana_ability = Activate { cost: Box::new(tap), effect: Rc::new(effect) };

        land.abilities.activated.push(mana_ability);
        land
    }
}