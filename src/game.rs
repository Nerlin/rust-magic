use std::collections::HashMap;

use crate::{
    abilities::Effect,
    action::{Action, Choice},
    card::{Card, Zone},
    events::{CardEvent, Event},
    mana::{Color, Mana},
};

pub struct Game {
    players: Vec<Player>,
    cards: HashMap<usize, Card>,
    stack: Vec<StackEntry>,
    uid: ObjectId,
}

pub type ObjectId = usize;

impl Game {
    pub fn new() -> Game {
        Game {
            uid: 0,
            stack: vec![],
            players: vec![],
            cards: HashMap::new(),
        }
    }

    pub fn get_uid(&mut self) -> ObjectId {
        self.uid += 1;
        self.uid
    }

    pub fn add_player(&mut self, mut player: Player) -> ObjectId {
        let player_id = self.get_uid();
        player.id = player_id;

        self.players.push(player);
        player_id
    }

    pub fn get_player(&mut self, player_id: ObjectId) -> Option<&mut Player> {
        for player in self.players.iter_mut() {
            if player.id == player_id {
                return Some(player);
            }
        }
        None
    }

    pub fn add_card(&mut self, mut card: Card) -> ObjectId {
        let card_id = self.get_uid();
        card.id = card_id;

        self.cards.insert(card.id, card);
        card_id
    }

    pub fn get_card(&mut self, card_id: ObjectId) -> Option<&mut Card> {
        self.cards.get_mut(&card_id)
    }
}

pub struct StackEntry {
    effect: Effect,
    action: Action,
}

pub struct Player {
    pub id: ObjectId,
    pub life: u16,
    pub mana: Mana,
}

impl Player {
    pub fn new() -> Player {
        Player {
            id: 0,
            life: 20,
            mana: Mana::new(),
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

    if !action.valid() {
        return;
    }

    if !pay_cost(game, &action) {
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

fn pay_cost(game: &mut Game, action: &Action) -> bool {
    return match &action.choices.cost {
        Choice::Mana(mana) => {
            return if let Some(player) = game.get_player(action.player_id) {
                player.mana -= *mana;
                true
            } else {
                false
            }
        }
        Choice::Tap(target) => {
            if let Some(card) = game.get_card(*target) {
                let tapped = card.tap();
                if tapped {
                    dispatch_event(
                        game,
                        Event::Tap(CardEvent {
                            owner: action.player_id,
                            source: action.card_id,
                            card: *target,
                        }),
                    );
                }
                return tapped;
            }
            false
        }
        _ => false,
    };
}

pub fn dispatch_event(game: &mut Game, event: Event) {
    // TODO: First iterate through the active player cards
    for card in game.cards.values() {
        if card.zone == Zone::Battlefield {
            for trigger in card.abilities.triggers.iter() {
                if event.meets(&trigger.condition) {
                    let mut action = Action::new(card.owner_id, card.id);
                    action.set_required_target(trigger.target.clone());
                    action.set_required_effect(trigger.effect.clone());

                    game.stack.push(StackEntry {
                        effect: trigger.effect.clone(),
                        action,
                    });
                }
            }
        }
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
                    if owner.id == player_id {
                        owner.life -= damage;
                    } else if let Some(player) = game.get_player(player_id) {
                        player.life -= damage;
                    }
                }
                _ => {}
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        abilities::{ActivatedAbility, Condition, Cost, Effect, Target, TriggeredAbility},
        action::Choice,
        card::{Card, CardType, Zone},
        game::{create_ability_action, resolve_stack, Game},
        mana::Mana,
    };

    use super::{play_ability, Player};

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
        action.choices.cost = Choice::Tap(card_id);

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

        let mut card = Card::default();
        card.name = String::from("City of Brass");
        card.kind = CardType::Land;
        card.zone = Zone::Battlefield;
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

        let mut action = create_ability_action(&mut game, player_id, card_id, 0).unwrap();
        action.choices.cost = Choice::Tap(card_id);
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
        card.zone = Zone::Battlefield;
        card.owner_id = player_id;
        card.abilities.activated.push(ActivatedAbility {
            cost: Cost::Mana(Mana::from("R")),
            effect: Effect::Damage(1),
            target: Target::Player,
        });
        let card_id = game.add_card(card);

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
}
