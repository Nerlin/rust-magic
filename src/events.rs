

#[derive(Clone, Debug, Ord, PartialOrd, Eq, PartialEq)]
pub enum Event {
    PhaseUpkeep,
    PhaseDraw,
    PhasePrecombat,
    PhasePostcombat,
    PhaseEnd,

    PermanentTap,
    PermanentUntap,
}