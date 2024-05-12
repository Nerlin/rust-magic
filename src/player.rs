use std::cell::RefCell;
use std::rc::Rc;

use rand::random;

use crate::cards::Card;
use crate::effects::Alive;
use crate::mana::Mana;

pub type Player = Rc<RefCell<PlayerState>>;

pub fn new_player() -> Player {
    Rc::new(RefCell::new(PlayerState::new()))
}

pub struct PlayerState {
    pub id: u64,
    pub life: u16,
    pub mana: Mana,
    pub battlefield: Vec<Card>,
}

impl PlayerState {
    pub const START_LIFE: u16 = 20;

    pub fn new() -> PlayerState {
        PlayerState {
            id: random(),
            life: 20,
            mana: Mana::new(),
            battlefield: vec![],
        }
    }
}

impl Alive for PlayerState {
    fn gain_life(&mut self, life: u16) {
        self.life += life;
    }

    fn lose_life(&mut self, life: u16) {
        self.life -= life;
    }

    fn take_damage(&mut self, damage: u16) {
        self.life -= damage;
    }
}