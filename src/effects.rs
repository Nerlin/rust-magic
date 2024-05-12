use crate::game::{Game, GameObject};
use crate::mana::Mana;
use crate::player::Player;

pub trait Effect {
    fn kind(&self) -> EffectKind;

    fn target(&self) -> Option<&GameObject>;

    fn resolve(&self);
}

pub enum EffectKind {
    Stacked,
    Immediate
}

pub struct ManaEffect {
    pub player: Player,
    pub mana: Mana,
}

impl Effect for ManaEffect {
    fn kind(&self) -> EffectKind {
        EffectKind::Immediate
    }

    fn target(&self) -> Option<&GameObject> {
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

pub trait Alive {
    fn gain_life(&mut self, life: u16);
    fn lose_life(&mut self, life: u16);
    fn take_damage(&mut self, damage: u16);
}

pub struct DamageEffect {
    pub game: Game,
    pub target: GameObject,
    pub damage: u16,
}

impl Effect for DamageEffect {
    fn kind(&self) -> EffectKind {
        EffectKind::Stacked
    }

    fn target(&self) -> Option<&GameObject> {
        Some(&self.target)
    }

    fn resolve(&self) {
        match self.target {
            GameObject::Player(id) => {
                if let Some(player) = self.game.borrow().get_player(id) {
                    player.borrow_mut().take_damage(self.damage);
                }
            }
            GameObject::Creature(id) => {
                if let Some(creature) = self.game.borrow().get_creature(id) {
                    creature.borrow_mut().take_damage(self.damage);
                }
            },
            _ => {}
        }
    }
}