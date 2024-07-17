use crate::{
    abilities::{Cost, Effect, Target},
    game::ObjectId,
    mana::Mana,
};

#[derive(Debug)]
pub struct Action {
    pub player_id: ObjectId,
    pub card_id: ObjectId,
    required: Required,
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

    pub fn valid(&self) -> bool {
        self.valid_cost() && self.valid_target() && self.valid_effect()
    }

    fn valid_cost(&self) -> bool {
        return match &self.required.cost {
            Cost::None => true,
            Cost::Mana(mana) => self.choices.cost.validate_mana(&mana),
            Cost::Tap(target) => match target {
                Target::Source => self.choices.cost.validate_tap(self.card_id),
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

    fn valid_effect(&self) -> bool {
        return match &self.required.effect {
            Effect::Mana(mana) => self.choices.effect.validate_mana(mana),
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
    Tap(ObjectId),
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

    fn validate_tap(&self, card_id: ObjectId) -> bool {
        if let Choice::Tap(tapped_card) = self {
            if *tapped_card == card_id {
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
