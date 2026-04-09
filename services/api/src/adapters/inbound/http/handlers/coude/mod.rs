pub mod dto;
pub mod players;
pub mod combats;
pub mod bets;
pub mod economy;
pub mod inventory;
pub mod social;

// Re-export tous les handlers pour le router
pub use players::*;
pub use combats::*;
pub use bets::*;
pub use economy::*;
pub use inventory::*;
pub use social::*;
