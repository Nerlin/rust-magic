use crate::cards::CardType;
use crate::events::Event;
use crate::player::Player;
use crate::zones::Zone;

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
    pub players: Vec<Player>,
    pub phase: Phase,
    pub active_player: Player,
}

impl Turn {
    pub fn new(active_player: Player, non_active_player: Player) -> Turn {
        let players = vec![
            active_player.clone(),
            non_active_player.clone(),
        ];
        Turn {
            players,
            phase: Phase::Untap,
            active_player,
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
        for card in &mut self.active_player.borrow_mut().battlefield {
            if let Zone::Battlefield(ref mut permanent) = &mut card.state.borrow_mut().zone {
                permanent.untap();
            }
        }
    }

    pub fn upkeep(&mut self) {
        self.phase = Phase::Upkeep;
        for card in &mut self.active_player.borrow_mut().battlefield {
            card.abilities.run_triggers(Event::PhaseUpkeep);
        }
    }

    pub fn draw(&mut self) {
        self.phase = Phase::Draw;
        for card in &mut self.active_player.borrow_mut().battlefield {
            card.abilities.run_triggers(Event::PhaseDraw);
        }
    }

    pub fn precombat(&mut self) {
        self.phase = Phase::Precombat;
        for card in &mut self.active_player.borrow_mut().battlefield {
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
        for card in &mut self.active_player.borrow_mut().battlefield {
            card.abilities.run_triggers(Event::PhasePostcombat);
        }
    }

    pub fn end(&mut self) {
        self.phase = Phase::End;
        for card in &mut self.active_player.borrow_mut().battlefield {
            card.abilities.run_triggers(Event::PhaseEnd);
        }
    }

    pub fn cleanup(&mut self) {
        self.phase = Phase::Cleanup;
        for card in &mut self.active_player.borrow_mut().battlefield {
            if let CardType::Creature(creature) = &mut card.class {
                creature.reset();
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