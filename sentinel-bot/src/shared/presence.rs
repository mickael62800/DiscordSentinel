//! Publication de la presence en direct dans Redis.
//!
//! Le bot est la SEULE source possible : lui seul recoit les evenements
//! Discord et connait les permissions des salons. L'API se contente de lire.
//!
//! # Pourquoi Redis et pas Postgres
//!
//! La presence est volatile. Une ligne oubliee en base apres un crash du bot
//! afficherait indefiniment quelqu'un dans un salon vide, et personne n'irait
//! la corriger a la main. Ici chaque cle porte un TTL : si le bot se tait, la
//! presence disparait d'elle-meme plutot que de mentir.
//!
//! # Pourquoi un instantane complet plutot que des deltas
//!
//! A chaque changement de voice state, on republie l'etat COMPLET de la
//! guilde depuis le cache Serenity. Appliquer des deltas (« untel est parti »)
//! obligerait a esperer qu'aucun evenement ne se perde ; un seul manque et la
//! liste derive sans jamais se corriger. Un instantane est auto-reparateur.

use redis::AsyncCommands;
use serde::Serialize;

/// Duree de vie des cles. Genereuse par rapport au seuil de fraicheur cote
/// API (3 min) : c'est l'API qui decide de ne plus afficher, le TTL n'est
/// qu'un filet de securite contre les cles orphelines.
const TTL_SECONDS: u64 = 600;

/// Nombre d'auteurs recents conserves par salon ecrit. Au-dela, la ligne
/// devient une liste de noms illisible.
const MAX_RECENT_AUTHORS: isize = 8;

/// Fenetre d'activite ecrite, alignee sur `TEXT_WINDOW_SECONDS` du domaine.
const TEXT_WINDOW_SECONDS: i64 = 15 * 60;

pub fn voice_key(guild_id: &str) -> String {
    format!("sentinel:presence:voice:{guild_id}")
}

pub fn text_index_key(guild_id: &str) -> String {
    format!("sentinel:presence:text:{guild_id}")
}

pub fn text_channel_key(guild_id: &str, channel_id: &str) -> String {
    format!("sentinel:presence:text:{guild_id}:{channel_id}")
}

#[derive(Debug, Clone, Serialize)]
pub struct VoiceMemberDto {
    pub user_id: String,
    pub username: String,
    pub self_mute: bool,
    pub self_deaf: bool,
    pub server_mute: bool,
    pub streaming: bool,
    pub video: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct VoiceChannelDto {
    pub channel_id: String,
    pub channel_name: String,
    pub members: Vec<VoiceMemberDto>,
    /// Salon que @everyone ne peut pas voir. L'API s'en sert pour ne pas le
    /// servir aux visiteurs anonymes.
    pub restreint: bool,
}

#[derive(Debug, Clone, Serialize)]
struct VoiceSnapshot {
    channels: Vec<VoiceChannelDto>,
    updated_at: String,
}

/// Remplace l'instantane vocal d'une guilde.
///
/// Une seule cle contenant tout l'etat : la remplacer est atomique du point
/// de vue du lecteur, alors qu'un hash champ par champ laisserait l'API lire
/// un etat a moitie mis a jour.
pub async fn publish_voice(
    conn: &mut redis::aio::MultiplexedConnection,
    guild_id: &str,
    channels: Vec<VoiceChannelDto>,
) -> redis::RedisResult<()> {
    let snapshot = VoiceSnapshot {
        channels,
        updated_at: chrono::Utc::now().to_rfc3339(),
    };
    let payload = serde_json::to_string(&snapshot).unwrap_or_else(|_| "{}".into());

    // SET avec expiration en une commande : un SET puis EXPIRE separes
    // laisseraient une cle immortelle si le bot mourait entre les deux.
    conn.set_ex::<_, _, ()>(voice_key(guild_id), payload, TTL_SECONDS)
        .await
}

/// Enregistre qu'un membre vient d'ecrire dans un salon.
///
/// Le ZSET porte l'horodatage en score : lire les auteurs recents devient un
/// ZRANGEBYSCORE, et les vieux disparaissent par simple nettoyage plutot que
/// par relecture-reecriture — ce qui evite les pertes en cas de messages
/// simultanes.
pub async fn touch_text(
    conn: &mut redis::aio::MultiplexedConnection,
    guild_id: &str,
    channel_id: &str,
    channel_name: &str,
    username: &str,
) -> redis::RedisResult<()> {
    let now = chrono::Utc::now().timestamp();
    let cle_salon = text_channel_key(guild_id, channel_id);

    // Un membre qui reecrit remonte au lieu de figurer deux fois.
    let _: () = conn.zadd(&cle_salon, username, now).await?;

    // Purge de la fenetre puis du surplus : sans elle, un salon actif depuis
    // des mois accumulerait tous ses participants.
    let _: () = conn
        .zrembyscore(&cle_salon, 0, now - TEXT_WINDOW_SECONDS)
        .await?;
    let _: () = conn
        .zremrangebyrank(&cle_salon, 0, -MAX_RECENT_AUTHORS - 1)
        .await?;
    let _: () = conn.expire(&cle_salon, TTL_SECONDS as i64).await?;

    // Index des salons actifs, avec leur nom et la date du dernier message.
    let index = text_index_key(guild_id);
    let valeur = serde_json::json!({
        "channel_name": channel_name,
        "last_message_at": now,
    })
    .to_string();
    let _: () = conn.hset(&index, channel_id, valeur).await?;
    let _: () = conn.expire(&index, TTL_SECONDS as i64).await?;

    Ok(())
}
