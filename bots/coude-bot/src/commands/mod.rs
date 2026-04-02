pub mod accepter;
pub mod assurance;
pub mod casino;
pub mod coude;
pub mod defend_item;
pub mod leaderboard;
pub mod pari;
pub mod prime;
pub mod profil;
pub mod refuser;
pub mod shop_cmd;
pub mod train;
pub mod voler;

use serenity::builder::CreateCommand;

pub fn all() -> Vec<CreateCommand> {
    vec![
        coude::register(),
        profil::register(),
        shop_cmd::register(),
        casino::register(),
        prime::register(),
        leaderboard::register(),
        pari::register(),
        voler::register(),
        assurance::register(),
        train::register(),
    ]
}
