use indexmap::IndexSet;
use rand::seq::SliceRandom;
use rand::thread_rng;

use crate::{
    abilities::{
        apply_static_abilities, ActivatedAbility, Cost, PlayAbility, StaticAbility,
        TriggeredAbility,
    },
    events::{dispatch_event, CardEvent, Event},
    game::{Game, GameStatus, ObjectId, Value},
};

#[derive(Default, Clone)]
pub struct Card {
    pub id: ObjectId,
    pub owner_id: ObjectId,
    pub name: String,
    pub kind: CardType,
    pub subtypes: IndexSet<CardSubtype>,
    pub cost: Cost,
    pub zone: Zone,

    /// Defines the ability that happens when the card is resolved
    pub play_ability: Option<PlayAbility>,

    pub activated_abilities: Vec<ActivatedAbility>,
    pub triggered_abilities: Vec<TriggeredAbility>,
    pub static_abilities: IndexSet<StaticAbility>,

    pub state: CardState,
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

    pub fn new_creature(owner_id: ObjectId, power: i16, toughness: i16) -> Card {
        let mut card = Card::new(owner_id);
        card.kind = CardType::Creature;
        card.state.power = Value::new(power);
        card.state.toughness = Value::new(toughness);
        card.state.summoning_sickness = Value::new(true);
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
        if self.zone == Zone::Battlefield && !self.state.tapped.current {
            self.state.tapped.current = true;
            true
        } else {
            false
        }
    }

    pub fn untap(&mut self) -> bool {
        if self.zone == Zone::Battlefield && self.state.tapped.current {
            self.state.tapped.current = false;
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

pub fn draw_card(game: &mut Game, player_id: ObjectId) -> Option<ObjectId> {
    let player = if let Some(player) = game.get_player(player_id) {
        player
    } else {
        return None;
    };

    let card_id = if let Some(card_id) = player.library.pop() {
        card_id
    } else {
        game.status = GameStatus::Lose(player_id);
        return None;
    };

    change_zone(game, card_id, Zone::Hand);
    dispatch_event(
        game,
        Event::Draw(CardEvent {
            owner: player_id,
            card: card_id,
            source: None,
        }),
    );

    return Some(card_id);
}

pub fn put_on_deck_top(game: &mut Game, card_id: ObjectId, player_id: ObjectId) {
    if let Some(player) = game.get_player(player_id) {
        for (_, cards) in player.zones_mut() {
            cards.shift_remove(&card_id);
        }
        player.library.insert(card_id);
    } else {
        return;
    };

    if let Some(card) = game.get_card(card_id) {
        card.zone = Zone::Library;
        card.state.reset();
    } else {
        panic!("Card {card_id} does not exist.");
    };
}

pub fn put_on_deck_bottom(game: &mut Game, card_id: ObjectId, player_id: ObjectId) {
    if let Some(player) = game.get_player(player_id) {
        for (_, cards) in player.zones_mut() {
            cards.shift_remove(&card_id);
        }
        player.library.shift_insert(0, card_id);
    } else {
        return;
    };

    if let Some(card) = game.get_card(card_id) {
        card.zone = Zone::Library;
        card.state.reset();
    } else {
        panic!("Card {card_id} does not exist.");
    };
}

pub fn shuffle_deck(game: &mut Game, player_id: ObjectId) {
    if let Some(player) = game.get_player(player_id) {
        let mut library: Vec<ObjectId> = player.library.clone().into_iter().collect();
        library.shuffle(&mut thread_rng());

        player.library = IndexSet::new();
        for card_id in library {
            player.library.insert(card_id);
        }
    }
}

fn change_zone(game: &mut Game, card_id: ObjectId, zone: Zone) {
    let player_id;
    if let Some(card) = game.get_card(card_id) {
        card.zone = zone.clone();
        card.state.reset();
        player_id = card.owner_id;
    } else {
        return;
    }

    apply_static_abilities(game, card_id);

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

#[derive(Clone, Default, PartialEq, PartialOrd)]
pub enum CardType {
    #[default]
    Land,
    Artifact,
    Enchantment,
    Creature,
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
    Dragon,
    Bird,
    Human,
    Spider,
}

#[derive(Clone, Default, PartialEq, PartialOrd)]
pub struct CardState {
    pub power: Value<i16>,
    pub toughness: Value<i16>,
    pub summoning_sickness: Value<bool>,
    pub tapped: Value<bool>,
}

impl CardState {
    pub fn new() -> CardState {
        let mut state = CardState::default();
        state.summoning_sickness.default = false;
        state.tapped.default = false;
        state
    }

    pub fn new_creature(power: i16, toughness: i16) -> CardState {
        CardState {
            power: Value::new(power),
            toughness: Value::new(toughness),
            summoning_sickness: Value::new(true),
            tapped: Value::new(false),
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
        self.summoning_sickness.reset();
        self.tapped.reset();
    }
}

pub fn is_alive(game: &mut Game, card_id: ObjectId) -> bool {
    if let Some(card) = game.get_card(card_id) {
        return card.kind == CardType::Creature && card.state.toughness.current > 0;
    }
    false
}

#[cfg(test)]
mod tests {
    use crate::{
        card::{draw_card, put_on_deck_bottom, put_on_deck_top, Card},
        game::{Game, GameStatus},
    };

    #[test]
    fn test_put_on_deck_top() {
        let (mut game, player_id, _) = Game::new();
        let forest_id = game.add_card(Card::new_land(player_id));
        let mountain_id = game.add_card(Card::new_land(player_id));

        put_on_deck_top(&mut game, forest_id, player_id);
        put_on_deck_top(&mut game, mountain_id, player_id);

        let top = draw_card(&mut game, player_id);
        let bottom = draw_card(&mut game, player_id);
        assert_eq!(top, Some(mountain_id));
        assert_eq!(bottom, Some(forest_id));
    }

    #[test]
    fn test_put_on_deck_bottom() {
        let (mut game, player_id, _) = Game::new();
        let forest_id = game.add_card(Card::new_land(player_id));
        let mountain_id = game.add_card(Card::new_land(player_id));

        put_on_deck_bottom(&mut game, forest_id, player_id);
        put_on_deck_bottom(&mut game, mountain_id, player_id);

        let top = draw_card(&mut game, player_id);
        let bottom = draw_card(&mut game, player_id);
        assert_eq!(top, Some(forest_id));
        assert_eq!(bottom, Some(mountain_id));
    }

    #[test]
    fn test_draw_card() {
        let (mut game, player_id, _) = Game::new();
        let card_id = game.add_card(Card::new_sorcery(player_id));

        put_on_deck_top(&mut game, card_id, player_id);
        let drawn_card = draw_card(&mut game, player_id);

        assert_eq!(drawn_card, Some(card_id));
    }

    #[test]
    fn test_draw_card_lose_game() {
        let (mut game, player_id, _) = Game::new();

        let result = draw_card(&mut game, player_id);
        assert_eq!(result, None);
        assert_eq!(game.status, GameStatus::Lose(player_id));
    }
}
