use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;

use rand::random;

use crate::abilities::{Abilities, Activate};
use crate::cost::Tap;
use crate::effects::{Alive, Effect, ManaEffect};
use crate::events::{Event, EventLoop, EventResult};
use crate::mana::{Color, Mana};
use crate::player::Player;
use crate::zones::Zone;

pub struct Card {
    pub name: String,
    pub class: CardType,
    pub cost: Mana,
    pub flavor: String,
    pub color: HashSet<Color>,
    pub abilities: Abilities,
    pub state: Rc<RefCell<CardState>>,
}

impl Card {
    pub fn basic_land(
        name: &str,
        owner: Player,
        mana: Mana,
        event_loop: Rc<RefCell<EventLoop>>,
    ) -> Card {
        let mut land = Card {
            name: String::from(name),
            class: CardType::Land,
            flavor: String::new(),
            cost: Mana::new(),
            color: HashSet::from([Color::Colorless]),
            state: Rc::new(RefCell::new(CardState::new(event_loop))),
            abilities: Abilities::new(),
        };
        let tap = Tap {};
        let effect = ManaEffect {
            player: owner.clone(),
            mana,
        };
        let mana_ability = Activate {
            cost: Box::new(tap),
            effect: Rc::new(effect),
        };

        land.abilities.activated.push(mana_ability);
        land
    }

    pub fn id(&self) -> u64 {
        self.state.borrow().id
    }

    pub fn activate(&self, index: usize) -> Option<Rc<dyn Effect>> {
        if let Some(ability) = self.abilities.activated.get(index) {
            ability.activate(self.state.clone())
        } else {
            None
        }
    }

    pub fn attach(&mut self, event_loop: Rc<RefCell<EventLoop>>) {
        for ability in self.abilities.triggers.iter() {
            event_loop
                .borrow_mut()
                .subscribe(ability.event.clone(), ability.clone());
        }
    }
}

pub type CardStateRef = Rc<RefCell<CardState>>;

pub fn new_card_state(event_loop: Rc<RefCell<EventLoop>>) -> CardStateRef {
    Rc::new(RefCell::new(CardState::new(event_loop)))
}

pub struct CardState {
    pub id: u64,
    pub events: Rc<RefCell<EventLoop>>,
    pub zone: Zone,
    permanent: Permanent,
}

impl CardState {
    pub fn new(events: Rc<RefCell<EventLoop>>) -> CardState {
        CardState {
            id: random(),
            zone: Zone::None,
            permanent: Permanent::new(),
            events,
        }
    }

    pub fn tap(&mut self) -> bool {
        if self.zone != Zone::Battlefield || self.permanent.tapped {
            return false;
        }

        let events = &self.events.borrow();
        let result = events.emit(Event::PermanentTap(self.id));
        if result.is_prevented() {
            return false;
        }

        self.permanent.tap()
    }

    pub fn untap(&mut self) -> bool {
        if self.zone != Zone::Battlefield || !self.permanent.tapped {
            return false;
        }

        let events = &self.events.borrow();
        let result = events.emit(Event::PermanentUntap(self.id));
        if result.is_prevented() {
            return false;
        }

        self.permanent.untap()
    }

    pub fn put_on_battlefield(&mut self) -> EventResult {
        let events = &self.events.borrow();
        let result = events.emit(Event::PermanentCreate(self.id));
        if let EventResult::Resolved = result {
            self.permanent.reset();
            self.zone = Zone::Battlefield;
            EventResult::Resolved
        } else {
            result
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

    pub fn reset(&mut self) {
        self.tapped = false;
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

pub type Creature = Rc<RefCell<CreatureState>>;

pub struct CreatureState {
    pub power: i16,
    pub toughness: i16,
    pub current_power: i16,
    pub current_toughness: i16,
}

impl CreatureState {
    pub fn new(power: i16, toughness: i16) -> CreatureState {
        CreatureState {
            power,
            toughness,
            current_power: power,
            current_toughness: toughness,
        }
    }

    pub fn reset(&mut self) {
        self.current_power = self.power;
        self.current_toughness = self.toughness;
    }
}

impl Alive for CreatureState {
    fn gain_life(&mut self, life: u16) {
        self.current_toughness += life as i16;
    }

    fn lose_life(&mut self, life: u16) {
        self.current_toughness -= life as i16;
    }

    fn take_damage(&mut self, damage: u16) {
        self.current_toughness -= damage as i16;
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::collections::HashSet;
    use std::rc::Rc;

    use crate::abilities::{Abilities, Activate, Trigger};
    use crate::cards::{Card, CardType, CreatureState, new_card_state};
    use crate::cost::{LifeCost, MultiCost, Tap};
    use crate::effects::{DamageEffect, ManaEffect};
    use crate::events::Event;
    use crate::game::{Game, GameObject, GameState, new_game};
    use crate::mana::{Color, Mana};
    use crate::player::{new_player, Player, PlayerState};
    use crate::zones::Zone;

    #[test]
    fn test_basic_land() {
        let game = new_game();
        let player = new_player();
        let forest = Card::basic_land(
            "Forest",
            player.clone(),
            Mana::from("G"),
            game.borrow().events()
        );
        forest.state.borrow_mut().zone = Zone::Battlefield;

        let effect = forest.activate(0).unwrap();
        effect.resolve();

        assert_eq!(player.borrow_mut().mana, Mana::from([(Color::Green, 1)]));
    }

    #[test]
    fn test_basic_land_tapped() {
        let game = new_game();
        let player = new_player();
        let mountain = Card::basic_land(
            "Mountain",
            player.clone(),
            Mana::from("R"),
            game.borrow().events()
        );

        mountain.state.borrow_mut().zone = Zone::Battlefield;
        mountain.state.borrow_mut().tap();

        let mana_ability = mountain.abilities.activated.get(0).unwrap();
        let effect = mana_ability.activate(mountain.state.clone());
        assert!(effect.is_none())
    }

    #[test]
    fn test_basic_land_untapped() {
        let game = new_game();
        let player = new_player();
        let island = Card::basic_land(
            "Island",
            player.clone(),
            Mana::from("U"),
            game.borrow().events()
        );
        island.state.borrow_mut().zone = Zone::Battlefield;

        let mana_ability = island.abilities.activated.get(0).unwrap();
        mana_ability
            .activate(island.state.clone())
            .unwrap()
            .resolve();

        island.state.borrow_mut().untap();

        mana_ability
            .activate(island.state.clone())
            .unwrap()
            .resolve();
        assert_eq!(player.borrow_mut().mana, Mana::from([(Color::Blue, 2)]));
    }

    #[test]
    fn test_blightsoil_druid() {
        let game = new_game();
        let player = new_player();

        let creature = create_blightsoil_druid(player.clone(), game.clone());
        creature.state.borrow_mut().zone = Zone::Battlefield;

        let mana_ability = creature.abilities.activated.get(0).unwrap();
        let effect = mana_ability.activate(creature.state.clone()).unwrap();
        effect.resolve();

        assert_eq!(player.borrow_mut().mana, Mana::from([(Color::Green, 1)]));
    }

    #[test]
    fn test_blightsoil_druid_low_life() {
        let game = new_game();
        let player = new_player();
        player.borrow_mut().life = 0;

        let creature = create_blightsoil_druid(player.clone(), game.clone());
        let mana_ability = creature.abilities.activated.get(0).unwrap();
        let effect = mana_ability.activate(creature.state.clone());

        assert!(effect.is_none());
    }

    #[test]
    fn test_blightsoil_druid_tapped() {
        let game = new_game();
        let player = new_player();

        let creature = create_blightsoil_druid(player.clone(), game.clone());
        creature.state.borrow_mut().tap();

        let mana_ability = creature.abilities.activated.get(0).unwrap();
        let effect = mana_ability.activate(creature.state.clone());

        assert!(effect.is_none());
    }

    #[test]
    fn test_creature_reset() {
        let game = new_game();
        let player = new_player();
        let mut card = create_blightsoil_druid(player.clone(), game.clone());
        if let CardType::Creature(creature) = &mut card.class {
            creature.borrow_mut().current_power = 0;
            creature.borrow_mut().current_toughness = 0;
            creature.borrow_mut().reset();

            let state = creature.borrow();

            assert_eq!(state.power, state.current_power);
            assert_eq!(state.toughness, state.current_toughness);
        }
    }

    #[test]
    fn test_city_of_brass_trigger() {
        let game = Rc::new(RefCell::new(GameState::new()));

        let mut card = create_city_of_brass(game.borrow().turn.ap.clone(), game.clone());
        card.attach(game.borrow().events.clone());
        card.state.borrow_mut().put_on_battlefield();
        card.state.borrow_mut().tap();

        game.borrow().stack().borrow_mut().resolve();
        assert_eq!(game.borrow().turn.ap.borrow().life, PlayerState::START_LIFE - 1);
    }

    fn create_blightsoil_druid(player: Player, game: Game) -> Card {
        let mut creature = Card {
            name: String::from("Blood Celebrant"),
            class: CardType::Creature(Rc::new(RefCell::new(CreatureState::new(1, 1)))),
            cost: Mana::from("B"),
            color: HashSet::from([Color::Black]),
            flavor: String::new(),
            state: new_card_state(game.borrow().events.clone()),
            abilities: Abilities::new(),
        };

        let tap = Tap {};
        let life_cost = LifeCost {
            player: player.clone(),
            cost: 1,
        };
        let cost = MultiCost {
            items: vec![Box::new(tap), Box::new(life_cost)],
        };
        let effect = ManaEffect {
            player: player.clone(),
            mana: Mana::from([(Color::Green, 1)]),
        };
        let mana_ability = Activate {
            cost: Box::new(cost),
            effect: Rc::new(effect),
        };

        creature.abilities.activated.push(mana_ability);
        creature
    }

    fn create_city_of_brass(player: Player, game: Game) -> Card {
        let mut land = Card {
            name: String::from("City of Brass"),
            class: CardType::Land,
            cost: Mana::new(),
            color: HashSet::from([Color::Colorless]),
            flavor: String::from(
                "There is so much to learn here, but few can endure the ringing of the spires.",
            ),
            state: new_card_state(game.borrow().events.clone()),
            abilities: Abilities::new(),
        };

        let trigger = Trigger::new(
            Event::PermanentTap(land.id()),
            Rc::new(DamageEffect {
                game: game.clone(),
                target: GameObject::Player(player.borrow().id),
                damage: 1,
            }),
            game.borrow().stack.clone(),
        );

        land.abilities.triggers.push(Rc::new(trigger));
        land
    }
}
