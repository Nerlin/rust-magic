use std::collections::HashMap;

use indexmap::IndexSet;

use crate::{
    abilities::Effect,
    action::Action,
    card::{Card, Zone},
    mana::Mana,
    turn::Turn,
};

pub struct Game {
    pub turn: Turn,
    pub status: GameStatus,
    pub(crate) players: Vec<Player>,
    pub(crate) cards: HashMap<usize, Card>,
    pub(crate) stack: Vec<Stacked>,
    uid: ObjectId,
}

#[derive(Debug, PartialEq, PartialOrd)]
pub enum GameStatus {
    Play,
    Lose(ObjectId),
}

pub type ObjectId = usize;

impl Game {
    pub fn new() -> (Game, ObjectId, ObjectId) {
        let mut game = Game::default();
        let player_id = game.add_player(Player::new());
        let opponent_id = game.add_player(Player::new());
        game.turn = Turn::new(player_id);
        (game, player_id, opponent_id)
    }

    pub fn get_uid(&mut self) -> ObjectId {
        self.uid += 1;
        self.uid
    }

    pub fn add_player(&mut self, mut player: Player) -> ObjectId {
        let player_id = self.get_uid();
        player.id = player_id;

        self.players.push(player);
        player_id
    }

    pub fn get_player(&mut self, player_id: ObjectId) -> Option<&mut Player> {
        for player in self.players.iter_mut() {
            if player.id == player_id {
                return Some(player);
            }
        }
        None
    }

    pub fn get_player_ids(&self) -> Vec<ObjectId> {
        self.players
            .iter()
            .map(|player| player.id)
            .collect::<Vec<ObjectId>>()
            .clone()
    }

    pub fn get_next_player(&self, player_id: ObjectId) -> ObjectId {
        let mut found = false;

        for player in self.players.iter() {
            if found {
                return player.id;
            } else if player.id == player_id {
                found = true;
            }
        }

        if let Some(player) = self.players.first() {
            player.id
        } else {
            0
        }
    }

    pub fn add_card(&mut self, mut card: Card) -> ObjectId {
        let card_id = self.get_uid();
        card.id = card_id;

        self.cards.insert(card.id, card);
        card_id
    }

    pub fn get_card(&mut self, card_id: ObjectId) -> Option<&mut Card> {
        self.cards.get_mut(&card_id)
    }
}

impl Default for Game {
    fn default() -> Self {
        Game {
            status: GameStatus::Play,
            uid: 0,
            stack: vec![],
            players: vec![],
            cards: HashMap::new(),
            turn: Turn::new(0),
        }
    }
}

pub enum Stacked {
    Spell { card_id: ObjectId, action: Action },
    Ability { effect: Effect, action: Action },
}

pub struct Player {
    pub id: ObjectId,
    pub life: i16,
    pub mana: Mana,

    pub library: IndexSet<ObjectId>,
    pub hand: IndexSet<ObjectId>,
    pub battlefield: IndexSet<ObjectId>,
    pub graveyard: IndexSet<ObjectId>,

    pub hand_size_limit: Value<usize>,

    /// Defines how many lands this player can play per turn
    pub land_limit: Value<usize>,
}

pub const DEFAULT_HAND_SIZE: usize = 7;
pub const DEFAULT_PLAYER_LIFE: i16 = 20;
pub const DEFAULT_LAND_LIMIT: usize = 1;

impl Player {
    pub fn new() -> Player {
        Player {
            id: 0,
            life: DEFAULT_PLAYER_LIFE,
            mana: Mana::new(),
            library: IndexSet::new(),
            hand: IndexSet::new(),
            battlefield: IndexSet::new(),
            graveyard: IndexSet::new(),
            hand_size_limit: Value::new(DEFAULT_HAND_SIZE),
            land_limit: Value::new(DEFAULT_LAND_LIMIT),
        }
    }

    pub fn zones(&self) -> Vec<(Zone, &IndexSet<ObjectId>)> {
        vec![
            (Zone::Library, &self.library),
            (Zone::Hand, &self.hand),
            (Zone::Battlefield, &self.battlefield),
            (Zone::Graveyard, &self.graveyard),
        ]
    }

    pub fn zones_mut(&mut self) -> Vec<(Zone, &mut IndexSet<ObjectId>)> {
        vec![
            (Zone::Library, &mut self.library),
            (Zone::Hand, &mut self.hand),
            (Zone::Battlefield, &mut self.battlefield),
            (Zone::Graveyard, &mut self.graveyard),
        ]
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd)]
pub struct Value<T: Clone + Copy + Default + PartialEq + PartialOrd> {
    pub current: T,
    pub default: T,
}

impl<T: Copy + Default + PartialEq + PartialOrd> Value<T> {
    pub fn new(value: T) -> Value<T> {
        Value {
            current: value,
            default: value,
        }
    }

    pub fn reset(&mut self) {
        self.current = self.default;
    }
}

pub fn add_mana(game: &mut Game, player_id: ObjectId, mana: Mana) {
    if let Some(player) = game.get_player(player_id) {
        player.mana += mana;
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        abilities::deal_player_damage,
        game::{Game, GameStatus, Player},
    };

    #[test]
    fn test_lethal_damage() {
        let mut game = Game::default();
        let mut player = Player::new();
        player.life = 3;

        let player_id = game.add_player(player);
        deal_player_damage(&mut game, player_id, 3);

        let player = game.get_player(player_id).unwrap();
        assert_eq!(player.life, 0);
        assert_eq!(game.status, GameStatus::Lose(player_id));
    }
}
