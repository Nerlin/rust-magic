use crate::{
    abilities::Abilities,
    events::{CardEvent, Event},
    game::{dispatch_event, Game, ObjectId},
    mana::Mana,
};

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
    Creature(CreatureState),
}

#[derive(Default)]
pub struct CreatureState {
    pub current: CreatureParams,
    pub default: CreatureParams,
}

#[derive(Default)]
pub struct CreatureParams {
    pub power: i16,
    pub toughness: i16,
    pub motion_sickness: bool,
}

impl CreatureState {
    pub fn new(power: i16, toughness: i16) -> CreatureState {
        CreatureState {
            current: CreatureParams {
                power,
                toughness,
                motion_sickness: true,
            },
            default: CreatureParams {
                power,
                toughness,
                motion_sickness: true,
            },
        }
    }

    /// Restores power and toughness of this creature to its default values.
    pub fn restore(&mut self) {
        self.current.power = self.default.power;
        self.current.toughness = self.default.toughness;
    }

    /// Resets the current state to the default state of this creature
    pub fn reset(&mut self) {
        self.current.power = self.default.power;
        self.current.toughness = self.default.toughness;
        self.current.motion_sickness = self.default.motion_sickness;
    }
}
