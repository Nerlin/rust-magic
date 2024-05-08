use std::cell::RefCell;
use std::rc::Rc;

use crate::effects::{Effect, EffectKind};
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

pub trait Activate {
    fn cost(&self) -> &Box<dyn Cost>;

    // Returns true if ability was activated successfully
    fn activate(&mut self) -> Option<Effect>;
}

pub struct Abilities {
    pub activated: Vec<Box<dyn Activate>>,
}

impl Abilities {
    pub fn new() -> Abilities {
        Abilities {
            activated: vec![],
        }
    }
}


pub struct ManaAbility {
    pub player: Rc<RefCell<Player>>,
    pub mana: Mana,
    pub cost: Box<dyn Cost>,
}

impl Activate for ManaAbility {
    fn cost(&self) -> &Box<dyn Cost> {
        &self.cost
    }

    fn activate(&mut self) -> Option<Effect> {
        let paid = self.cost.pay();
        if !paid {
            return None;
        }

        let player_rc = self.player.clone();
        let mana = self.mana.clone();

        Some(Effect {
            kind: EffectKind::Immediate,
            target: None,
            resolver: Box::new(move || {
                let mut player = player_rc.borrow_mut();

                for (color, amount) in &mana {
                    let player_amount = player.mana.get(color).unwrap_or(&0).clone();
                    player.mana.insert(color.clone(), player_amount + amount);
                }
            }),
        })
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::rc::Rc;

    use crate::abilities::Activate;
    use crate::cards::Land;
    use crate::mana::{CMC, Color};
    use crate::player::Player;

    #[test]
    fn test_basic_land() {
        let player = Rc::new(RefCell::new(Player::new()));
        let mut forest = Land::basic("Forest", player.clone(), CMC::new("G").to_mana());
        let mana_ability: &mut Box<dyn Activate> = forest.abilities.activated.get_mut(0).unwrap();
        let effect = &mana_ability.activate().unwrap();
        effect.resolve();

        assert_eq!(player.borrow_mut().mana, HashMap::from([
            (Color::Green, 1)
        ]));
    }
}