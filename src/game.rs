use std::collections::HashMap;

use indexmap::IndexSet;
use rand::seq::SliceRandom;
use rand::thread_rng;

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
    pub(crate) stack: Vec<StackEntry>,
    uid: ObjectId,
}

#[derive(Debug, PartialEq, PartialOrd)]
pub enum GameStatus {
    Play,
    Lose(ObjectId),
}

pub type ObjectId = usize;

impl Game {
    pub fn new() -> Game {
        Game {
            status: GameStatus::Play,
            uid: 0,
            stack: vec![],
            players: vec![],
            cards: HashMap::new(),
            turn: Turn::new(0),
        }
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

pub struct StackEntry {
    pub effect: Effect,
    pub action: Action,
}

pub struct Player {
    pub id: ObjectId,
    pub life: i16,
    pub mana: Mana,

    pub library: IndexSet<ObjectId>,
    pub hand: IndexSet<ObjectId>,
    pub battlefield: IndexSet<ObjectId>,
    pub graveyard: IndexSet<ObjectId>,

    pub max_hand_size: usize,
}

pub const DEFAULT_HAND_SIZE: usize = 7;
pub const DEFAULT_PLAYER_LIFE: i16 = 20;

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
            max_hand_size: DEFAULT_HAND_SIZE,
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

pub fn start_game(game: &mut Game) {
    if game.players.len() != 2 {
        panic!("The game must include exactly two players.");
    }

    let active_player = game.players.choose(&mut thread_rng()).unwrap();
    game.turn = Turn::new(active_player.id);
}
