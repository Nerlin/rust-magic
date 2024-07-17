use std::collections::HashSet;

use crate::{
    abilities::Effect,
    action::Action,
    card::{untap_card, Zone},
    deck::draw_card,
    events::{Event, PhaseEvent},
    game::{dispatch_event, Game, ObjectId},
};

pub struct Turn {
    pub phase: Phase,
    pub active_player: ObjectId,
}

impl Turn {
    pub fn new(player_id: ObjectId) -> Turn {
        Turn {
            phase: Phase::Untap,
            active_player: player_id,
        }
    }
}

pub struct Priority {
    pub player_id: ObjectId,
    passes: HashSet<ObjectId>,
}

impl Priority {
    pub fn new(active_player: ObjectId) -> Priority {
        Priority {
            player_id: active_player,
            passes: HashSet::new(),
        }
    }

    pub fn pass(&mut self, next_player: ObjectId) {
        self.passes.insert(self.player_id);
        self.player_id = next_player;
    }

    pub fn reset(&mut self) {
        self.passes.clear();
    }

    pub fn passed(&self, player_id: ObjectId) -> bool {
        self.passes.contains(&player_id)
    }
}

#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub enum Phase {
    Untap,
    Upkeep,
    Draw,
    Precombat,
    DeclareAttackers,
    DeclareBlockers,
    Combat,
    Postcombat,
    End,
    Cleanup,
}

pub fn all_passed(game: &Game, priority: Priority) -> bool {
    for player in game.players.iter() {
        if !priority.passed(player.id) {
            return false;
        }
    }
    true
}

pub fn untap_phase(game: &mut Game) {
    game.turn.phase = Phase::Untap;

    let tapped_cards: Vec<ObjectId> = game
        .cards
        .values()
        .filter(|card| {
            card.owner_id == game.turn.active_player
                && card.zone == Zone::Battlefield
                && card.tapped
        })
        .map(|card| card.id)
        .collect();

    for card_id in tapped_cards {
        untap_card(game, card_id, None);
    }

    dispatch_event(
        game,
        Event::Phase(PhaseEvent {
            owner: game.turn.active_player,
            phase: Phase::Untap,
        }),
    );
}

pub fn upkeep_phase(game: &mut Game) -> Priority {
    change_phase(game, Phase::Upkeep)
}

pub fn draw_phase(game: &mut Game) -> Priority {
    game.turn.phase = Phase::Draw;
    draw_card(game, game.turn.active_player);

    dispatch_event(
        game,
        Event::Phase(PhaseEvent {
            owner: game.turn.active_player,
            phase: Phase::Draw,
        }),
    );
    Priority::new(game.turn.active_player)
}

pub fn precombat_phase(game: &mut Game) -> Priority {
    change_phase(game, Phase::Precombat)
}

pub fn postcombat_phase(game: &mut Game) -> Priority {
    change_phase(game, Phase::Postcombat)
}

pub fn end_phase(game: &mut Game) -> Priority {
    change_phase(game, Phase::End)
}

pub fn cleanup_phase(game: &mut Game) -> Option<Action> {
    game.turn.phase = Phase::Cleanup;

    if let Some(player) = game.get_player(game.turn.active_player) {
        let hand_size = player.hand.len();
        if hand_size > player.max_hand_size {
            let mut action = Action::new(player.id, 0);
            action.set_required_effect(Effect::Discard(hand_size - player.max_hand_size));

            return Some(action);
        }
    }
    None
}

fn change_phase(game: &mut Game, phase: Phase) -> Priority {
    game.turn.phase = phase.clone();

    dispatch_event(
        game,
        Event::Phase(PhaseEvent {
            owner: game.turn.active_player,
            phase,
        }),
    );
    Priority::new(game.turn.active_player)
}

#[cfg(test)]
mod tests {
    use crate::{
        game::{Game, Player},
        turn::all_passed,
    };

    use super::Priority;

    #[test]
    fn test_priority_pass() {
        let mut game = Game::new();
        let player_id = game.add_player(Player::new());
        let opponent_id = game.add_player(Player::new());

        let mut priority = Priority::new(player_id);
        priority.pass(opponent_id);

        assert!(priority.passed(player_id));
        assert_eq!(priority.player_id, opponent_id);
    }

    #[test]
    fn test_priority_all_passed() {
        let mut game = Game::new();
        let player_id = game.add_player(Player::new());
        let opponent_id = game.add_player(Player::new());

        let mut priority = Priority::new(player_id);
        priority.pass(opponent_id);
        priority.pass(player_id);

        assert!(all_passed(&game, priority));
    }
}
