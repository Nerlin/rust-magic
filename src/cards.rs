use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;

use crate::abilities::{Abilities, Activate};
use crate::cost::Tap;
use crate::effects::ManaEffect;
use crate::mana::{Color, Mana};
use crate::player::Player;
use crate::zones::Zone;

pub struct Card {
    pub name: String,
    pub class: CardType,
    pub cost: Mana,
    pub flavor: Option<String>,
    pub color: HashSet<Color>,
    pub zone: Zone,
    pub abilities: Abilities,
}

pub enum CardType {
    Land,
    Creature,
    Sorcery,
    Instant,
    Enchantment,
    Artifact,
}

pub struct Permanent {
    pub tapped: bool,
}

impl Permanent {
    pub fn new() -> Permanent {
        Permanent { tapped: false }
    }

    pub fn tap(&mut self) -> bool {
        return if !self.tapped {
            self.tapped = true;
            true
        } else {
            false
        };
    }

    pub fn untap(&mut self) -> bool {
        return if self.tapped {
            self.tapped = false;
            true
        } else {
            false
        };
    }
}

pub struct Land {
    pub card: Card,
    pub permanent: Rc<RefCell<Permanent>>,
}

impl Land {
    pub fn basic(name: &str, owner: Player, mana: Mana) -> Land {
        let mut land = Land {
            card: Card {
                name: String::from(name),
                class: CardType::Land,
                flavor: None,
                cost: Mana::new(),

                // Lands are colorless
                color: HashSet::from([Color::Colorless]),
                abilities: Abilities::new(),
                zone: Zone::None,
            },
            permanent: Rc::new(RefCell::new(Permanent::new())),

        };

        let tap = Tap { target: land.permanent.clone() };
        let effect = ManaEffect { player: owner, mana };
        let mana_ability = Activate { cost: Box::new(tap), effect: Rc::new(effect) };

        land.card.abilities.activated.push(mana_ability);
        land
    }
}

pub struct Creature {
    pub card: Card,
    pub permanent: Rc<RefCell<Permanent>>,
    pub power: i16,
    pub toughness: i16,
}


#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::collections::HashSet;
    use std::rc::Rc;

    use crate::abilities::{Abilities, Activate};
    use crate::cards::{Card, CardType, Creature, Land, Permanent};
    use crate::cost::{LifeCost, MultiCost, Tap};
    use crate::effects::ManaEffect;
    use crate::mana::{CMC, Color, Mana};
    use crate::player::{new_player, Player};
    use crate::zones::Zone;

    #[test]
    fn test_basic_land() {
        let player = new_player();
        let mut forest = Land::basic("Forest", player.clone(), CMC::new("G").to_mana());
        let mana_ability = forest.card.abilities.activated.get_mut(0).unwrap();
        let effect = mana_ability.activate().unwrap();
        effect.resolve();

        assert_eq!(player.borrow_mut().mana, Mana::from([
            (Color::Green, 1)
        ]));
    }

    #[test]
    fn test_basic_land_tapped() {
        let player = new_player();
        let mut mountain = Land::basic("Mountain", player.clone(), CMC::new("R").to_mana());
        mountain.permanent.borrow_mut().tap();

        let mana_ability = mountain.card.abilities.activated.get_mut(0).unwrap();
        let effect = mana_ability.activate();
        assert!(effect.is_none())
    }

    #[test]
    fn test_basic_land_untapped() {
        let player = new_player();
        let mut island = Land::basic("Island", player.clone(), CMC::new("U").to_mana());

        let mana_ability = island.card.abilities.activated.get_mut(0).unwrap();
        mana_ability.activate().unwrap().resolve();
        island.permanent.borrow_mut().untap();

        mana_ability.activate().unwrap().resolve();
        assert_eq!(player.borrow_mut().mana, Mana::from([
            (Color::Blue, 2)
        ]));
    }

    #[test]
    fn test_blightsoil_druid() {
        let player = new_player();
        let mut creature = create_blightsoil_druid(player.clone());

        let mana_ability = creature.card.abilities.activated.get_mut(0).unwrap();
        let effect = mana_ability.activate().unwrap();
        effect.resolve();

        assert_eq!(
            player.borrow_mut().mana,
            Mana::from([(Color::Green, 1)])
        );
    }

    #[test]
    fn test_blightsoil_druid_low_life() {
        let player = new_player();
        player.borrow_mut().life = 0;

        let mut creature = create_blightsoil_druid(player.clone());

        let mana_ability = creature.card.abilities.activated.get_mut(0).unwrap();
        let effect = mana_ability.activate();

        assert!(effect.is_none());
    }

    #[test]
    fn test_blightsoil_druid_tapped() {
        let player = new_player();

        let mut creature = create_blightsoil_druid(player.clone());
        creature.permanent.borrow_mut().tap();

        let mana_ability = creature.card.abilities.activated.get_mut(0).unwrap();
        let effect = mana_ability.activate();

        assert!(effect.is_none());
    }

    fn create_blightsoil_druid(player: Player) -> Creature {
        let mut creature = Creature {
            card: Card {
                name: String::from("Blood Celebrant"),
                class: CardType::Creature,
                cost: CMC::new("B").to_mana(),
                zone: Zone::Battlefield,
                color: HashSet::from([Color::Black]),
                flavor: None,
                abilities: Abilities::new(),
            },
            permanent: Rc::new(RefCell::new(Permanent::new())),
            power: 1,
            toughness: 1,
        };

        let tap = Tap { target: creature.permanent.clone() };
        let life_cost = LifeCost { player: player.clone(), cost: 1 };
        let cost = MultiCost { items: vec![Box::new(tap), Box::new(life_cost)] };
        let effect = ManaEffect { player: player.clone(), mana: Mana::from([(Color::Green, 1)]) };
        let mana_ability = Activate { cost: Box::new(cost), effect: Rc::new(effect) };

        creature.card.abilities.activated.push(mana_ability);
        creature
    }
}