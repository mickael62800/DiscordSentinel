pub mod accepter;
pub mod annuler;
pub mod assurance;
pub mod cagnotte;
pub mod classe;
pub mod coude;
pub mod defend_item;
pub mod donner;
pub mod hp;
pub mod leaderboard;
pub mod pari;
pub mod potion;
pub mod prime;
pub mod profil;
pub mod refuser;
pub mod repos;
pub mod reset_stats;
pub mod resume;
pub mod saison;
pub mod shop_cmd;
pub mod train;
pub mod voler;

use serenity::builder::CreateCommand;

pub fn all() -> Vec<CreateCommand> {
    vec![
        coude::register(),
        profil::register(),
        shop_cmd::register(),
        prime::register(),
        leaderboard::register(),
        pari::register(),
        potion::register(),
        voler::register(),
        assurance::register(),
        train::register(),
        classe::register(),
        donner::register(),
        hp::register(),
        repos::register(),
        saison::register(),
        reset_stats::register(),
        resume::register(),
        cagnotte::register(),
    ]
}
