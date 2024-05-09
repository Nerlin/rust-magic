use std::rc::Rc;

use crate::effects::Effect;

pub trait Cost {
    // Returns true if the cost is paid.
    fn pay(&mut self) -> bool;
}

pub struct Activate {
    pub cost: Box<dyn Cost>,
    pub effect: Rc<dyn Effect>
}

impl Activate {
    pub fn activate(&mut self) -> Option<Rc<dyn Effect>> {
        let paid = self.cost.pay();
        if paid {
            Some(self.effect.clone())
        } else {
            None
        }
    }
}

pub struct Abilities {
    pub activated: Vec<Activate>,
}

impl Abilities {
    pub fn new() -> Abilities {
        Abilities {
            activated: vec![],
        }
    }
}
