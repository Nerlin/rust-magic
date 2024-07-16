use crate::{abilities::Abilities, game::ObjectId, mana::Mana};

pub struct Card {
    pub id: ObjectId,
    pub owner_id: ObjectId,
    pub name: String,
    pub kind: CardType,
    pub cost: Mana,
    pub abilities: Abilities,
    pub zone: Zone,
}

pub enum CardType {
    Land,
    Artifact,
}

pub enum Zone {
    Battlefield(BattlefieldState),
    Graveyard,
}

pub struct BattlefieldState {
    pub tapped: bool,
}

impl BattlefieldState {
    pub fn new() -> BattlefieldState {
        BattlefieldState { tapped: false }
    }

    pub fn tap(&mut self) -> bool {
        if !self.tapped {
            self.tapped = true;
            true
        } else {
            false
        }
    }

    pub fn untap(&mut self) -> bool {
        if self.tapped {
            self.tapped = false;
            true
        } else {
            false
        }
    }
}
