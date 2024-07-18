use std::collections::{HashMap, HashSet};

use crate::{
    abilities::Effect,
    action::Action,
    card::{put_on_graveyard, untap_card, CardType, Zone},
    deck::draw_card,
    events::{Event, PhaseEvent},
    game::{dispatch_event, take_damage, Game, ObjectId},
};

pub struct Turn {
    pub phase: Phase,
    pub combat: Combat,
    pub active_player: ObjectId,
}

impl Turn {
    pub fn new(player_id: ObjectId) -> Turn {
        Turn {
            phase: Phase::Untap,
            combat: Combat::new(),
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

pub struct Combat {
    pub attackers: HashMap<ObjectId, ObjectId>,
    pub blockers: HashMap<ObjectId, Vec<ObjectId>>,
}

impl Combat {
    pub fn new() -> Combat {
        Combat {
            attackers: HashMap::new(),
            blockers: HashMap::new(),
        }
    }
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
        if let Some(card) = game.get_card(card_id) {
            if let CardType::Creature(creature) = &mut card.kind {
                creature.current.motion_sickness = false;
            }
        }
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

pub fn declare_attackers_phase_begin(game: &mut Game) {
    game.turn.phase = Phase::DeclareAttackers;
}

pub fn declare_attacker(game: &mut Game, attacker_id: ObjectId, target: ObjectId) {
    if can_declare_attacker(game, attacker_id) {
        game.turn.combat.attackers.insert(attacker_id, target);
    }
}

pub fn can_declare_attacker(game: &mut Game, card_id: ObjectId) -> bool {
    let active_player = game.turn.active_player;

    if let Some(card) = game.get_card(card_id) {
        if card.owner_id != active_player || card.zone != Zone::Battlefield {
            return false;
        }
        if let CardType::Creature(creature) = &card.kind {
            return !card.tapped && !creature.current.motion_sickness;
        }
    }
    false
}

pub fn declare_attackers_phase_end(game: &mut Game) -> Priority {
    for attacker in game.turn.combat.attackers.clone().keys() {
        if let Some(card) = game.get_card(*attacker) {
            card.tap();
        }
    }

    dispatch_event(
        game,
        Event::Phase(PhaseEvent {
            owner: game.turn.active_player,
            phase: Phase::DeclareAttackers,
        }),
    );
    Priority::new(game.turn.active_player)
}

pub fn declare_blockers_phase_begin(game: &mut Game) {
    game.turn.phase = Phase::DeclareBlockers;
}

pub fn declare_blocker(game: &mut Game, blocker: ObjectId, attacker: ObjectId) {
    if can_declare_blocker(game, blocker, attacker) {
        let mut blockers = game
            .turn
            .combat
            .blockers
            .remove(&attacker)
            .unwrap_or(vec![]);
        blockers.push(blocker);

        game.turn.combat.blockers.insert(attacker, blockers);
    }
}

pub fn can_declare_blocker(game: &mut Game, blocker: ObjectId, attacker: ObjectId) -> bool {
    let defending_player = if let Some(player_id) = game.turn.combat.attackers.get(&attacker) {
        player_id.clone()
    } else {
        return false;
    };

    if let Some(card) = game.get_card(blocker) {
        if card.owner_id != defending_player || card.zone != Zone::Battlefield {
            return false;
        }
        if let CardType::Creature(_) = &card.kind {
            return !card.tapped;
        }
    }
    false
}

pub fn declare_blockers_phase_end(game: &mut Game) -> Priority {
    dispatch_event(
        game,
        Event::Phase(PhaseEvent {
            owner: game.turn.active_player,
            phase: Phase::DeclareBlockers,
        }),
    );
    Priority::new(game.turn.active_player)
}

pub fn combat(game: &mut Game) {
    // TODO: Implement combat damage distribution

    game.turn.phase = Phase::Combat;

    let attackers = game.turn.combat.attackers.clone();
    let blockers = game.turn.combat.blockers.clone();

    for (attacker, attack_target) in attackers.iter() {
        let mut attack_damage = 0;
        if let Some(card) = game.get_card(*attacker) {
            if let CardType::Creature(creature) = &card.kind {
                attack_damage = creature.current.power
            }
        }

        if let Some(attacker_blockers) = blockers.get(attacker) {
            for blocker in attacker_blockers.iter() {
                let mut block_damage = 0;

                // Attacker is blocked by another creature.
                // Blocker takes damage.
                if let Some(card) = game.get_card(*blocker) {
                    if let CardType::Creature(creature) = &mut card.kind {
                        block_damage = creature.current.power;
                        creature.current.toughness -= attack_damage;
                    }
                }

                // Attacker takes damage.
                if let Some(card) = game.get_card(*attacker) {
                    if let CardType::Creature(creature) = &mut card.kind {
                        creature.current.toughness -= block_damage;
                    }
                }
            }
        } else {
            // Attacker is not blocked, the defending player takes damage.
            take_damage(game, *attack_target, attack_damage as u16);
        }
    }

    let mut dead = vec![];
    for combatant in attackers.keys().chain(blockers.values().flatten()) {
        if let Some(card) = game.get_card(*combatant) {
            if let CardType::Creature(creature) = &mut card.kind {
                if creature.current.toughness <= 0 {
                    dead.push(combatant);
                }
            }
        }
    }

    for card_id in dead {
        put_on_graveyard(game, *card_id);
    }
}

pub fn postcombat_phase(game: &mut Game) -> Priority {
    change_phase(game, Phase::Postcombat)
}

pub fn end_phase(game: &mut Game) -> Priority {
    change_phase(game, Phase::End)
}

pub fn cleanup_phase(game: &mut Game) -> Option<Action> {
    game.turn.phase = Phase::Cleanup;

    for card in game.cards.values_mut() {
        if let CardType::Creature(creature) = &mut card.kind {
            creature.restore();
        }
    }

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

pub fn pass_turn(game: &mut Game) -> bool {
    let mut passed = false;

    for player_id in game.get_player_ids() {
        if player_id != game.turn.active_player {
            game.turn = Turn::new(player_id);
            passed = true;
        }
    }
    passed
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
        card::{put_on_battlefield, Card, CardType, CreatureState, Zone},
        game::{Game, Player},
        turn::{all_passed, declare_blocker, Turn},
    };

    use super::{
        combat, declare_attacker, declare_attackers_phase_begin, declare_attackers_phase_end,
        declare_blockers_phase_begin, declare_blockers_phase_end, Priority,
    };

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

    #[test]
    fn test_combat_no_blockers() {
        let mut game = Game::new();
        let player_id = game.add_player(Player::new());
        let opponent_id = game.add_player(Player::new());
        game.turn = Turn::new(player_id);

        let mut creature_state = CreatureState::new(3, 1);
        creature_state.default.motion_sickness = false;

        let mut card = Card::new();
        card.owner_id = player_id;
        card.kind = CardType::Creature(creature_state);
        let attacker_id = game.add_card(card);

        put_on_battlefield(&mut game, attacker_id);
        declare_attackers_phase_begin(&mut game);
        declare_attacker(&mut game, attacker_id, opponent_id);
        declare_attackers_phase_end(&mut game);
        declare_blockers_phase_begin(&mut game);
        declare_blockers_phase_end(&mut game);
        combat(&mut game);

        let card = game.get_card(attacker_id).unwrap();
        assert!(card.tapped);

        let opponent = game.get_player(opponent_id).unwrap();
        assert_eq!(opponent.life, 17);
    }

    #[test]
    fn test_combat_with_blocker() {
        let mut game = Game::new();
        let player_id = game.add_player(Player::new());
        let opponent_id = game.add_player(Player::new());
        game.turn = Turn::new(player_id);

        let mut creature_state = CreatureState::new(3, 1);
        creature_state.default.motion_sickness = false;

        let mut card = Card::new();
        card.owner_id = player_id;
        card.kind = CardType::Creature(creature_state);
        let attacker_id = game.add_card(card);
        put_on_battlefield(&mut game, attacker_id);

        let mut card = Card::new();
        card.owner_id = opponent_id;
        card.kind = CardType::Creature(CreatureState::new(2, 2));
        let blocker_id = game.add_card(card);
        put_on_battlefield(&mut game, blocker_id);

        declare_attackers_phase_begin(&mut game);
        declare_attacker(&mut game, attacker_id, opponent_id);
        declare_attackers_phase_end(&mut game);
        declare_blockers_phase_begin(&mut game);
        declare_blocker(&mut game, blocker_id, attacker_id);
        declare_blockers_phase_end(&mut game);
        combat(&mut game);

        let card = game.get_card(attacker_id).unwrap();
        assert!(card.zone == Zone::Graveyard);

        let card = game.get_card(blocker_id).unwrap();
        assert!(card.zone == Zone::Graveyard);

        let opponent = game.get_player(opponent_id).unwrap();
        assert_eq!(opponent.life, 20);
    }

    #[test]
    fn test_combat_with_multiple_blockers() {
        let mut game = Game::new();
        let player_id = game.add_player(Player::new());
        let opponent_id = game.add_player(Player::new());
        game.turn = Turn::new(player_id);

        let mut creature_state = CreatureState::new(3, 3);
        creature_state.default.motion_sickness = false;

        let mut card = Card::new();
        card.owner_id = player_id;
        card.kind = CardType::Creature(creature_state);
        let attacker_id = game.add_card(card);
        put_on_battlefield(&mut game, attacker_id);

        let mut card = Card::new();
        card.owner_id = opponent_id;
        card.kind = CardType::Creature(CreatureState::new(1, 2));
        let blocker_one = game.add_card(card);
        put_on_battlefield(&mut game, blocker_one);

        let mut card = Card::new();
        card.owner_id = opponent_id;
        card.kind = CardType::Creature(CreatureState::new(2, 2));
        let blocker_two = game.add_card(card);
        put_on_battlefield(&mut game, blocker_two);

        declare_attackers_phase_begin(&mut game);
        declare_attacker(&mut game, attacker_id, opponent_id);
        declare_attackers_phase_end(&mut game);
        declare_blockers_phase_begin(&mut game);
        declare_blocker(&mut game, blocker_one, attacker_id);
        declare_blocker(&mut game, blocker_two, attacker_id);
        declare_blockers_phase_end(&mut game);
        combat(&mut game);

        let card = game.get_card(attacker_id).unwrap();
        assert!(card.zone == Zone::Graveyard);

        // TODO: Test combat damage distribution
        let opponent = game.get_player(opponent_id).unwrap();
        assert_eq!(opponent.life, 20);
    }
}
