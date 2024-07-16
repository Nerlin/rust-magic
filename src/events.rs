use crate::{
    abilities::{Condition, Target},
    game::ObjectId,
};

pub enum Event {
    Tap(CardEvent),
    Untap(CardEvent),
}

impl Event {
    /// Defines if this event meets the trigger condition.
    pub fn meets(&self, condition: &Condition) -> bool {
        match self {
            Event::Tap(event) => {
                if let Condition::Tap(target) = condition {
                    match target {
                        Target::Source => event.source == event.card,
                        _ => false,
                    }
                } else {
                    false
                }
            }
            Event::Untap(event) => {
                if let Condition::Untap(target) = condition {
                    match target {
                        Target::Source => event.source == event.card,
                        _ => false,
                    }
                } else {
                    false
                }
            }
        }
    }
}

pub struct CardEvent {
    /// The player whos card triggered an event
    pub owner: ObjectId,

    /// The card that triggered the event
    pub source: ObjectId,

    pub card: ObjectId,
}
