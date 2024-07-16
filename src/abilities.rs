use crate::mana::Mana;

pub struct Abilities {
    pub activated: Vec<ActivatedAbility>,
    pub triggers: Vec<TriggeredAbility>,
}

#[derive(Clone)]
pub struct ActivatedAbility {
    pub cost: Cost,
    pub effect: Effect,
    pub target: Target,
}

#[derive(Clone)]
pub struct TriggeredAbility {
    pub condition: Condition,
    pub effect: Effect,
    pub target: Target,
}

#[derive(Clone)]
pub enum Cost {
    None,
    Mana(Mana),
    Tap(Target),
}

#[derive(Clone)]
pub enum Effect {
    None,
    Mana(Mana),
    Damage(u16),
}

#[derive(Clone)]
pub enum Condition {
    Tap(Target),
    Untap(Target),
}

#[derive(Clone)]
pub enum Target {
    None,
    Source,
    Owner,
    Player,
}
