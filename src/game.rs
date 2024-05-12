use std::cell::RefCell;
use std::rc::Rc;

use crate::cards::{CardStateRef, CardType, Creature};
use crate::events::{Event, EventLoop, Stack};
use crate::player::{new_player, Player};

pub type Game = Rc<RefCell<GameState>>;

pub fn new_game() -> Game {
    Rc::new(RefCell::new(GameState::new()))
}

pub struct GameState {
    pub players: Vec<Player>,
    pub stack: Rc<RefCell<Stack>>,
    pub turn: Turn,
    pub events: Rc<RefCell<EventLoop>>
}

impl GameState {
    pub fn new() -> GameState {
        let ap = new_player();
        let nap = new_player();
        let players = vec![
            ap.clone(),
            nap.clone(),
        ];

        GameState {
            players,
            stack: Rc::new(RefCell::new(Stack::new())),
            turn: Turn::new(ap.clone(), nap.clone()),
            events: Rc::new(RefCell::new(EventLoop::new())),
        }
    }

    pub fn stack(&self) -> Rc<RefCell<Stack>> {
        self.stack.clone()
    }

    pub fn events(&self) -> Rc<RefCell<EventLoop>> {
        self.events.clone()
    }

    pub fn get_player(&self, id: u64) -> Option<Player> {
        for player in self.players.iter() {
            if player.borrow().id == id {
                return Some(player.clone());
            }
        }
        None
    }

    pub fn get_card(&self, id: u64) -> Option<CardStateRef> {
        for player in self.players.iter() {
            let player_state = player.borrow();
            for card in player_state.battlefield.iter() {
                if card.state.borrow().id == id {
                    return Some(card.state.clone());
                }
            }
        }
        None
    }

    pub fn get_creature(&self, id: u64) -> Option<Creature> {
        for player in self.players.iter() {
            let player_state = player.borrow();
            for card in player_state.battlefield.iter() {
                if card.state.borrow().id == id {
                    if let CardType::Creature(creature) = &card.class {
                        return Some(creature.clone());
                    }
                }
            }
        }
        None
    }
}

#[derive(Clone, Debug, Ord, PartialOrd, Eq, PartialEq)]
pub enum Phase {
    Untap,
    Upkeep,
    Draw ,
    Precombat,
    Attackers ,
    Blockers,
    Combat,
    Postcombat,
    End,
    Cleanup
}

pub struct Turn {
    pub phase: Phase,

    // Active player
    pub ap: Player,

    // Non-active player
    pub nap: Player
}

impl Turn {
    pub fn new(active_player: Player, non_active_player: Player) -> Turn {
        Turn {
            phase: Phase::Untap,
            ap: active_player,
            nap: non_active_player
        }
    }

    pub fn start(&mut self) {
        self.untap();
        self.upkeep();
        self.draw();
        self.precombat();
        self.declare_attackers();
        self.declare_blockers();
        self.combat();
        self.postcombat();
        self.end();
        self.cleanup();
    }

    pub fn untap(&mut self) {
        self.phase = Phase::Untap;
        for card in &mut self.ap.borrow_mut().battlefield {
            card.state.borrow_mut().untap();
        }
    }

    pub fn upkeep(&mut self) {
        self.phase = Phase::Upkeep;
        for card in &mut self.ap.borrow_mut().battlefield {
            card.abilities.run_triggers(Event::PhaseUpkeep);
        }
    }

    pub fn draw(&mut self) {
        self.phase = Phase::Draw;
        for card in &mut self.ap.borrow_mut().battlefield {
            card.abilities.run_triggers(Event::PhaseDraw);
        }
    }

    pub fn precombat(&mut self) {
        self.phase = Phase::Precombat;
        for card in &mut self.ap.borrow_mut().battlefield {
            card.abilities.run_triggers(Event::PhasePrecombat);
        }
    }

    pub fn declare_attackers(&mut self) {
        self.phase = Phase::Attackers;
    }

    pub fn declare_blockers(&mut self) {
        self.phase = Phase::Blockers;
    }

    pub fn combat(&mut self) {
        self.phase = Phase::Combat;
    }

    pub fn postcombat(&mut self) {
        self.phase = Phase::Postcombat;
        for card in &mut self.ap.borrow_mut().battlefield {
            card.abilities.run_triggers(Event::PhasePostcombat);
        }
    }

    pub fn end(&mut self) {
        self.phase = Phase::End;
        for card in &mut self.ap.borrow_mut().battlefield {
            card.abilities.run_triggers(Event::PhaseEnd);
        }
    }

    pub fn cleanup(&mut self) {
        self.phase = Phase::Cleanup;
        for card in &mut self.ap.borrow_mut().battlefield {
            if let CardType::Creature(creature) = &mut card.class {
                creature.borrow_mut().reset();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::game::{Phase, Turn};
    use crate::player::new_player;

    #[test]
    fn test_turn_start() {
        let ap = new_player();
        let nap = new_player();
        let mut turn = Turn::new(ap, nap);
        turn.start();
        assert_eq!(turn.phase, Phase::Cleanup);
    }
}

#[derive(Debug, Hash, PartialOrd, PartialEq, Ord, Eq)]
pub enum GameObject {
    Player(u64),
    Card(u64),
    Creature(u64)
}
