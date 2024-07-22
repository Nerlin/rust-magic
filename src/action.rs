use crate::{
    abilities::{Cost, Effect, Target},
    card::{put_on_graveyard, tap_card, CardType, Zone},
    game::{Game, ObjectId},
    mana::Mana,
};

#[derive(Debug)]
pub struct Action {
    pub player_id: ObjectId,
    pub card_id: ObjectId,
    pub(crate) required: Required,
    pub choices: Choices,
}

impl Action {
    pub fn new(player_id: ObjectId, card_id: ObjectId) -> Action {
        Action {
            player_id,
            card_id,
            required: Required {
                cost: Cost::None,
                target: Target::None,
                effect: Effect::None,
            },
            choices: Choices {
                cost: Choice::None,
                target: Choice::None,
                effect: Choice::None,
            },
        }
    }

    pub fn set_required_cost(&mut self, cost: Cost) {
        self.required.cost = cost;
    }

    pub fn set_required_target(&mut self, target: Target) {
        match target {
            Target::Owner => {
                self.choices.target = Choice::Player(self.player_id);
            }
            _ => {}
        }
        self.required.target = target;
    }

    pub fn set_required_effect(&mut self, effect: Effect) {
        self.required.effect = effect;
    }

    pub fn pay(&self, game: &mut Game) -> bool {
        self.pay_cost(game, &self.required.cost, &self.choices.cost)
    }

    fn pay_cost(&self, game: &mut Game, cost: &Cost, choice: &Choice) -> bool {
        return match &cost {
            Cost::None => true,
            Cost::Mana(_) => match &choice {
                Choice::Mana(mana) => {
                    return if let Some(player) = game.get_player(self.player_id) {
                        player.mana -= *mana;
                        true
                    } else {
                        false
                    }
                }
                Choice::And(choices) => choices
                    .iter()
                    .any(|choice| self.pay_cost(game, cost, choice)),
                _ => false,
            },
            Cost::Tap(_) => match &choice {
                Choice::Card(card_id) => tap_card(game, *card_id, Some(self.card_id)),
                Choice::And(choices) => choices
                    .iter()
                    .any(|choice| self.pay_cost(game, cost, choice)),
                _ => false,
            },
            Cost::Sacrifice(_) => match &choice {
                Choice::Card(card_id) => {
                    put_on_graveyard(game, *card_id);
                    true
                }
                Choice::And(choices) => choices
                    .iter()
                    .any(|choice| self.pay_cost(game, cost, choice)),
                _ => false,
            },
            Cost::And(costs) => costs.iter().all(|cost| self.pay_cost(game, cost, choice)),
        };
    }

    pub fn valid(&self, game: &mut Game) -> bool {
        self.valid_cost(game, &self.required.cost)
            && self.valid_target(game, &self.required.target)
            && self.valid_effect(game)
    }

    fn valid_cost(&self, game: &mut Game, cost: &Cost) -> bool {
        return match cost {
            Cost::None => true,
            Cost::Mana(mana) => self.choices.cost.validate_mana(&Mana::from(*mana)),
            Cost::Tap(target) => match target {
                Target::Source => {
                    if !self.choices.cost.validate_card(self.card_id) {
                        return false;
                    }

                    if let Some(card) = game.get_card(self.card_id) {
                        // Creatures can be tapped for their ability only if they don't have summoning sickness
                        return card.kind != CardType::Creature
                            || !card.state.summoning_sickness.current;
                    }

                    true
                }
                _ => true,
            },
            Cost::Sacrifice(target) => match target {
                Target::Source => self.choices.cost.validate_card(self.card_id),
                Target::Creature => {
                    if let Some(creature_id) = self.choices.cost.validate_creature(game) {
                        if let Some(card) = game.get_card(creature_id) {
                            return card.zone == Zone::Battlefield
                                && card.owner_id == self.player_id;
                        }
                    }
                    false
                }
                _ => false,
            },
            Cost::And(costs) => costs.iter().all(|cost| self.valid_cost(game, cost)),
        };
    }

    fn valid_target(&self, game: &mut Game, target: &Target) -> bool {
        return match &target {
            Target::None => true,
            Target::Source => self.choices.target.validate_card(self.card_id),
            Target::Player => self.choices.target.validate_player(None),
            Target::Creature => self.choices.target.validate_creature(game).is_some(),
            Target::Owner => self.choices.target.validate_player(Some(self.player_id)),
            Target::AnyOf(options) => options.iter().any(|option| self.valid_target(game, option)),
        };
    }

    fn valid_effect(&self, game: &mut Game) -> bool {
        return match &self.required.effect {
            Effect::Mana(mana) => self.choices.effect.validate_mana(mana),
            Effect::Discard(card_count) => {
                return if let Choice::And(choices) = &self.choices.effect {
                    return choices.len() == *card_count
                        && choices.iter().all(|choice| {
                            if let Choice::Card(card_id) = &choice {
                                if let Some(card) = game.get_card(*card_id) {
                                    return card.owner_id == self.player_id;
                                }
                            }
                            false
                        });
                } else {
                    false
                };
            }
            _ => true,
        };
    }
}

#[derive(Debug)]
pub struct Required {
    pub cost: Cost,
    pub target: Target,
    pub effect: Effect,
}

#[derive(Debug)]
pub struct Choices {
    pub cost: Choice,
    pub target: Choice,
    pub effect: Choice,
}

#[derive(Debug)]
pub enum Choice {
    None,
    Mana(Mana),
    Player(ObjectId),
    Card(ObjectId),
    And(Vec<Choice>),
}

impl Choice {
    fn validate_mana(&self, cost: &Mana) -> bool {
        match self {
            Choice::Mana(mana) => mana.enough(cost),
            Choice::And(choices) => choices.iter().any(|choice| choice.validate_mana(cost)),
            _ => false,
        }
    }

    fn validate_card(&self, card_id: ObjectId) -> bool {
        match self {
            Choice::Card(chosen_card) => *chosen_card == card_id,
            Choice::And(choices) => choices.iter().any(|choice| choice.validate_card(card_id)),
            _ => false,
        }
    }

    fn validate_player(&self, player_id: Option<ObjectId>) -> bool {
        match self {
            Choice::Player(chosen_player) => player_id == None || player_id == Some(*chosen_player),
            Choice::And(choices) => choices
                .iter()
                .any(|choice| choice.validate_player(player_id)),
            _ => false,
        }
    }

    fn validate_creature(&self, game: &mut Game) -> Option<ObjectId> {
        match self {
            Choice::Card(card_id) => {
                if let Some(card) = game.get_card(*card_id) {
                    if card.kind == CardType::Creature {
                        return Some(*card_id);
                    }
                }
                None
            }
            Choice::And(choices) => {
                for choice in choices.iter() {
                    let card_id = choice.validate_creature(game);
                    if card_id.is_some() {
                        return card_id;
                    }
                }
                None
            }
            _ => None,
        }
    }
}
