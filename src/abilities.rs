use std::rc::Rc;

use crate::effects::Effect;
use crate::mana::Mana;
use crate::player::Player;

pub trait Cost {
    // Returns true if the cost is paid.
    fn pay(&mut self) -> bool;
}

pub struct ManaCost {
    pub player: Player,
    pub cost: Mana,
}

impl Cost for ManaCost {
    fn pay(&mut self) -> bool {
        for (color, amount) in &self.cost {
            match self.player.mana.get(color) {
                Some(player_amount) => {
                    if player_amount >= amount {
                        self.player.mana.insert(color.clone(), player_amount - amount);
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

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::rc::Rc;

    use crate::cards::Land;
    use crate::mana::{CMC, Color};
    use crate::player::Player;

    #[test]
    fn test_basic_land() {
        let player = Rc::new(RefCell::new(Player::new()));
        let mut forest = Land::basic("Forest", player.clone(), CMC::new("G").to_mana());
        let mana_ability = forest.abilities.activated.get_mut(0).unwrap();
        let effect = mana_ability.activate().unwrap();
        effect.resolve();

        assert_eq!(player.borrow_mut().mana, HashMap::from([
            (Color::Green, 1)
        ]));
    }

    #[test]
    fn test_basic_land_tapped() {
        let player = Rc::new(RefCell::new(Player::new()));
        let mut mountain = Land::basic("Mountain", player.clone(), CMC::new("R").to_mana());
        mountain.permanent.borrow_mut().tap();

        let mana_ability = mountain.abilities.activated.get_mut(0).unwrap();
        let effect = mana_ability.activate();
        assert!(effect.is_none())
    }

    #[test]
    fn test_basic_land_untapped() {
        let player = Rc::new(RefCell::new(Player::new()));
        let mut island = Land::basic("Island", player.clone(), CMC::new("U").to_mana());
        let mana_ability = island.abilities.activated.get_mut(0).unwrap();
        mana_ability.activate().unwrap().resolve();
        island.permanent.borrow_mut().untap();

        mana_ability.activate().unwrap().resolve();
        assert_eq!(player.borrow_mut().mana, HashMap::from([
            (Color::Blue, 2)
        ]));
    }
}