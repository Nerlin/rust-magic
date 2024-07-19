use crate::{
    abilities::{Condition, Target},
    action::Action,
    game::{Game, ObjectId, Stacked},
    turn::Step,
};

#[derive(Debug)]
pub enum Event {
    Tap(CardEvent),
    Untap(CardEvent),
    Draw(CardEvent),
    Phase(PhaseEvent),
}

impl Event {
    /// Defines if this event meets the trigger condition.
    pub fn meets(&self, condition: &Condition) -> bool {
        match self {
            Event::Tap(event) => {
                if let Condition::Tap(target) = condition {
                    match target {
                        Target::Source => {
                            if let Some(source) = event.source {
                                source == event.card
                            } else {
                                false
                            }
                        }
                        _ => false,
                    }
                } else {
                    false
                }
            }
            Event::Untap(event) => {
                if let Condition::Untap(target) = condition {
                    match target {
                        Target::Source => {
                            if let Some(source) = event.source {
                                source == event.card
                            } else {
                                false
                            }
                        }
                        _ => false,
                    }
                } else {
                    false
                }
            }
            Event::Draw(_) => condition == &Condition::Draw,
            Event::Phase(event) => {
                if let Condition::Phase(phase) = condition {
                    phase == &event.phase
                } else {
                    false
                }
            }
        }
    }
}

#[derive(Debug, Default)]
pub struct CardEvent {
    /// The player whos card triggered an event
    pub owner: ObjectId,

    /// The card that triggered the event
    pub source: Option<ObjectId>,

    /// The card targeted by the event
    pub card: ObjectId,
}

#[derive(Debug)]
pub struct PhaseEvent {
    pub owner: ObjectId,
    pub phase: Step,
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

                game.stack.push(Stacked::Ability {
                    effect: trigger.effect.clone(),
                    action,
                });
            }
        }
    }
}
