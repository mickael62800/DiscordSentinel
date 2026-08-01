//! Presence en direct : qui est en vocal, ou ca discute a l'ecrit.
//!
//! Etat volatil par nature. Il ne va PAS en base : une ligne oubliee apres un
//! crash du bot afficherait un fantome dans un salon vide, et c'est le genre
//! d'erreur que personne ne va corriger a la main. Il vit dans Redis, sous
//! une cle a expiration : si le bot se tait, la presence disparait d'elle-meme
//! au lieu de mentir.
//!
//! # Ce qui est publiable
//!
//! La page membre est PUBLIQUE. Or la presence dans un salon prive est une
//! information privee : annoncer « Kalyx est dans #staff » a tout Internet
//! serait une fuite. Seuls les salons visibles par @everyone sont publies, et
//! le filtrage se fait cote bot — lui seul connait les permissions Discord.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Au-dela, une presence est consideree comme perimee : le bot a probablement
/// cesse de publier. Plus court que le TTL Redis, pour que la page cesse
/// d'afficher des fantomes avant meme que la cle expire.
pub const STALE_AFTER_SECONDS: i64 = 180;

/// Fenetre d'activite ecrite. Quinze minutes : assez pour qu'un salon calme
/// paraisse vivant, assez court pour ne pas annoncer une conversation finie.
pub const TEXT_WINDOW_SECONDS: i64 = 15 * 60;

/// Un membre dans un salon vocal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceMember {
    pub user_id: String,
    pub username: String,
    /// Micro coupe par le membre lui-meme.
    pub self_mute: bool,
    /// Casque coupe. Implique qu'il n'entend rien : l'afficher evite qu'on
    /// s'etonne de son silence.
    pub self_deaf: bool,
    /// Coupe par un moderateur. Distinct de `self_mute` : ce n'est pas le
    /// meme fait, et les confondre donnerait une image fausse.
    pub server_mute: bool,
    pub streaming: bool,
    pub video: bool,
}

impl VoiceMember {
    /// Le membre peut-il parler ? Sert au rendu : un micro coupe s'affiche
    /// autrement qu'un membre silencieux.
    pub fn can_speak(&self) -> bool {
        !self.self_mute && !self.server_mute && !self.self_deaf
    }
}

/// Un salon vocal et ses occupants.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceChannelPresence {
    pub channel_id: String,
    pub channel_name: String,
    pub members: Vec<VoiceMember>,
}

/// Instantane de la presence vocale d'une guilde.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoicePresence {
    pub channels: Vec<VoiceChannelPresence>,
    /// Date de la derniere publication par le bot.
    pub updated_at: DateTime<Utc>,
}

impl VoicePresence {
    /// L'instantane est-il encore credible ?
    ///
    /// Un instantane perime vaut mieux masque qu'affiche : montrer « 11 en
    /// vocal » alors que le bot est tombe il y a une heure est pire que ne
    /// rien montrer.
    pub fn is_fresh(&self, now: DateTime<Utc>) -> bool {
        (now - self.updated_at).num_seconds() < STALE_AFTER_SECONDS
    }

    pub fn total_members(&self) -> usize {
        self.channels.iter().map(|c| c.members.len()).sum()
    }

    /// Salons non vides, les plus peuples d'abord.
    ///
    /// Un salon vide n'a rien a faire dans la vitrine : la liste montre ou il
    /// se passe quelque chose, pas l'arborescence du serveur.
    pub fn occupied_channels(&self) -> Vec<&VoiceChannelPresence> {
        let mut occupes: Vec<&VoiceChannelPresence> =
            self.channels.iter().filter(|c| !c.members.is_empty()).collect();
        occupes.sort_by(|a, b| b.members.len().cmp(&a.members.len()));
        occupes
    }
}

/// Activite recente dans un salon ecrit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextChannelActivity {
    pub channel_id: String,
    pub channel_name: String,
    /// Pseudos ayant parle dans la fenetre, du plus recent au plus ancien.
    pub recent_authors: Vec<String>,
    pub last_message_at: DateTime<Utc>,
}

impl TextChannelActivity {
    pub fn is_within_window(&self, now: DateTime<Utc>) -> bool {
        (now - self.last_message_at).num_seconds() < TEXT_WINDOW_SECONDS
    }
}

#[cfg(test)]
#[path = "tests/presence.rs"]
mod tests;
