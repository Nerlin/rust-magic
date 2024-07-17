use crate::mana::Mana;

#[derive(Default)]
pub struct Abilities {
    pub activated: Vec<ActivatedAbility>,
    pub triggers: Vec<TriggeredAbility>,
}

#[derive(Clone, Debug)]
pub struct ActivatedAbility {
    pub cost: Cost,
    pub effect: Effect,
    pub target: Target,
}

#[derive(Clone, Debug)]
pub struct TriggeredAbility {
    pub condition: Condition,
    pub effect: Effect,
    pub target: Target,
}

#[derive(Clone, Debug)]
pub enum Cost {
    None,
    Mana(Mana),
    Tap(Target),
}

#[derive(Clone, Debug)]
pub enum Effect {
    None,
    Mana(Mana),
    Damage(u16),
}

#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub enum Condition {
    Tap(Target),
    Untap(Target),
    Draw,
}

#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub enum Target {
    None,
    Source,
    Owner,
    Player,
}
