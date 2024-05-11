use std::cell::RefCell;
use std::rc::Rc;

use crate::cards::CardState;
use crate::effects::Effect;
use crate::events::Event;

pub trait Cost {
    // Returns true if the cost is paid.
    fn pay(&self, card_state: Rc<RefCell<CardState>>) -> bool;
}

pub struct Activate {
    pub cost: Box<dyn Cost>,
    pub effect: Rc<dyn Effect>
}

impl Activate {
    pub fn activate(&self, card_state: Rc<RefCell<CardState>>) -> Option<Rc<dyn Effect>> {
        let paid = self.cost.pay(card_state);
        if paid {
            Some(self.effect.clone())
        } else {
            None
        }
    }
}

pub struct Trigger {
    pub event: Event,
    pub effect: Rc<dyn Effect>
}

impl Trigger {
    pub fn trigger(&self) -> Rc<dyn Effect> {
        self.effect.clone()
    }
}

pub struct Abilities {
    pub activated: Vec<Activate>,
    pub triggers: Vec<Trigger>,
}

impl Abilities {
    pub fn new() -> Abilities {
        Abilities {
            activated: vec![],
            triggers: vec![],
        }
    }

    pub fn run_triggers(&self, event: Event) {
        for ability in self.triggers.iter() {
            if ability.event == event {
                ability.trigger();
            }
        }
    }
}
