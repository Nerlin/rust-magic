use std::collections::HashMap;
use std::rc::Rc;

use crate::effects::Effect;

#[derive(Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub enum Event {
    PhaseUpkeep,
    PhaseDraw,
    PhasePrecombat,
    PhasePostcombat,
    PhaseEnd,

    PermanentCreate(u64),
    PermanentTap(u64),
    PermanentUntap(u64),
}


pub struct EventLoop {
    counter: u64,
    subscribers: HashMap<Event, HashMap<u64, Rc<dyn EventHandler>>>,
}

pub trait EventHandler {
    fn handle(&self, event: &Event) -> EventResult;
}

impl EventLoop {
    pub fn new() -> EventLoop {
        EventLoop {
            counter: 0,
            subscribers: HashMap::new()
        }
    }

    pub fn subscribe(&mut self, event: Event, subscriber: Rc<dyn EventHandler>) -> u64 {
        let subscriber_id =  self.counter;
        if let Some(subscribers) = self.subscribers.get_mut(&event) {
            subscribers.insert(subscriber_id, subscriber);
        } else {
            self.subscribers.insert(event, HashMap::from([(subscriber_id, subscriber)]));
        }

        if self.counter < u64::MAX {
            self.counter = self.counter + 1;
        } else {
            self.counter = 0;
        }

        subscriber_id
    }

    pub fn unsubscribe(&mut self, subscriber_id: u64) {
        for subscribers in self.subscribers.values_mut() {
            subscribers.remove(&subscriber_id);
        }
    }

    pub fn emit(&self, event: Event) -> EventResult {
        if let Some(subscribers) = self.subscribers.get(&event) {
            for subscriber in subscribers.values() {
                if let EventResult::Prevented = subscriber.handle(&event) {
                    return EventResult::Prevented
                }
            }
        }
        return EventResult::Resolved
    }
}

pub enum EventResult {
    None,
    Resolved,
    Prevented,
}

impl EventResult {
    pub fn is_resolved(&self) -> bool {
        if let EventResult::Resolved = self {
            true
        } else {
            false
        }
    }

    pub fn is_prevented(&self) -> bool {
        if let EventResult::Prevented = self {
            true
        } else {
            false
        }
    }
}

pub struct Stack {
    pub effects: Vec<Rc<dyn Effect>>,
}

impl Stack {
    pub fn new() -> Stack {
        Stack {
            effects: vec![],
        }
    }

    pub fn push(&mut self, effect: Rc<dyn Effect>) {
        self.effects.push(effect)
    }

    pub fn resolve(&mut self) {
        if let Some(effect) = self.effects.pop() {
            effect.resolve()
        }
    }
}
