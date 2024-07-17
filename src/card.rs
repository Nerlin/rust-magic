use crate::{abilities::Abilities, game::ObjectId, mana::Mana};

#[derive(Default)]
pub struct Card {
    pub id: ObjectId,
    pub owner_id: ObjectId,
    pub name: String,
    pub kind: CardType,
    pub cost: Mana,
    pub abilities: Abilities,
    pub zone: Zone,

    pub tapped: bool,
}

impl Card {
    pub fn new() -> Card {
        Card::default()
    }

    pub fn tap(&mut self) -> bool {
        if self.zone == Zone::Battlefield && !self.tapped {
            self.tapped = true;
            true
        } else {
            false
        }
    }

    pub fn untap(&mut self) -> bool {
        if self.zone == Zone::Battlefield && self.tapped {
            self.tapped = false;
            true
        } else {
            false
        }
    }
}

#[derive(Clone, Default, PartialEq, PartialOrd)]
pub enum Zone {
    #[default]
    None,
    Battlefield,
    Graveyard,
    Library,
    Hand,
}

#[derive(Default)]
pub enum CardType {
    #[default]
    Land,
    Artifact,
}
