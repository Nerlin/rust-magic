use std::cell::RefCell;
use std::rc::Rc;

use crate::cards::{Card};
use crate::mana::Mana;

pub type Player = Rc<RefCell<PlayerState>>;

pub fn new_player() -> Player {
    Rc::new(RefCell::new(PlayerState::new()))
}

pub struct PlayerState {
    pub life: Life,
    pub mana: Mana,

    pub battlefield: Vec<Card>,
}

pub type Life = u16;

impl PlayerState {
    pub fn new() -> PlayerState {
        PlayerState {
            life: 20,
            mana: Mana::new(),
            battlefield: vec![],
        }
    }
}