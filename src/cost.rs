use std::cell::RefCell;
use std::rc::Rc;

use crate::abilities::Cost;
use crate::cards::CardState;
use crate::mana::Mana;
use crate::player::Player;

pub struct Tap {
}

impl Cost for Tap {
    fn pay(&self, card_state: Rc<RefCell<CardState>>) -> bool {
        card_state.borrow_mut().tap()
    }
}

pub struct ManaCost {
    pub player: Player,
    pub cost: Mana,
}

impl Cost for ManaCost {
    fn pay(&self, _card_state: Rc<RefCell<CardState>>) -> bool {
        let mut player = self.player.borrow_mut();
        for (color, amount) in &self.cost {
            match player.mana.get(color) {
                Some(player_amount) => {
                    if player_amount >= amount {
                        let player_amount = player_amount.clone();
                        player.mana.insert(color.clone(), player_amount - amount);
                    } else {
                        return false;
                    }
                }
                None => {
                    return false;
                }
            }
        }
        true
    }
}

pub struct LifeCost {
    pub player: Player,
    pub cost: u16
}

impl Cost for LifeCost {
    fn pay(&self, _card_state: Rc<RefCell<CardState>>) -> bool {
        let mut player = self.player.borrow_mut();
        if player.life >= self.cost {
            player.life = player.life - self.cost;
            return true
        }
        false
    }
}

pub struct MultiCost {
    pub items: Vec<Box<dyn Cost>>
}

impl Cost for MultiCost {
    fn pay(&self, card_state: Rc<RefCell<CardState>>) -> bool {
        for cost in self.items.iter() {
            let paid = cost.pay(card_state.clone());
            if !paid {
                return false;
            }
        }
        true
    }
}