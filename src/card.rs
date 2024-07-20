use std::collections::HashSet;

use crate::{
    abilities::{Abilities, Cost},
    events::{dispatch_event, CardEvent, Event},
    game::{Game, ObjectId, Value},
};

#[derive(Default)]
pub struct Card {
    pub id: ObjectId,
    pub owner_id: ObjectId,
    pub name: String,
    pub kind: CardType,
    pub subtypes: HashSet<CardSubtype>,
    pub cost: Cost,
    pub abilities: Abilities,
    pub zone: Zone,

    pub tapped: bool,
}

impl Card {
    pub fn new(owner_id: ObjectId) -> Card {
        let mut card = Card::default();
        card.owner_id = owner_id;
        card
    }

    pub fn new_land(owner_id: ObjectId) -> Card {
        let mut card = Card::new(owner_id);
        card.kind = CardType::Land;
        card
    }

    pub fn new_creature(owner_id: ObjectId, state: CreatureState) -> Card {
        let mut card = Card::new(owner_id);
        card.kind = CardType::Creature(state);
        card
    }

    pub fn new_artifact(owner_id: ObjectId) -> Card {
        let mut card = Card::new(owner_id);
        card.kind = CardType::Artifact;
        card
    }

    pub fn new_enchantment(owner_id: ObjectId) -> Card {
        let mut card = Card::new(owner_id);
        card.kind = CardType::Enchantment;
        card
    }

    pub fn new_instant(owner_id: ObjectId) -> Card {
        let mut card = Card::new(owner_id);
        card.kind = CardType::Instant;
        card
    }

    pub fn new_sorcery(owner_id: ObjectId) -> Card {
        let mut card = Card::new(owner_id);
        card.kind = CardType::Sorcery;
        card
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

pub fn tap_card(game: &mut Game, card_id: ObjectId, source: Option<ObjectId>) -> bool {
    if let Some(card) = game.get_card(card_id) {
        if card.tap() {
            let owner_id = card.owner_id;
            dispatch_event(
                game,
                Event::Tap(CardEvent {
                    source,
                    owner: owner_id,
                    card: card_id,
                }),
            );
            return true;
        }
    }
    false
}

pub fn untap_card(game: &mut Game, card_id: ObjectId, source: Option<ObjectId>) -> bool {
    if let Some(card) = game.get_card(card_id) {
        if card.untap() {
            let owner_id = card.owner_id;
            dispatch_event(
                game,
                Event::Untap(CardEvent {
                    source,
                    owner: owner_id,
                    card: card_id,
                }),
            )
        }
        return true;
    }
    false
}

pub fn put_on_battlefield(game: &mut Game, card_id: ObjectId) {
    change_zone(game, card_id, Zone::Battlefield)
}

pub fn put_on_graveyard(game: &mut Game, card_id: ObjectId) {
    change_zone(game, card_id, Zone::Graveyard)
}

pub fn put_on_stack(game: &mut Game, card_id: ObjectId) {
    change_zone(game, card_id, Zone::Stack)
}

pub fn put_in_hand(game: &mut Game, card_id: ObjectId) {
    change_zone(game, card_id, Zone::Hand)
}

fn change_zone(game: &mut Game, card_id: ObjectId, zone: Zone) {
    let player_id;
    if let Some(card) = game.get_card(card_id) {
        card.zone = zone.clone();
        card.tapped = false;

        if let CardType::Creature(creature) = &mut card.kind {
            creature.reset();
        }

        player_id = card.owner_id;
    } else {
        return;
    }

    if let Some(player) = game.get_player(player_id) {
        for (player_zone, cards) in player.zones_mut() {
            if player_zone == zone {
                cards.insert(card_id);
            } else {
                cards.shift_remove(&card_id);
            }
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, PartialOrd)]
pub enum Zone {
    #[default]
    None,
    Battlefield,
    Graveyard,
    Library,
    Hand,
    Stack,
}

#[derive(Default, PartialEq, PartialOrd)]
pub enum CardType {
    #[default]
    Land,
    Artifact,
    Enchantment,
    Creature(CreatureState),
    Instant,
    Sorcery,
}

#[derive(Clone, Copy, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum CardSubtype {
    #[default]
    None,

    Forest,
    Mountain,
    Swamp,
    Plains,
    Island,

    Spirit,
}

#[derive(Default, PartialEq, PartialOrd)]
pub struct CreatureState {
    pub power: Value<i16>,
    pub toughness: Value<i16>,
    pub motion_sickness: Value<bool>,
}

impl CreatureState {
    pub fn new(power: i16, toughness: i16) -> CreatureState {
        CreatureState {
            power: Value::new(power),
            toughness: Value::new(toughness),
            motion_sickness: Value::new(true),
        }
    }

    /// Restores power and toughness of this creature to its default values.
    pub fn restore(&mut self) {
        self.power.reset();
        self.toughness.reset();
    }

    /// Resets the current state to the default state of this creature
    pub fn reset(&mut self) {
        self.power.reset();
        self.toughness.reset();
        self.motion_sickness.reset();
    }
}

pub fn is_alive(game: &mut Game, card_id: ObjectId) -> bool {
    if let Some(card) = game.get_card(card_id) {
        if let CardType::Creature(creature) = &card.kind {
            return creature.toughness.current > 0;
        }
    }
    false
}
