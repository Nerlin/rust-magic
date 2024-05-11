use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;

use crate::abilities::{Abilities, Activate};
use crate::cost::Tap;
use crate::effects::{Effect, ManaEffect};
use crate::mana::{Color, Mana};
use crate::player::Player;
use crate::zones::Zone;

pub struct Card {
    pub name: String,
    pub class: CardType,
    pub cost: Mana,
    pub flavor: Option<String>,
    pub color: HashSet<Color>,
    pub abilities: Abilities,
    pub state: Rc<RefCell<CardState>>,
}

impl Card {
    pub fn basic_land(name: &str, owner: Player, mana: Mana) -> Card {
        let mut land = Card {
            name: String::from(name),
            class: CardType::Land,
            flavor: None,
            cost: Mana::new(),
            color: HashSet::from([Color::Colorless]),
            abilities: Abilities::new(),
            state: Rc::new(RefCell::new(CardState { zone: Zone::None })),
        };
        let tap = Tap {};
        let effect = ManaEffect { player: owner, mana };
        let mana_ability = Activate { cost: Box::new(tap), effect: Rc::new(effect) };

        land.abilities.activated.push(mana_ability);
        land
    }

    pub fn activate(&self, index: usize) -> Option<Rc<dyn Effect>> {
        if let Some(ability) = self.abilities.activated.get(index) {
            ability.activate(self.state.clone())
        } else {
            None
        }
    }
}

pub struct CardState {
    pub zone: Zone,
}

impl CardState {
    pub fn as_permanent(&mut self) -> Option<&mut Permanent> {
        return if let Zone::Battlefield(ref mut permanent) = &mut self.zone {
            Some(permanent)
        } else {
            None
        }
    }
}

pub enum CardType {
    Land,
    Creature(Creature),
    Sorcery,
    Instant,
    Enchantment,
    Artifact,
}

#[derive(Clone, Debug, Ord, PartialOrd, Eq, PartialEq)]
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

pub struct Creature {
    pub power: i16,
    pub toughness: i16,
    pub state: Rc<RefCell<CreatureState>>,
}

pub struct CreatureState {
    pub power: i16,
    pub toughness: i16,
}

impl Creature {
    pub fn new(power: i16, toughness: i16) -> Creature {
        Creature {
            power,
            toughness,
            state: Rc::new(RefCell::new(CreatureState { power, toughness }))
        }
    }

    pub fn reset(&mut self) {
        let mut state = self.state.borrow_mut();
        state.power = self.power;
        state.toughness = self.toughness;
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::collections::HashSet;
    use std::rc::Rc;

    use crate::abilities::{Abilities, Activate};
    use crate::cards::{Card, CardState, CardType, Creature, Permanent};
    use crate::cost::{LifeCost, MultiCost, Tap};
    use crate::effects::ManaEffect;
    use crate::mana::{CMC, Color, Mana};
    use crate::player::{new_player, Player};
    use crate::zones::Zone;

    #[test]
    fn test_basic_land() {
        let player = new_player();
        let forest = Card::basic_land("Forest", player.clone(), CMC::new("G").to_mana());
        forest.state.borrow_mut().zone = Zone::Battlefield(Permanent::new());

        let effect = forest.activate(0).unwrap();
        effect.resolve();

        assert_eq!(player.borrow_mut().mana, Mana::from([
            (Color::Green, 1)
        ]));
    }

    #[test]
    fn test_basic_land_tapped() {
        let player = new_player();
        let mut mountain =  Card::basic_land("Mountain", player.clone(), CMC::new("R").to_mana());
        let mut permanent = Permanent::new();
        permanent.tap();

        mountain.state.borrow_mut().zone = Zone::Battlefield(permanent);

        let mana_ability = mountain.abilities.activated.get_mut(0).unwrap();
        let effect = mana_ability.activate(mountain.state.clone());
        assert!(effect.is_none())
    }

    #[test]
    fn test_basic_land_untapped() {
        let player = new_player();
        let mut island = Card::basic_land("Island", player.clone(), CMC::new("U").to_mana());
        island.state.borrow_mut().zone = Zone::Battlefield(Permanent::new());

        let mana_ability = island.abilities.activated.get_mut(0).unwrap();
        mana_ability.activate(island.state.clone()).unwrap().resolve();

        island.state.borrow_mut().as_permanent().unwrap().untap();

        mana_ability.activate(island.state.clone()).unwrap().resolve();
        assert_eq!(player.borrow_mut().mana, Mana::from([
            (Color::Blue, 2)
        ]));
    }

    #[test]
    fn test_blightsoil_druid() {
        let player = new_player();

        let mut creature = create_blightsoil_druid(player.clone());
        let mana_ability = creature.abilities.activated.get_mut(0).unwrap();
        let effect = mana_ability.activate(creature.state.clone()).unwrap();
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

        let creature = create_blightsoil_druid(player.clone());
        let mana_ability = creature.abilities.activated.get(0).unwrap();
        let effect = mana_ability.activate(creature.state.clone());

        assert!(effect.is_none());
    }

    #[test]
    fn test_blightsoil_druid_tapped() {
        let player = new_player();

        let mut creature = create_blightsoil_druid(player.clone());
        creature.state.borrow_mut().as_permanent().unwrap().tap();

        let mana_ability = creature.abilities.activated.get_mut(0).unwrap();
        let effect = mana_ability.activate(creature.state.clone());

        assert!(effect.is_none());
    }

    #[test]
    fn test_creature_reset() {
        let player = new_player();
        let mut card = create_blightsoil_druid(player.clone());
        if let CardType::Creature(creature) = &mut card.class {
            creature.state.borrow_mut().power = 0;
            creature.state.borrow_mut().toughness = 0;
            creature.reset();

            assert_eq!(creature.state.borrow().power, creature.power);
            assert_eq!(creature.state.borrow().toughness, creature.toughness);
        }
    }

    fn create_blightsoil_druid(player: Player) -> Card {
        let mut creature = Card {
            name: String::from("Blood Celebrant"),
            class: CardType::Creature(Creature::new(1, 1)),
            cost: CMC::new("B").to_mana(),
            color: HashSet::from([Color::Black]),
            flavor: None,
            abilities: Abilities::new(),
            state: Rc::new(RefCell::new(CardState { zone: Zone::Battlefield(Permanent::new()) }))
        };

        let tap = Tap {};
        let life_cost = LifeCost { player: player.clone(), cost: 1 };
        let cost = MultiCost { items: vec![Box::new(tap), Box::new(life_cost)] };
        let effect = ManaEffect { player: player.clone(), mana: Mana::from([(Color::Green, 1)]) };
        let mana_ability = Activate { cost: Box::new(cost), effect: Rc::new(effect) };

        creature.abilities.activated.push(mana_ability);
        creature
    }
}