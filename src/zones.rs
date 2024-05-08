


#[derive(Clone, Debug, Ord, PartialOrd, Eq, PartialEq)]
pub enum Zone {
    None,
    Hand,
    Library,
    Battlefield,
    Graveyard,
    Stack,
    Exile,
    Sideboard
}