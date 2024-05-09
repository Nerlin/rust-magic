use crate::cards::Card;
use crate::mana::Mana;
use crate::player::Player;

pub trait Effect {
    fn kind(&self) -> EffectKind;

    fn target(&self) -> Option<Target>;

    fn resolve(&self);
}

pub enum EffectKind {
    Stack,
    Immediate
}

pub enum Target {
    Player(Player),
    Card(Card),
}

pub struct ManaEffect {
    pub player: Player,
    pub mana: Mana,
}

impl Effect for ManaEffect {
    fn kind(&self) -> EffectKind {
        EffectKind::Immediate
    }

    fn target(&self) -> Option<Target> {
        None
    }

    fn resolve(&self) {
        let mut player = self.player.borrow_mut();
        for (color, amount) in &self.mana {
            let player_amount = player.mana.get(color).unwrap_or(&0).clone();
            player.mana.insert(color.clone(), player_amount + amount);
        }
    }
}