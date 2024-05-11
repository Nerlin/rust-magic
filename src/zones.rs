use crate::cards::Permanent;

#[derive(Clone, Debug, Ord, PartialOrd, Eq, PartialEq)]
pub enum Zone {
    None,
    Hand,
    Library,
    Battlefield(Permanent),
    Graveyard,
    Stack,
    Exile,
    Sideboard
}