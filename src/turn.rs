use std::cmp;

use indexmap::{IndexMap, IndexSet};

use crate::{
    abilities::{deal_player_damage, Effect, StaticAbility},
    action::Action,
    card::{is_alive, put_on_graveyard, untap_card, CardType, Zone},
    deck::draw_card,
    events::{dispatch_event, Event, PhaseEvent},
    game::{Game, ObjectId, Value},
};

pub struct Turn {
    pub step: Step,
    pub priority: Option<Priority>,
    pub combat: Combat,
    pub active_player: ObjectId,
    pub lands_played: usize,
}

impl Turn {
    pub fn new(player_id: ObjectId) -> Turn {
        Turn {
            step: Step::Untap,
            priority: None,
            combat: Combat::new(),
            active_player: player_id,
            lands_played: 0,
        }
    }
}

#[derive(Default)]
pub struct Priority {
    pub player_id: ObjectId,
    passes: IndexSet<ObjectId>,
}

impl Priority {
    pub fn new(active_player: ObjectId) -> Priority {
        Priority {
            player_id: active_player,
            passes: IndexSet::new(),
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
pub enum Step {
    Untap,
    Upkeep,
    Draw,
    Precombat,
    CombatBegin,
    DeclareAttackers,
    DeclareBlockers,
    CombatDamage,
    CombatEnd,
    Postcombat,
    End,
    Cleanup,
}

impl Step {
    pub fn main(&self) -> bool {
        self == &Step::Precombat || self == &Step::Postcombat
    }
}

#[derive(Clone)]
pub struct Combat {
    pub attackers: IndexMap<ObjectId, Attacker>,
    blockers_toughness: IndexMap<ObjectId, i16>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AttackType {
    #[default]
    Regular,
    FirstStrike,
}

impl Combat {
    pub fn new() -> Combat {
        Combat {
            attackers: IndexMap::new(),
            blockers_toughness: IndexMap::new(),
        }
    }

    pub fn get_attackers(&self) -> Vec<ObjectId> {
        self.attackers.keys().cloned().collect()
    }

    pub fn get_blockers(&self) -> Vec<ObjectId> {
        self.attackers
            .values()
            .map(|attacker| attacker.blockers.iter().cloned())
            .flatten()
            .collect()
    }

    pub fn get_combatants(&self) -> Vec<ObjectId> {
        self.get_attackers()
            .into_iter()
            .chain(self.get_blockers())
            .collect()
    }
}

#[derive(Clone, Debug, Default)]
pub struct Attacker {
    pub id: ObjectId,
    pub target: ObjectId,
    pub attacks: IndexMap<AttackType, Attack>,
    pub blockers: IndexSet<ObjectId>,
    pub blocked: bool,
}

#[derive(Clone, Debug, Default)]
pub struct Attack {
    pub power: Value<i16>,
    pub assignments: IndexMap<ObjectId, i16>,
}

impl Attack {
    pub fn new(power: i16) -> Attack {
        Attack {
            power: Value::new(power),
            assignments: IndexMap::new(),
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

pub fn untap_step(game: &mut Game) {
    game.turn.step = Step::Untap;

    let tapped_cards: Vec<ObjectId> = game
        .cards
        .values()
        .filter(|card| {
            card.owner_id == game.turn.active_player
                && card.zone == Zone::Battlefield
                && card.state.tapped.current
        })
        .map(|card| card.id)
        .collect();

    for card_id in tapped_cards {
        untap_card(game, card_id, None);
        if let Some(card) = game.get_card(card_id) {
            if card.kind == CardType::Creature {
                card.state.motion_sickness.current = false;
            }
        }
    }

    dispatch_event(
        game,
        Event::Phase(PhaseEvent {
            owner: game.turn.active_player,
            phase: Step::Untap,
        }),
    );
}

pub fn upkeep_step(game: &mut Game) {
    change_step(game, Step::Upkeep);
}

pub fn draw_step(game: &mut Game) {
    game.turn.step = Step::Draw;
    draw_card(game, game.turn.active_player);

    dispatch_event(
        game,
        Event::Phase(PhaseEvent {
            owner: game.turn.active_player,
            phase: Step::Draw,
        }),
    );
    game.turn.priority = Some(Priority::new(game.turn.active_player));
}

pub fn precombat_step(game: &mut Game) {
    change_step(game, Step::Precombat);
}

pub fn combat_begin_step(game: &mut Game) {
    change_step(game, Step::CombatBegin);
}

pub fn declare_attackers_step_start(game: &mut Game) {
    game.turn.step = Step::DeclareAttackers;
    game.turn.priority = None;
}

pub fn declare_attacker(game: &mut Game, attacker_id: ObjectId, target: ObjectId) {
    if can_declare_attacker(game, attacker_id) {
        let mut attacks: IndexMap<AttackType, Attack>;
        if let Some(card) = game.get_card(attacker_id) {
            attacks = IndexMap::new();

            let power = card.state.power.current;
            if card.static_abilities.contains(&StaticAbility::FirstStrike) {
                attacks.insert(AttackType::FirstStrike, Attack::new(power));
            } else if card.static_abilities.contains(&StaticAbility::DoubleStrike) {
                attacks.insert(AttackType::FirstStrike, Attack::new(power));
                attacks.insert(AttackType::Regular, Attack::new(power));
            } else {
                attacks.insert(AttackType::Regular, Attack::new(power));
            }
        } else {
            return;
        }

        game.turn.combat.attackers.insert(
            attacker_id,
            Attacker {
                id: attacker_id,
                target,
                attacks,
                blockers: IndexSet::new(),
                blocked: false,
            },
        );
    }
}

/// Declares the specified attacker and blocker, automatically assigns combat damage and
/// runs the combat damage step.
pub fn fast_combat(game: &mut Game, attacker_id: ObjectId, blocker_ids: &[ObjectId]) {
    fast_declare_attacker(game, attacker_id);
    fast_declare_blockers(game, blocker_ids, attacker_id);
    combat_damage_step_start(game);
    combat_damage_step_end(game);
}

pub fn fast_declare_attacker(game: &mut Game, attacker_id: ObjectId) {
    let opponent_id = game.get_next_player(game.turn.active_player);
    declare_attackers_step_start(game);
    declare_attacker(game, attacker_id, opponent_id);
    declare_attackers_step_end(game);
}

pub fn fast_declare_blockers(game: &mut Game, blocker_ids: &[ObjectId], attacker_id: ObjectId) {
    declare_blockers_step_start(game);
    for blocker_id in blocker_ids {
        declare_blocker(game, *blocker_id, attacker_id);
    }
    declare_blockers_step_end(game);
}

pub fn can_declare_attacker(game: &mut Game, card_id: ObjectId) -> bool {
    let active_player = game.turn.active_player;

    if let Some(card) = game.get_card(card_id) {
        if card.owner_id != active_player
            || card.zone != Zone::Battlefield
            || card.kind != CardType::Creature
        {
            return false;
        }
        if card.static_abilities.contains(&StaticAbility::Defender) {
            return false;
        }
        return !card.state.tapped.current && !card.state.motion_sickness.current;
    }
    false
}

pub fn declare_attackers_step_end(game: &mut Game) {
    for attacker in game.turn.combat.attackers.clone().keys() {
        if let Some(card) = game.get_card(*attacker) {
            if !card.static_abilities.contains(&StaticAbility::Vigilance) {
                card.tap();
            }
        }
    }

    dispatch_event(
        game,
        Event::Phase(PhaseEvent {
            owner: game.turn.active_player,
            phase: Step::DeclareAttackers,
        }),
    );
    game.turn.priority = Some(Priority::new(game.turn.active_player));
}

pub fn declare_blockers_step_start(game: &mut Game) {
    game.turn.step = Step::DeclareBlockers;
    game.turn.priority = None;
}

pub fn declare_blocker(game: &mut Game, blocker_id: ObjectId, attacker_id: ObjectId) {
    if can_declare_blocker(game, blocker_id, attacker_id) {
        if let Some(attacker) = game.turn.combat.attackers.get_mut(&attacker_id) {
            attacker.blockers.insert(blocker_id);
        }
    }
}

pub fn can_declare_blocker(game: &mut Game, blocker_id: ObjectId, attacker_id: ObjectId) -> bool {
    let defending_player = if let Some(attacker) = game.turn.combat.attackers.get(&attacker_id) {
        attacker.target.clone()
    } else {
        return false;
    };

    let attacker_abilities = if let Some(card) = game.get_card(attacker_id) {
        card.static_abilities.clone()
    } else {
        IndexSet::new()
    };

    if let Some(blocker) = game.get_card(blocker_id) {
        if blocker.owner_id != defending_player
            || blocker.zone != Zone::Battlefield
            || blocker.kind != CardType::Creature
        {
            return false;
        }

        for ability in attacker_abilities.iter() {
            match ability {
                StaticAbility::Flying => {
                    // Flying creatures can only be blocked by other flying creatures
                    // or by creatures with reach
                    if !blocker.static_abilities.contains(&StaticAbility::Flying)
                        && !blocker.static_abilities.contains(&StaticAbility::Reach)
                    {
                        return false;
                    }
                }
                _ => {}
            }
        }
        return true;
    }
    false
}

pub fn declare_blockers_step_end(game: &mut Game) {
    dispatch_event(
        game,
        Event::Phase(PhaseEvent {
            owner: game.turn.active_player,
            phase: Step::DeclareBlockers,
        }),
    );
    game.turn.priority = Some(Priority::new(game.turn.active_player));
}

pub fn combat_damage_step_start(game: &mut Game) {
    game.turn.step = Step::CombatDamage;

    set_blockers_toughness(game);

    let attackers = game.turn.combat.attackers.clone();
    for attack_type in &[AttackType::FirstStrike, AttackType::Regular] {
        for attacker in attackers.values() {
            if let Some(attack) = attacker.attacks.get(attack_type) {
                // Distribute combat damage automatically
                let mut damage_left = attack.power.default;

                let attacker_id = attacker.id;
                for blocker_id in attacker.blockers.iter() {
                    if damage_left <= 0 {
                        break;
                    }

                    let toughness = *game
                        .turn
                        .combat
                        .blockers_toughness
                        .get(blocker_id)
                        .unwrap_or(&0);

                    let damage = cmp::min(toughness, damage_left);
                    game.turn
                        .combat
                        .blockers_toughness
                        .insert(*blocker_id, toughness - damage);

                    if let Some(attacker) = game.turn.combat.attackers.get_mut(&attacker_id) {
                        let attack = attacker.attacks.get_mut(attack_type).unwrap();
                        attack.assignments.insert(*blocker_id, damage);
                        damage_left = damage_left.saturating_sub(damage);
                        attack.power.current = damage_left;
                    }
                }
            }
        }
    }
}

fn set_blockers_toughness(game: &mut Game) {
    let attackers = game.turn.combat.attackers.clone();
    for attacker in attackers.values() {
        for blocker_id in attacker.blockers.iter() {
            let toughness = if let Some(blocker) = game.get_card(*blocker_id) {
                blocker.state.toughness.current
            } else {
                0
            };
            game.turn
                .combat
                .blockers_toughness
                .insert(*blocker_id, toughness);
        }
    }
}

pub fn reset_combat_assignments(game: &mut Game, attacker_id: ObjectId) {
    if let Some(attacker) = game.turn.combat.attackers.get_mut(&attacker_id) {
        for attack in attacker.attacks.values_mut() {
            attack.power.reset();

            for (blocker, damage) in attack.assignments.iter() {
                let toughness = game
                    .turn
                    .combat
                    .blockers_toughness
                    .get(blocker)
                    .unwrap_or(&0);
                game.turn
                    .combat
                    .blockers_toughness
                    .insert(*blocker, toughness + damage);
            }
            attack.assignments.clear();
        }
    }
}

pub fn assign_combat_damage(
    game: &mut Game,
    attacker_id: ObjectId,
    blocker_id: ObjectId,
    attack_type: AttackType,
    damage: i16,
) -> bool {
    if damage < 0 {
        return false;
    }

    let toughness = *game
        .turn
        .combat
        .blockers_toughness
        .get(&blocker_id)
        .unwrap_or(&0);
    if damage > toughness {
        return false;
    }

    if let Some(attacker) = game.turn.combat.attackers.get_mut(&attacker_id) {
        if let Some(attack) = attacker.attacks.get_mut(&attack_type) {
            let current = *attack.assignments.get(&blocker_id).unwrap_or(&0);
            if attack.power.current == attack.power.default {
                attack.power.current -= damage;
                attack.assignments.insert(blocker_id, damage);
                game.turn
                    .combat
                    .blockers_toughness
                    .insert(blocker_id, toughness - damage);
                return true;
            } else {
                let diff = current - damage;
                if diff < 0 && attack.power.current >= -diff {
                    attack.power.current -= diff;
                    attack.assignments.insert(blocker_id, damage);
                    game.turn
                        .combat
                        .blockers_toughness
                        .insert(blocker_id, toughness - diff);
                    return true;
                } else if diff > 0 {
                    attack.power.current += diff;
                    attack.assignments.insert(blocker_id, damage);
                    game.turn
                        .combat
                        .blockers_toughness
                        .insert(blocker_id, toughness + diff);
                    return true;
                }
            }
        } else {
            return false;
        }
    }
    false
}

pub fn is_combat_damage_assigned(game: &mut Game) -> bool {
    let attackers = game.turn.combat.attackers.clone();

    for attack in &[AttackType::FirstStrike, AttackType::Regular] {
        for attacker in attackers.values() {
            if let Some(attack) = attacker.attacks.get(attack) {
                let mut total_assigned = 0;
                let mut max_assigned = 0;
                for (blocker_id, damage) in attack.assignments.iter() {
                    let toughness = game
                        .turn
                        .combat
                        .blockers_toughness
                        .get(blocker_id)
                        .unwrap_or(&0);
                    max_assigned += toughness;
                    game.turn
                        .combat
                        .blockers_toughness
                        .insert(*blocker_id, toughness - damage);
                    total_assigned += damage;
                }

                max_assigned = cmp::min(max_assigned, attack.power.default);
                if total_assigned < max_assigned {
                    return false;
                }
            }
        }
    }
    true
}

pub fn combat_damage_step_end(game: &mut Game) {
    if !is_combat_damage_assigned(game) {
        return;
    }

    let (mut attackers_first, mut attackers_last) =
        split_first_strike(game, game.turn.combat.get_attackers());
    let (mut blockers_first, mut blockers_last) =
        split_first_strike(game, game.turn.combat.get_blockers());

    deal_combat_damage(
        game,
        &mut attackers_first,
        &mut blockers_first,
        AttackType::FirstStrike,
    );

    // Filter creatures that were killed during first combat damage step;
    // these creature can't deal damage in the main combat damage step.
    attackers_last = attackers_last
        .into_iter()
        .filter(|attacker_id| is_alive(game, *attacker_id))
        .collect();
    blockers_last = blockers_last
        .into_iter()
        .filter(|blocker_id| is_alive(game, *blocker_id))
        .collect();
    deal_combat_damage(
        game,
        &mut attackers_last,
        &mut blockers_last,
        AttackType::Regular,
    );

    let mut dead = vec![];
    let attackers = game.turn.combat.attackers.clone();
    for attacker in attackers.values() {
        if !is_alive(game, attacker.id) {
            dead.push(attacker.id);
        }
        for blocker in attacker.blockers.iter() {
            if !is_alive(game, *blocker) {
                dead.push(*blocker);
            }
        }
    }

    for card_id in dead {
        put_on_graveyard(game, card_id);
    }
}

fn split_first_strike(
    game: &mut Game,
    creatures: Vec<ObjectId>,
) -> (IndexSet<ObjectId>, IndexSet<ObjectId>) {
    let mut hit_first = IndexSet::new();
    let mut hit_last = IndexSet::new();

    for creature_id in creatures.iter() {
        if let Some(card) = game.get_card(*creature_id) {
            if card.static_abilities.contains(&StaticAbility::DoubleStrike) {
                hit_first.insert(*creature_id);
                hit_last.insert(*creature_id);
            } else if card.static_abilities.contains(&StaticAbility::FirstStrike) {
                hit_first.insert(*creature_id);
            } else {
                hit_last.insert(*creature_id);
            }
        }
    }

    (hit_first, hit_last)
}

fn deal_combat_damage(
    game: &mut Game,
    can_attack: &mut IndexSet<ObjectId>,
    can_counterattack: &mut IndexSet<ObjectId>,
    attack_type: AttackType,
) {
    let attackers = game.turn.combat.clone().get_attackers();
    for attacker_id in attackers.iter() {
        let mut block = IndexSet::new();

        let trample = if let Some(card) = game.get_card(*attacker_id) {
            card.static_abilities.contains(&StaticAbility::Trample)
        } else {
            false
        };

        if let Some(attacker) = game.turn.combat.attackers.get_mut(attacker_id) {
            block = attacker.blockers.clone();

            // Creatures with trample can deal remaining damage to the defending player
            attacker.blocked = !trample && block.len() > 0;
        }

        for blocker in block.iter() {
            let damage_dealt = if let Some(attacker) = game.turn.combat.attackers.get(attacker_id) {
                if let Some(attack) = attacker.attacks.get(&attack_type) {
                    *attack.assignments.get(blocker).unwrap_or(&0)
                } else {
                    0
                }
            } else {
                0
            };

            let mut damage_taken = 0;

            // Blocker takes damage
            if let Some(card) = game.get_card(*blocker) {
                damage_taken = card.state.power.current;
                if can_attack.contains(attacker_id) {
                    card.state.toughness.current -= damage_dealt;
                }
            }

            // Attacker takes damage
            if let Some(card) = game.get_card(*attacker_id) {
                if can_counterattack.contains(blocker) {
                    card.state.toughness.current -= damage_taken;
                }
            }
        }

        if let Some(attacker) = game.turn.combat.attackers.get(attacker_id) {
            if let Some(attack) = attacker.attacks.get(&attack_type) {
                if !attacker.blocked && can_attack.contains(attacker_id) {
                    // Attacker is not blocked, the defending player takes the remaining damage.
                    deal_player_damage(game, attacker.target, attack.power.current as u16);
                }
            }
        }
    }
}

pub fn combat_end_step(game: &mut Game) {
    change_step(game, Step::CombatEnd);
}

pub fn postcombat_step(game: &mut Game) {
    change_step(game, Step::Postcombat);
}

pub fn end_step(game: &mut Game) {
    change_step(game, Step::End);
}

pub fn cleanup_step(game: &mut Game) -> Option<Action> {
    game.turn.step = Step::Cleanup;

    for card in game.cards.values_mut() {
        if card.kind == CardType::Creature {
            card.state.restore();
        }
    }

    if let Some(player) = game.get_player(game.turn.active_player) {
        let hand_size = player.hand.len();
        if hand_size > player.hand_size_limit.current {
            let mut action = Action::new(player.id, 0);
            action.set_required_effect(Effect::Discard(hand_size - player.hand_size_limit.current));

            return Some(action);
        }
    }
    None
}

pub fn pass_priority(game: &mut Game) {
    let next_player = game.get_next_player(game.turn.active_player);
    if let Some(priority) = &mut game.turn.priority {
        priority.pass(next_player);
    }
}

pub fn pass_turn(game: &mut Game) {
    let next_player = game.get_next_player(game.turn.active_player);
    game.turn = Turn::new(next_player);
}

fn change_step(game: &mut Game, phase: Step) {
    game.turn.step = phase.clone();

    // Purge mana pools of all players between phases
    for player in game.players.iter_mut() {
        player.mana.clear();
    }

    dispatch_event(
        game,
        Event::Phase(PhaseEvent {
            owner: game.turn.active_player,
            phase,
        }),
    );
    game.turn.priority = Some(Priority::new(game.turn.active_player));
}

#[cfg(test)]
mod tests {
    use crate::{
        abilities::StaticAbility,
        card::{put_on_battlefield, Card, Zone},
        game::Game,
        turn::{
            all_passed, assign_combat_damage, combat_damage_step_start, declare_blocker,
            is_combat_damage_assigned, AttackType,
        },
    };

    use super::{
        combat_damage_step_end, declare_attacker, declare_attackers_step_end,
        declare_attackers_step_start, declare_blockers_step_end, declare_blockers_step_start,
        Priority,
    };

    #[test]
    fn test_priority_pass() {
        let (_, player_id, opponent_id) = Game::new();

        let mut priority = Priority::new(player_id);
        priority.pass(opponent_id);

        assert!(priority.passed(player_id));
        assert_eq!(priority.player_id, opponent_id);
    }

    #[test]
    fn test_priority_all_passed() {
        let (game, player_id, opponent_id) = Game::new();

        let mut priority = Priority::new(player_id);
        priority.pass(opponent_id);
        priority.pass(player_id);

        assert!(all_passed(&game, priority));
    }

    #[test]
    fn test_combat_no_blockers() {
        let (mut game, player_id, opponent_id) = Game::new();

        let mut card = Card::new_creature(player_id, 3, 1);
        card.static_abilities.insert(StaticAbility::Haste);

        let attacker_id = game.add_card(card);

        put_on_battlefield(&mut game, attacker_id);
        declare_attackers_step_start(&mut game);
        declare_attacker(&mut game, attacker_id, opponent_id);
        declare_attackers_step_end(&mut game);
        declare_blockers_step_start(&mut game);
        declare_blockers_step_end(&mut game);
        combat_damage_step_end(&mut game);

        let card = game.get_card(attacker_id).unwrap();
        assert!(card.state.tapped.current);

        let opponent = game.get_player(opponent_id).unwrap();
        assert_eq!(opponent.life, 17);
    }

    #[test]
    fn test_combat_with_blocker() {
        let (mut game, player_id, opponent_id) = Game::new();

        let mut card = Card::new_creature(player_id, 3, 1);
        card.static_abilities.insert(StaticAbility::Haste);

        let attacker_id = game.add_card(card);
        put_on_battlefield(&mut game, attacker_id);

        let blocker_id = game.add_card(Card::new_creature(opponent_id, 2, 2));
        put_on_battlefield(&mut game, blocker_id);

        declare_attackers_step_start(&mut game);
        declare_attacker(&mut game, attacker_id, opponent_id);
        declare_attackers_step_end(&mut game);
        declare_blockers_step_start(&mut game);
        declare_blocker(&mut game, blocker_id, attacker_id);
        declare_blockers_step_end(&mut game);
        combat_damage_step_start(&mut game);
        combat_damage_step_end(&mut game);

        let card = game.get_card(attacker_id).unwrap();
        assert!(card.zone == Zone::Graveyard);

        let card = game.get_card(blocker_id).unwrap();
        assert!(card.zone == Zone::Graveyard);

        let opponent = game.get_player(opponent_id).unwrap();
        assert_eq!(opponent.life, 20);
    }

    #[test]
    fn test_combat_with_multiple_blockers() {
        let (mut game, player_id, opponent_id) = Game::new();

        let mut card = Card::new_creature(player_id, 5, 3);
        card.static_abilities.insert(StaticAbility::Haste);

        let attacker_id = game.add_card(card);
        put_on_battlefield(&mut game, attacker_id);

        let blocker_one = game.add_card(Card::new_creature(opponent_id, 1, 2));
        put_on_battlefield(&mut game, blocker_one);

        let blocker_two = game.add_card(Card::new_creature(opponent_id, 2, 2));
        put_on_battlefield(&mut game, blocker_two);

        declare_attackers_step_start(&mut game);
        declare_attacker(&mut game, attacker_id, opponent_id);
        declare_attackers_step_end(&mut game);
        declare_blockers_step_start(&mut game);
        declare_blocker(&mut game, blocker_one, attacker_id);
        declare_blocker(&mut game, blocker_two, attacker_id);
        declare_blockers_step_end(&mut game);
        combat_damage_step_start(&mut game);
        combat_damage_step_end(&mut game);

        let attacker = game.get_card(attacker_id).unwrap();
        assert!(attacker.zone == Zone::Graveyard);

        let blocker = game.get_card(blocker_one).unwrap();
        assert!(blocker.zone == Zone::Graveyard);

        let blocker = game.get_card(blocker_two).unwrap();
        assert!(blocker.zone == Zone::Graveyard);

        let opponent = game.get_player(opponent_id).unwrap();
        assert_eq!(opponent.life, 20);
    }

    #[test]
    fn test_assign_combat_damage() {
        let (mut game, player_id, opponent_id) = Game::new();

        let mut card = Card::new_creature(player_id, 2, 1);
        card.static_abilities.insert(StaticAbility::Haste);
        let attacker_id = game.add_card(card);
        put_on_battlefield(&mut game, attacker_id);

        let blocker_one = game.add_card(Card::new_creature(opponent_id, 1, 3));
        put_on_battlefield(&mut game, blocker_one);

        let blocker_two = game.add_card(Card::new_creature(opponent_id, 1, 1));
        put_on_battlefield(&mut game, blocker_two);

        declare_attackers_step_start(&mut game);
        declare_attacker(&mut game, attacker_id, opponent_id);
        declare_attackers_step_end(&mut game);
        declare_blockers_step_start(&mut game);
        declare_blocker(&mut game, blocker_one, attacker_id);
        declare_blocker(&mut game, blocker_two, attacker_id);
        declare_blockers_step_end(&mut game);
        combat_damage_step_start(&mut game);
        assign_combat_damage(&mut game, attacker_id, blocker_one, AttackType::Regular, 1);
        assign_combat_damage(&mut game, attacker_id, blocker_two, AttackType::Regular, 1);
        combat_damage_step_end(&mut game);

        let attacker = game.get_card(attacker_id).unwrap();
        assert!(attacker.zone == Zone::Graveyard);

        let blocker = game.get_card(blocker_one).unwrap();
        assert_eq!(blocker.zone, Zone::Battlefield);
        assert_eq!(blocker.state.toughness.current, 2);

        let blocker = game.get_card(blocker_two).unwrap();
        assert!(blocker.zone == Zone::Graveyard);
    }

    #[test]
    fn test_assign_illegal_combat_damage() {
        let (mut game, player_id, opponent_id) = Game::new();

        let mut card = Card::new_creature(player_id, 2, 2);
        card.static_abilities.insert(StaticAbility::Haste);
        let attacker_id = game.add_card(card);
        put_on_battlefield(&mut game, attacker_id);

        let blocker_id = game.add_card(Card::new_creature(opponent_id, 1, 3));
        put_on_battlefield(&mut game, blocker_id);

        declare_attackers_step_start(&mut game);
        declare_attacker(&mut game, attacker_id, opponent_id);
        declare_attackers_step_end(&mut game);
        declare_blockers_step_start(&mut game);
        declare_blocker(&mut game, blocker_id, attacker_id);
        declare_blockers_step_end(&mut game);

        combat_damage_step_start(&mut game);

        // Assign more damage than the creature can deal
        assert!(!assign_combat_damage(
            &mut game,
            attacker_id,
            blocker_id,
            AttackType::Regular,
            3
        ));

        // Assign negative damage
        assert!(!assign_combat_damage(
            &mut game,
            attacker_id,
            blocker_id,
            AttackType::Regular,
            -2
        ));

        // Assign less damage than the creature can deal
        assert!(assign_combat_damage(
            &mut game,
            attacker_id,
            blocker_id,
            AttackType::Regular,
            0
        ));
        assert!(!is_combat_damage_assigned(&mut game));

        // Assign first strike damage when creature does not have first strike
        assert!(!assign_combat_damage(
            &mut game,
            attacker_id,
            blocker_id,
            AttackType::FirstStrike,
            1
        ));
    }
}
