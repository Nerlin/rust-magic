use crate::cards::Card;
use crate::player::Player;

pub struct Effect {
    pub kind: EffectKind,
    pub target: Option<Target>,
    pub resolver: Box<dyn Fn() -> ()>,
}

impl Effect {
    pub fn resolve(&self) {
        let resolver = self.resolver.as_ref();
        resolver()
    }
}

pub enum EffectKind {
    Stack,
    Immediate
}

pub enum Target {
    Player(Player),
    Card(Card),
}