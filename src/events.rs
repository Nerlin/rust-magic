use crate::{
    abilities::{Condition, Target},
    game::ObjectId,
};

#[derive(Debug)]
pub enum Event {
    Tap(CardEvent),
    Untap(CardEvent),
    Draw(CardEvent),
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
