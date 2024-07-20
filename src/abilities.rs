use crate::{
    action::{Action, Choice},
    card::{put_on_battlefield, put_on_graveyard, put_on_stack, CardType},
    game::{Game, GameStatus, ObjectId, Stacked, Value},
    mana::{Color, Mana},
    turn::Step,
};

#[derive(Clone, Debug)]
pub struct Resolve {
    pub effect: Effect,
    pub target: Target,
}

#[derive(Clone, Debug)]
pub struct ActivatedAbility {
    pub cost: Cost,
    pub effect: Effect,
    pub target: Target,
}

#[derive(Clone, Debug)]
pub struct TriggeredAbility {
    pub condition: Condition,
    pub effect: Effect,
    pub target: Target,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum StaticAbility {
    #[default]
    None,
    Haste,
    Flying,
    Reach,
    Vigilance,
    Defender,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub enum Cost {
    #[default]
    None,
    Mana(&'static str),
    Tap(Target),
    Sacrifice(Target),

    // Must pay all
    And(&'static [Cost]),
}

#[derive(Clone, Debug)]
pub enum Effect {
    None,
    Mana(Mana),
    Damage(u16),
    Discard(usize),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Condition {
    Tap(Target),
    Untap(Target),
    Draw,
    Phase(Step),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Target {
    None,
    Source,
    Owner,
    Player,
    Creature,

    // Defines that any of the specified targets can be selected
    AnyOf(&'static [Target]),
}

pub fn create_card_action(
    game: &mut Game,
    player_id: ObjectId,
    card_id: ObjectId,
) -> Option<Action> {
    let card = if let Some(card) = game.get_card(card_id) {
        card
    } else {
        return None;
    };

    return if let Some(resolve) = &card.effect {
        let cost = card.cost.clone();
        let effect = resolve.effect.clone();
        let target = resolve.target.clone();

        let mut action = Action::new(player_id, card_id);
        action.set_required_cost(cost);
        action.set_required_target(target);
        action.set_required_effect(effect);
        Some(action)
    } else {
        None
    };
}

pub fn can_play_card(game: &mut Game, card_id: ObjectId, player_id: ObjectId) -> bool {
    if let Some(priority) = &game.turn.priority {
        if priority.player_id != player_id {
            // Players can play cards only when they have priority
            return false;
        }
    }

    let is_stack_empty = game.stack.len() == 0;
    let is_main_phase = game.turn.step.main();
    let is_active_player = game.turn.active_player == player_id;
    let lands_limit = if let Some(player) = game.get_player(player_id) {
        player.land_limit.current
    } else {
        0
    };

    if let Some(card) = game.get_card(card_id) {
        return if card.owner_id != player_id {
            // Players can only play their own cards
            return false;
        } else if card.kind == CardType::Instant {
            // Instant spells can be played without time restrictions
            true
        } else {
            // Other cards can be played on the sorcery speed:
            // - stack must be empty;
            // - must be in the main phase;
            // - must be an active player;
            let mut can_play = is_stack_empty && is_main_phase && is_active_player;
            if card.kind == CardType::Land {
                can_play &= game.turn.lands_played < lands_limit;
            }
            can_play
        };
    }
    false
}

pub fn play_card(game: &mut Game, card_id: ObjectId, action: Action) {
    if !can_play_card(game, card_id, action.player_id) {
        return;
    }

    if !action.valid(game) {
        return;
    }

    if !action.pay(game) {
        return;
    }

    if let Some(card) = game.get_card(card_id) {
        if card.kind == CardType::Land {
            // Lands don't use stack, must be played directly on the battlefield,
            // but not more than the player land limit per turn
            game.turn.lands_played += 1;
            put_on_battlefield(game, card_id);
        } else {
            let spell = Stacked::Spell { card_id, action };
            game.stack.push(spell);

            put_on_stack(game, card_id)
        }
    }
}

pub fn create_ability_action(
    game: &mut Game,
    player_id: ObjectId,
    card_id: ObjectId,
    ability_id: usize,
) -> Option<Action> {
    let card = if let Some(card) = game.get_card(card_id) {
        card
    } else {
        return None;
    };

    match card.activated_abilities.get_mut(ability_id) {
        Some(ability) => {
            let cost = ability.cost.clone();
            let target = ability.target.clone();

            let mut action = Action::new(player_id, card_id);
            action.set_required_cost(cost);
            action.set_required_target(target);
            Some(action)
        }
        None => None,
    }
}

pub fn play_ability(game: &mut Game, card_id: ObjectId, ability_id: usize, action: Action) {
    let card = if let Some(card) = game.get_card(card_id) {
        card
    } else {
        return;
    };

    let ability = if let Some(ability) = card.activated_abilities.get_mut(ability_id) {
        ability.clone()
    } else {
        return;
    };

    if !action.valid(game) {
        return;
    }

    if !action.pay(game) {
        return;
    }

    let effect = ability.effect.clone();
    let entry = Stacked::Ability { effect, action };
    if let Effect::Mana(_) = ability.effect {
        // Mana abilities are resolved without stack.
        resolve_stacked(game, entry);
    } else {
        game.stack.push(entry);
    }
}

pub fn resolve_stack(game: &mut Game) {
    while let Some(entry) = game.stack.pop() {
        resolve_stacked(game, entry);
    }
}

pub fn resolve_stacked(game: &mut Game, stacked: Stacked) {
    match stacked {
        Stacked::Spell { card_id, action } => {
            let played_effect = if let Some(card) = game.get_card(card_id) {
                if let Some(resolve) = &card.effect {
                    Some(resolve.effect.clone())
                } else {
                    None
                }
            } else {
                None
            };

            if let Some(effect) = played_effect {
                resolve_effect(game, effect, action);
            }

            if let Some(card) = game.get_card(card_id) {
                match &card.kind {
                    CardType::Artifact | CardType::Enchantment | CardType::Creature => {
                        put_on_battlefield(game, card_id)
                    }
                    CardType::Instant | CardType::Sorcery => put_on_graveyard(game, card_id),
                    CardType::Land => panic!("Lands must not use stack."),
                }
            }
        }
        Stacked::Ability { effect, action } => resolve_effect(game, effect, action),
    }
}

fn resolve_effect(game: &mut Game, effect: Effect, action: Action) {
    if let Some(owner) = game.get_player(action.player_id) {
        match effect {
            Effect::None => {}
            Effect::Mana(mana) => {
                if mana.has(&Color::Any) {
                    if let Choice::Mana(mana) = action.choices.effect {
                        owner.mana += mana;
                    } else {
                        panic!("The ability required choosing mana.");
                    }
                } else {
                    owner.mana += mana;
                }
            }
            Effect::Damage(damage) => match action.choices.target {
                Choice::Player(player_id) => {
                    deal_player_damage(game, player_id, damage);
                }
                Choice::Card(card_id) => {
                    deal_damage(game, card_id, damage);
                }
                _ => {}
            },
            Effect::Discard(_) => match action.choices.effect {
                Choice::And(choices) => {
                    for choice in choices.iter() {
                        if let Choice::Card(card_id) = choice {
                            put_on_graveyard(game, *card_id);
                        }
                    }
                }
                _ => {}
            },
        }
    }
}

pub(crate) fn deal_player_damage(game: &mut Game, player_id: ObjectId, damage: u16) {
    if damage == 0 {
        return;
    }

    if let Some(player) = game.get_player(player_id) {
        player.life -= damage as i16;
        if player.life <= 0 {
            game.status = GameStatus::Lose(player_id);
        }
    }
}

pub(crate) fn deal_damage(game: &mut Game, card_id: ObjectId, damage: u16) {
    if damage == 0 {
        return;
    }

    if let Some(card) = game.get_card(card_id) {
        match &mut card.kind {
            CardType::Creature => {
                card.state.toughness.current -= damage as i16;
                if card.state.toughness.current <= 0 {
                    put_on_graveyard(game, card_id);
                }
            }
            _ => {}
        }
    }
}

pub fn apply_static_abilities(game: &mut Game, card_id: ObjectId) {
    if let Some(card) = game.get_card(card_id) {
        for ability in card.static_abilities.iter() {
            match ability {
                StaticAbility::Haste => {
                    card.state.motion_sickness = Value::new(false);
                }
                _ => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        abilities::{
            can_play_card, create_ability_action, create_card_action, play_ability, play_card,
            resolve_stack, ActivatedAbility, Condition, Cost, Effect, Resolve, Target,
            TriggeredAbility,
        },
        action::{Action, Choice},
        card::{put_in_hand, put_on_battlefield, Card, CardSubtype, Zone},
        game::{add_mana, Game, Player},
        mana::Mana,
        turn::{pass_priority, pass_turn, postcombat_step, precombat_step, upkeep_step, Turn},
    };

    #[test]
    fn test_mana_ability() {
        let mut game = Game::new();
        let player_id = game.add_player(Player::new());

        let mut card = Card::new_land(player_id);
        card.name = String::from("Forest");
        card.zone = Zone::Battlefield;
        card.subtypes.insert(CardSubtype::Forest);
        card.activated_abilities.push(ActivatedAbility {
            cost: Cost::Tap(Target::Source),
            effect: Effect::Mana(Mana::from("G")),
            target: Target::None,
        });
        let card_id = game.add_card(card);

        let mut action = create_ability_action(&mut game, player_id, card_id, 0).unwrap();
        action.choices.cost = Choice::Card(card_id);

        play_ability(&mut game, card_id, 0, action);

        let player = game.get_player(player_id).unwrap();
        assert_eq!(player.mana.green, 1);

        let card = game.get_card(card_id).unwrap();
        assert!(card.state.tapped.current);
    }

    #[test]
    fn test_mana_ability_with_trigger() {
        let mut game = Game::new();
        let player_id = game.add_player(Player::new());
        game.turn = Turn::new(player_id);

        let mut card = Card::new_land(player_id);
        card.name = String::from("City of Brass");
        card.activated_abilities.push({
            ActivatedAbility {
                cost: Cost::Tap(Target::Source),
                effect: Effect::Mana(Mana::from("*")),
                target: Target::None,
            }
        });
        card.triggered_abilities.push({
            TriggeredAbility {
                condition: Condition::Tap(Target::Source),
                effect: Effect::Damage(1),
                target: Target::Owner,
            }
        });
        let card_id = game.add_card(card);

        put_on_battlefield(&mut game, card_id);

        let mut action = create_ability_action(&mut game, player_id, card_id, 0).unwrap();
        action.choices.cost = Choice::Card(card_id);
        action.choices.effect = Choice::Mana(Mana::from("B"));

        play_ability(&mut game, card_id, 0, action);
        resolve_stack(&mut game);

        let player = game.get_player(player_id).unwrap();
        assert_eq!(player.mana.black, 1);
        assert_eq!(player.life, 19);
    }

    #[test]
    fn test_activate_damage_ability_for_mana() {
        let mut game = Game::new();
        let mut player = Player::new();
        player.mana.red = 1;

        let player_id = game.add_player(player);
        let opponent_id = game.add_player(Player::new());

        let mut card = Card::new_artifact(player_id);
        card.activated_abilities.push(ActivatedAbility {
            cost: Cost::Mana("R"),
            effect: Effect::Damage(1),
            target: Target::Player,
        });
        let card_id = game.add_card(card);

        put_on_battlefield(&mut game, card_id);

        let mut action = create_ability_action(&mut game, player_id, card_id, 0).unwrap();
        action.choices.cost = Choice::Mana(Mana::from("R"));
        action.choices.target = Choice::Player(opponent_id);

        play_ability(&mut game, card_id, 0, action);
        resolve_stack(&mut game);

        let opponent = game.get_player(opponent_id).unwrap();
        assert_eq!(opponent.life, 19);

        let player = game.get_player(player_id).unwrap();
        assert_eq!(player.mana.red, 0);
    }

    #[test]
    fn test_activate_ability_any_of_target_creature() {
        let mut game = Game::new();
        let mut player = Player::new();
        player.mana.red = 1;

        let player_id = game.add_player(player);
        game.turn = Turn::new(player_id);

        let mut card = Card::new_sorcery(player_id);
        card.cost = Cost::Mana("R");
        card.effect = Some(Resolve {
            effect: Effect::Damage(2),
            target: Target::AnyOf(&[Target::Player, Target::Creature]),
        });
        let sorcery_id = game.add_card(card);
        put_in_hand(&mut game, sorcery_id);

        let creature_id = game.add_card(Card::new_creature(player_id, 2, 2));
        put_on_battlefield(&mut game, creature_id);
        precombat_step(&mut game);

        let mut action = create_card_action(&mut game, player_id, sorcery_id).unwrap();
        action.choices.cost = Choice::Mana(Mana::from("R"));
        action.choices.target = Choice::Card(creature_id);

        play_card(&mut game, sorcery_id, action);
        resolve_stack(&mut game);

        let card = game.get_card(sorcery_id).unwrap();
        assert_eq!(card.zone, Zone::Graveyard);

        let creature = game.get_card(creature_id).unwrap();
        assert_eq!(creature.zone, Zone::Graveyard);
    }

    #[test]
    fn test_play_land() {
        let mut game = Game::new();
        let player_id = game.add_player(Player::new());
        let card_id = game.add_card(Card::new_land(player_id));

        pass_turn(&mut game);
        precombat_step(&mut game);
        put_in_hand(&mut game, card_id);
        play_card(&mut game, card_id, Action::new(player_id, card_id));

        let card = game.get_card(card_id).unwrap();
        assert_eq!(card.zone, Zone::Battlefield);
    }

    #[test]
    fn test_can_play_one_land_per_turn() {
        let mut game = Game::new();
        let player_id = game.add_player(Player::new());
        let card_id = game.add_card(Card::new_land(player_id));

        pass_turn(&mut game);
        postcombat_step(&mut game);
        put_in_hand(&mut game, card_id);
        play_card(&mut game, card_id, Action::new(player_id, card_id));

        assert!(!can_play_card(&mut game, card_id, player_id));
    }

    #[test]
    fn test_can_play_instant_if_has_priority() {
        let mut game = Game::new();
        let player_id = game.add_player(Player::new());
        let opponent_id = game.add_player(Player::new());

        let mut card = Card::new_instant(opponent_id);
        card.cost = Cost::Mana("R");
        card.effect = Some(Resolve {
            effect: Effect::Damage(3),
            target: Target::AnyOf(&[Target::Player, Target::Creature]),
        });
        let card_id = game.add_card(card);

        pass_turn(&mut game);
        upkeep_step(&mut game);
        pass_priority(&mut game);
        add_mana(&mut game, opponent_id, Mana::from("RRR"));

        let mut action = create_card_action(&mut game, opponent_id, card_id).unwrap();
        action.choices.cost = Choice::Mana(Mana::from("R"));
        action.choices.target = Choice::Player(player_id);

        play_card(&mut game, card_id, action);
        resolve_stack(&mut game);

        let player = game.get_player(player_id).unwrap();
        assert_eq!(player.life, 17);
    }

    #[test]
    fn test_scorched_rusalka() {
        let mut game = Game::new();
        let player_id = game.add_player(Player::new());
        let opponent_id = game.add_player(Player::new());

        let mut card = Card::new_creature(player_id, 1, 1);
        card.cost = Cost::Mana("R");
        card.subtypes.insert(CardSubtype::Spirit);
        card.activated_abilities.push(ActivatedAbility {
            cost: Cost::And(&[Cost::Mana("R"), Cost::Sacrifice(Target::Creature)]),
            effect: Effect::Damage(1),
            target: Target::Player,
        });
        let card_id = game.add_card(card);

        put_on_battlefield(&mut game, card_id);
        add_mana(&mut game, player_id, Mana::from("R"));

        let mut action = create_ability_action(&mut game, player_id, card_id, 0).unwrap();
        action.choices.cost =
            Choice::And(vec![Choice::Card(card_id), Choice::Mana(Mana::from("R"))]);
        action.choices.target = Choice::Player(opponent_id);

        play_ability(&mut game, card_id, 0, action);
        resolve_stack(&mut game);

        let opponent = game.get_player(opponent_id).unwrap();
        assert_eq!(opponent.life, 19);

        let card = game.get_card(card_id).unwrap();
        assert_eq!(card.zone, Zone::Graveyard);
    }
}
