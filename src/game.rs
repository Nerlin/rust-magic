use std::collections::HashMap;

use indexmap::IndexSet;
use rand::seq::SliceRandom;
use rand::thread_rng;

use crate::{
    abilities::Effect,
    action::{Action, Choice},
    card::{Card, Zone},
    events::Event,
    mana::{Color, Mana},
    turn::Turn,
};

pub struct Game {
    pub turn: Turn,
    pub status: GameStatus,
    pub(crate) players: Vec<Player>,
    pub(crate) cards: HashMap<usize, Card>,
    stack: Vec<StackEntry>,
    uid: ObjectId,
}

#[derive(Debug, PartialEq, PartialOrd)]
pub enum GameStatus {
    Play,
    Lose(ObjectId),
}

pub type ObjectId = usize;

impl Game {
    pub fn new() -> Game {
        Game {
            status: GameStatus::Play,
            uid: 0,
            stack: vec![],
            players: vec![],
            cards: HashMap::new(),
            turn: Turn::new(0),
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

    pub fn get_player_ids(&self) -> Vec<ObjectId> {
        self.players
            .iter()
            .map(|player| player.id)
            .collect::<Vec<ObjectId>>()
            .clone()
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
    pub life: i16,
    pub mana: Mana,

    pub library: IndexSet<ObjectId>,
    pub hand: IndexSet<ObjectId>,
    pub battlefield: IndexSet<ObjectId>,
    pub graveyard: IndexSet<ObjectId>,

    pub max_hand_size: usize,
}

pub const DEFAULT_HAND_SIZE: usize = 7;
pub const DEFAULT_PLAYER_LIFE: i16 = 20;

impl Player {
    pub fn new() -> Player {
        Player {
            id: 0,
            life: DEFAULT_PLAYER_LIFE,
            mana: Mana::new(),
            library: IndexSet::new(),
            hand: IndexSet::new(),
            battlefield: IndexSet::new(),
            graveyard: IndexSet::new(),
            max_hand_size: DEFAULT_HAND_SIZE,
        }
    }

    pub fn zones(&self) -> Vec<(Zone, &IndexSet<ObjectId>)> {
        vec![
            (Zone::Library, &self.library),
            (Zone::Hand, &self.hand),
            (Zone::Battlefield, &self.battlefield),
            (Zone::Graveyard, &self.graveyard),
        ]
    }

    pub fn zones_mut(&mut self) -> Vec<(Zone, &mut IndexSet<ObjectId>)> {
        vec![
            (Zone::Library, &mut self.library),
            (Zone::Hand, &mut self.hand),
            (Zone::Battlefield, &mut self.battlefield),
            (Zone::Graveyard, &mut self.graveyard),
        ]
    }
}

pub fn start_game(game: &mut Game) {
    if game.players.len() != 2 {
        panic!("The game must include exactly two players.");
    }

    let active_player = game.players.choose(&mut thread_rng()).unwrap();
    game.turn = Turn::new(active_player.id);
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

pub(crate) fn dispatch_event(game: &mut Game, event: Event) {
    run_player_triggers(game, game.turn.active_player, &event);
    for player_id in game.get_player_ids() {
        if player_id != game.turn.active_player {
            run_player_triggers(game, player_id, &event);
        }
    }
}

fn run_player_triggers(game: &mut Game, player_id: ObjectId, event: &Event) {
    let player = if let Some(player) = game.get_player(player_id) {
        player
    } else {
        return;
    };

    let battlefield = player.battlefield.clone();
    for card_id in battlefield {
        let triggers = if let Some(card) = game.get_card(card_id) {
            card.abilities.triggers.clone()
        } else {
            vec![]
        };

        for trigger in triggers.iter() {
            if event.meets(&trigger.condition) {
                let mut action = Action::new(player_id, card_id);
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
                        discard(game, *card_id);
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
        if player.life == 0 {
            game.status = GameStatus::Lose(player_id);
        }
    }
}

pub(crate) fn discard(game: &mut Game, card_id: ObjectId) {
    if let Some(card) = game.get_card(card_id) {
        card.zone = Zone::Graveyard;
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        abilities::{ActivatedAbility, Condition, Cost, Effect, Target, TriggeredAbility},
        action::Choice,
        card::{put_on_battlefield, Card, CardType, Zone},
        game::{create_ability_action, resolve_stack, Game, GameStatus},
        mana::Mana,
        turn::Turn,
    };

    use super::{play_ability, take_damage, Player};

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
