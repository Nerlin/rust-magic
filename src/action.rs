use crate::{
    abilities::{Cost, Effect, Target},
    card::tap_card,
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
        return match &self.required.cost {
            Cost::None => true,
            Cost::Mana(_) => match &self.choices.cost {
                Choice::Mana(mana) => {
                    return if let Some(player) = game.get_player(self.player_id) {
                        player.mana -= *mana;
                        true
                    } else {
                        false
                    }
                }
                _ => false,
            },
            Cost::Tap(_) => match &self.choices.cost {
                Choice::Card(card_id) => tap_card(game, *card_id, Some(self.card_id)),
                _ => false,
            },
        };
    }

    pub fn valid(&self, game: &mut Game) -> bool {
        self.valid_cost() && self.valid_target() && self.valid_effect(game)
    }

    fn valid_cost(&self) -> bool {
        return match &self.required.cost {
            Cost::None => true,
            Cost::Mana(mana) => self.choices.cost.validate_mana(&mana),
            Cost::Tap(target) => match target {
                Target::Source => self.choices.cost.validate_card(self.card_id),
                _ => true,
            },
        };
    }

    fn valid_target(&self) -> bool {
        return match &self.required.target {
            Target::None => true,
            Target::Source => self.choices.target.validate_card(self.card_id),
            Target::Player => self.choices.target.validate_player(),
            Target::Owner => self.choices.target.validate_owner(self.player_id),
        };
    }

    fn valid_effect(&self, game: &mut Game) -> bool {
        return match &self.required.effect {
            Effect::Mana(mana) => self.choices.effect.validate_mana(mana),
            Effect::Discard(card_count) => {
                return if let Choice::CardsExact(cards) = &self.choices.effect {
                    return cards.len() == *card_count
                        && cards.iter().all(|card_id| {
                            if let Some(card) = game.get_card(*card_id) {
                                card.owner_id == self.player_id
                            } else {
                                false
                            }
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
    CardsExact(Vec<ObjectId>),
}

impl Choice {
    fn validate_mana(&self, cost: &Mana) -> bool {
        if let Choice::Mana(mana) = self {
            if mana.enough(cost) {
                return true;
            }
        }
        false
    }

    fn validate_card(&self, card_id: ObjectId) -> bool {
        if let Choice::Card(chosen_card) = self {
            if *chosen_card == card_id {
                return true;
            }
        }
        false
    }

    fn validate_player(&self) -> bool {
        return if let Choice::Player(_) = self {
            true
        } else {
            false
        };
    }

    fn validate_owner(&self, owner_id: ObjectId) -> bool {
        return if let Choice::Player(player_id) = self {
            *player_id == owner_id
        } else {
            false
        };
    }
}
