use std::cell::RefCell;
use std::rc::Rc;

use crate::cards::CardState;
use crate::effects::Effect;
use crate::events::{Event, EventHandler, EventResult, Stack};

pub trait Cost {
    // Returns true if the cost is paid.
    fn pay(&self, card_state: Rc<RefCell<CardState>>) -> bool;
}

pub struct Activate {
    pub cost: Box<dyn Cost>,
    pub effect: Rc<dyn Effect>,
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
    pub effect: Rc<dyn Effect>,
    stack: Rc<RefCell<Stack>>,
}

impl Trigger {
    pub fn new(event: Event, effect: Rc<dyn Effect>, stack: Rc<RefCell<Stack>>) -> Trigger {
        let trigger = Trigger {
            event,
            effect,
            stack,
        };
        trigger
    }

    pub fn trigger(&self) {
        let effect = self.effect.clone();
        self.stack.borrow_mut().push(effect);
    }
}

impl EventHandler for Trigger {
    fn handle(&self, _event: &Event) -> EventResult {
        self.trigger();
        EventResult::Resolved
    }
}

pub struct Abilities {
    pub activated: Vec<Activate>,
    pub triggers: Vec<Rc<Trigger>>,
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
