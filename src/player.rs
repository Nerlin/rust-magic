use crate::mana::Mana;

pub struct Player {
    pub life: u16,
    pub mana: Mana
}

impl Player {
    pub fn new() -> Player {
        Player {
            life: 20,
            mana: Mana::new()
        }
    }
}