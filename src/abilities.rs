use crate::{
    action::{Action, Choice},
    card::put_on_graveyard,
    game::{Game, GameStatus, ObjectId, StackEntry},
    mana::{Color, Mana},
    turn::Phase,
};

#[derive(Default)]
pub struct Abilities {
    pub activated: Vec<ActivatedAbility>,
    pub triggers: Vec<TriggeredAbility>,
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

#[derive(Clone, Debug)]
pub enum Cost {
    None,
    Mana(Mana),
    Tap(Target),
}

#[derive(Clone, Debug)]
pub enum Effect {
    None,
    Mana(Mana),
    Damage(u16),
    Discard(usize),
}

#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub enum Condition {
    Tap(Target),
    Untap(Target),
    Draw,
    Phase(Phase),
}

#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub enum Target {
    None,
    Source,
    Owner,
    Player,
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

    match card.abilities.activated.get_mut(ability_id) {
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

    let ability = if let Some(ability) = card.abilities.activated.get_mut(ability_id) {
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
    let entry = StackEntry { effect, action };
    if let Effect::Mana(_) = entry.effect {
        // Mana abilities are resolved without stack.
        resolve_stack_effect(game, entry);
    } else {
        game.stack.push(entry);
    }
}

pub fn resolve_stack(game: &mut Game) {
    while let Some(entry) = game.stack.pop() {
        resolve_stack_effect(game, entry);
    }
}

fn resolve_stack_effect(game: &mut Game, entry: StackEntry) {
    if let Some(owner) = game.get_player(entry.action.player_id) {
        match entry.effect {
            Effect::None => {}
            Effect::Mana(mana) => {
                if mana.has(&Color::Any) {
                    if let Choice::Mana(mana) = entry.action.choices.effect {
                        owner.mana += mana;
                    } else {
                        panic!("The ability required choosing mana.");
                    }
                } else {
                    owner.mana += mana;
                }
            }
            Effect::Damage(damage) => match entry.action.choices.target {
                Choice::Player(player_id) => {
                    take_damage(game, player_id, damage);
                }
                _ => {}
            },
            Effect::Discard(_) => match entry.action.choices.effect {
                Choice::CardsExact(cards) => {
                    for card_id in cards.iter() {
                        put_on_graveyard(game, *card_id);
                    }
                }
                _ => {}
            },
        }
    }
}

pub(crate) fn take_damage(game: &mut Game, player_id: ObjectId, damage: u16) {
    if damage == 0 {
        return;
    }

    if let Some(player) = game.get_player(player_id) {
        player.life = player.life.saturating_sub(damage as i16);
        if player.life <= 0 {
            game.status = GameStatus::Lose(player_id);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        abilities::{
            create_ability_action, play_ability, resolve_stack, take_damage, ActivatedAbility,
            Condition, Cost, Effect, Target, TriggeredAbility,
        },
        action::Choice,
        card::{put_on_battlefield, Card, CardType, Zone},
        game::{Game, GameStatus, Player},
        mana::Mana,
        turn::Turn,
    };

    #[test]
    fn test_basic_land() {
        let mut game = Game::new();
        let player_id = game.add_player(Player::new());

        let mut card = Card::default();
        card.name = String::from("Forest");
        card.kind = CardType::Land;
        card.owner_id = player_id;
        card.zone = Zone::Battlefield;
        card.abilities.activated.push(ActivatedAbility {
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
        assert!(card.tapped);
    }

    #[test]
    fn test_city_of_brass() {
        let mut game = Game::new();
        let player_id = game.add_player(Player::new());
        game.turn = Turn::new(player_id);

        let mut card = Card::default();
        card.name = String::from("City of Brass");
        card.kind = CardType::Land;
        card.owner_id = player_id;
        card.abilities.activated.push({
            ActivatedAbility {
                cost: Cost::Tap(Target::Source),
                effect: Effect::Mana(Mana::from("*")),
                target: Target::None,
            }
        });
        card.abilities.triggers.push({
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

        let mut card = Card::default();
        card.kind = CardType::Artifact;
        card.owner_id = player_id;
        card.abilities.activated.push(ActivatedAbility {
            cost: Cost::Mana(Mana::from("R")),
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
    fn test_lethal_damage() {
        let mut game = Game::new();
        let mut player = Player::new();
        player.life = 3;

        let player_id = game.add_player(player);
        take_damage(&mut game, player_id, 3);

        let player = game.get_player(player_id).unwrap();
        assert_eq!(player.life, 0);
        assert_eq!(game.status, GameStatus::Lose(player_id));
    }
}
