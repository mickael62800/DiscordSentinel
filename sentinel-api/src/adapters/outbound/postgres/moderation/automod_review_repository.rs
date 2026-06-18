use async_trait::async_trait;
use chrono::DateTime;
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use super::super::pg_err_ctx;
use sentinel_core::domain::entities::moderation::review::automod::AutomodReview;
use sentinel_core::domain::entities::moderation::review::automod::DiscussionChannel;
use sentinel_core::domain::entities::moderation::review::automod::DiscussionMessage;
use sentinel_core::domain::entities::moderation::review::automod::NewAutomodReview;
use sentinel_core::domain::entities::moderation::review::automod::NewDiscussionChannel;
use sentinel_core::domain::entities::moderation::review::automod::ReviewVote;
use sentinel_core::domain::errors::DomainError;
use crate::ports::outbound::moderation::automod_review_repository::AutomodReviewRepository;

const TBL: &str = "automod_reviews";
fn pg_err(e: sqlx::Error) -> DomainError { pg_err_ctx(TBL, e) }

/// Construit l'entree JSON d'un incident (pour la liste agregee `incidents`).
fn incident_json(r: &NewAutomodReview) -> serde_json::Value {
    serde_json::json!({
        "message_id": r.message_id.as_str(),
        "channel_id": r.channel_id.as_str(),
        "content_preview": r.content_preview,
        "score": r.score,
        "reason": r.reason,
        "suggested_action": r.suggested_action.as_str(),
        "at": chrono::Utc::now().to_rfc3339(),
    })
}

#[derive(sqlx::FromRow)]
struct Row {
    id: Uuid,
    guild_id: String,
    channel_id: String,
    message_id: String,
    user_id: String,
    user_name: String,
    content_preview: String,
    suggested_action: String,
    score: f64,
    reason: String,
    flags: serde_json::Value,
    status: String,
    applied_action: Option<String>,
    resolved_by_id: Option<String>,
    resolved_by_name: Option<String>,
    resolved_source: Option<String>,
    created_at: DateTime<Utc>,
    resolved_at: Option<DateTime<Utc>>,
    voting_deadline: Option<DateTime<Utc>>,
    decided_action: Option<String>,
    #[sqlx(default)]
    quorum_met: bool,
    decided_at: Option<DateTime<Utc>>,
    #[sqlx(default)]
    incident_count: i32,
    #[sqlx(default)]
    cumulative_score: f64,
    #[sqlx(default)]
    incidents: serde_json::Value,
}

impl From<Row> for AutomodReview {
    fn from(r: Row) -> Self {
        Self {
            id: r.id,
            guild_id: r.guild_id.into(),
            channel_id: r.channel_id.into(),
            message_id: r.message_id.into(),
            user_id: r.user_id.into(),
            user_name: r.user_name,
            content_preview: r.content_preview,
            suggested_action: r.suggested_action,
            score: r.score,
            reason: r.reason,
            flags: r.flags,
            status: r.status,
            applied_action: r.applied_action,
            resolved_by_id: r.resolved_by_id,
            resolved_by_name: r.resolved_by_name,
            resolved_source: r.resolved_source,
            created_at: r.created_at,
            resolved_at: r.resolved_at,
            voting_deadline: r.voting_deadline,
            decided_action: r.decided_action,
            quorum_met: r.quorum_met,
            decided_at: r.decided_at,
            incident_count: r.incident_count,
            cumulative_score: r.cumulative_score,
            incidents: if r.incidents.is_null() { serde_json::json!([]) } else { r.incidents },
        }
    }
}

#[derive(sqlx::FromRow)]
struct VoteRow {
    id: Uuid,
    review_id: Uuid,
    voter_id: String,
    voter_name: String,
    vote_action: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(sqlx::FromRow)]
struct DiscussionRow {
    id: Uuid,
    review_id: Uuid,
    guild_id: String,
    channel_id: String,
    opened_by_id: String,
    opened_by_name: String,
    created_at: DateTime<Utc>,
}

impl From<DiscussionRow> for DiscussionChannel {
    fn from(r: DiscussionRow) -> Self {
        Self {
            id: r.id,
            review_id: r.review_id,
            guild_id: r.guild_id,
            channel_id: r.channel_id,
            opened_by_id: r.opened_by_id,
            opened_by_name: r.opened_by_name,
            created_at: r.created_at,
        }
    }
}

impl From<VoteRow> for ReviewVote {
    fn from(r: VoteRow) -> Self {
        Self {
            id: r.id,
            review_id: r.review_id,
            voter_id: r.voter_id,
            voter_name: r.voter_name,
            vote_action: r.vote_action,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

pub struct PgAutomodReviewRepository {
    pool: PgPool,
}

impl PgAutomodReviewRepository {
    pub fn new(pool: PgPool) -> Self { Self { pool } }
}

#[async_trait]
impl AutomodReviewRepository for PgAutomodReviewRepository {
    async fn create(&self, r: NewAutomodReview) -> Result<AutomodReview, DomainError> {
        // Mode vote si une echeance est fournie : statut 'voting'.
        let status = if r.voting_deadline.is_some() { "voting" } else { "pending" };
        let incidents = serde_json::json!([incident_json(&r)]);
        let row: Row = sqlx::query_as(
            "INSERT INTO automod_reviews \
                (guild_id, channel_id, message_id, user_id, user_name, content_preview, \
                 suggested_action, score, reason, flags, status, voting_deadline, \
                 incident_count, cumulative_score, incidents) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,1,$13,$14) \
             RETURNING *",
        )
        .bind(r.guild_id.as_str())
        .bind(r.channel_id.as_str())
        .bind(r.message_id.as_str())
        .bind(r.user_id.as_str())
        .bind(&r.user_name)
        .bind(&r.content_preview)
        .bind(r.suggested_action.as_str())
        .bind(r.score)
        .bind(&r.reason)
        .bind(&r.flags)
        .bind(status)
        .bind(r.voting_deadline)
        .bind(r.score)
        .bind(&incidents)
        .fetch_one(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(row.into())
    }

    async fn create_or_merge(
        &self,
        r: NewAutomodReview,
        aggregate: bool,
        window_minutes: i64,
    ) -> Result<(AutomodReview, bool), DomainError> {
        if aggregate {
            // Fenetre d'inactivite : on ne fusionne que dans une carte ayant eu
            // une infraction recemment. 0/negatif => pas de limite (legacy).
            let window = window_minutes.max(0);
            // Serialise les agregations concurrentes du meme (guild, user) :
            // sans ca, deux messages quasi simultanes pourraient creer 2 cartes
            // ou perdre un incident (read-modify-write sur le tableau JSON).
            let mut tx = self.pool.begin().await.map_err(pg_err)?;
            sqlx::query("SELECT pg_advisory_xact_lock(hashtext($1))")
                .bind(format!("automod_review:{}:{}", r.guild_id.as_str(), r.user_id.as_str()))
                .execute(&mut *tx)
                .await
                .map_err(pg_err)?;

            // Carte ouverte existante pour ce (guild, user) ET active (dernier
            // incident dans la fenetre). Si window = 0 -> pas de filtre temporel.
            let existing: Option<Row> = sqlx::query_as(
                "SELECT * FROM automod_reviews \
                 WHERE guild_id = $1 AND user_id = $2 AND status = 'voting' \
                   AND ($3 = 0 OR last_incident_at > NOW() - make_interval(mins => $3)) \
                 ORDER BY last_incident_at DESC LIMIT 1",
            )
            .bind(r.guild_id.as_str())
            .bind(r.user_id.as_str())
            .bind(window as i32)
            .fetch_optional(&mut *tx)
            .await
            .map_err(pg_err)?;

            if let Some(prev) = existing {
                // Agrege l'incident dans la carte existante.
                let mut incidents = if prev.incidents.is_null() {
                    serde_json::json!([])
                } else {
                    prev.incidents.clone()
                };
                if let Some(arr) = incidents.as_array_mut() {
                    arr.push(incident_json(&r));
                }
                let new_count = prev.incident_count + 1;
                let new_cumulative = prev.cumulative_score + r.score;
                let new_max_score = prev.score.max(r.score);
                let new_action = sentinel_core::domain::entities::moderation::review::automod::more_severe_suggested(
                    &prev.suggested_action,
                    r.suggested_action.as_str(),
                );
                // Plafond anti-troll : la deadline ne peut etre repoussee au-dela
                // de created_at + 7 jours (un membre tres actif ne garde pas la
                // carte ouverte indefiniment).
                let cap = prev.created_at + chrono::Duration::days(7);
                let new_deadline = r.voting_deadline.map(|d| d.min(cap));
                let updated: Row = sqlx::query_as(
                    "UPDATE automod_reviews SET \
                        incidents = $1, incident_count = $2, cumulative_score = $3, \
                        score = $4, suggested_action = $5, reason = $6, voting_deadline = $7, \
                        content_preview = $9, channel_id = $10, message_id = $11, \
                        last_incident_at = NOW() \
                     WHERE id = $8 AND status = 'voting' \
                     RETURNING *",
                )
                .bind(&incidents)
                .bind(new_count)
                .bind(new_cumulative)
                .bind(new_max_score)
                .bind(&new_action)
                .bind(&r.reason)
                .bind(new_deadline)
                .bind(prev.id)
                .bind(&r.content_preview)
                .bind(r.channel_id.as_str())
                .bind(r.message_id.as_str())
                .fetch_one(&mut *tx)
                .await
                .map_err(pg_err)?;
                tx.commit().await.map_err(pg_err)?;
                return Ok((updated.into(), true));
            }

            // Aucune carte ouverte : on cree dans la meme transaction (sous le
            // verrou) pour eviter une creation concurrente en double.
            let status = if r.voting_deadline.is_some() { "voting" } else { "pending" };
            let incidents = serde_json::json!([incident_json(&r)]);
            let row: Row = sqlx::query_as(
                "INSERT INTO automod_reviews \
                    (guild_id, channel_id, message_id, user_id, user_name, content_preview, \
                     suggested_action, score, reason, flags, status, voting_deadline, \
                     incident_count, cumulative_score, incidents) \
                 VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,1,$13,$14) \
                 RETURNING *",
            )
            .bind(r.guild_id.as_str())
            .bind(r.channel_id.as_str())
            .bind(r.message_id.as_str())
            .bind(r.user_id.as_str())
            .bind(&r.user_name)
            .bind(&r.content_preview)
            .bind(r.suggested_action.as_str())
            .bind(r.score)
            .bind(&r.reason)
            .bind(&r.flags)
            .bind(status)
            .bind(r.voting_deadline)
            .bind(r.score)
            .bind(&incidents)
            .fetch_one(&mut *tx)
            .await
            .map_err(pg_err)?;
            tx.commit().await.map_err(pg_err)?;
            return Ok((row.into(), false));
        }
        // Pas d'agregation : creation normale.
        let review = self.create(r).await?;
        Ok((review, false))
    }

    async fn get(&self, id: Uuid) -> Result<Option<AutomodReview>, DomainError> {
        let row: Option<Row> = sqlx::query_as("SELECT * FROM automod_reviews WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(pg_err)?;
        Ok(row.map(Into::into))
    }

    async fn find_by_message_id(
        &self,
        guild_id: &str,
        message_id: &str,
    ) -> Result<Option<AutomodReview>, DomainError> {
        let row: Option<Row> = sqlx::query_as(
            "SELECT * FROM automod_reviews \
             WHERE guild_id = $1 AND message_id = $2 \
             ORDER BY created_at DESC LIMIT 1",
        )
        .bind(guild_id)
        .bind(message_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(row.map(Into::into))
    }

    async fn list_pending(
        &self,
        guild_id: &str,
        limit: i64,
    ) -> Result<Vec<AutomodReview>, DomainError> {
        let rows: Vec<Row> = sqlx::query_as(
            "SELECT * FROM automod_reviews \
             WHERE guild_id = $1 AND status = 'pending' \
             ORDER BY created_at DESC LIMIT $2",
        )
        .bind(guild_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn list_recent(
        &self,
        guild_id: &str,
        limit: i64,
    ) -> Result<Vec<AutomodReview>, DomainError> {
        let rows: Vec<Row> = sqlx::query_as(
            "SELECT * FROM automod_reviews \
             WHERE guild_id = $1 \
             ORDER BY created_at DESC LIMIT $2",
        )
        .bind(guild_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn resolve(
        &self,
        id: Uuid,
        applied_action: &str,
        resolved_by_id: &str,
        resolved_by_name: &str,
        resolved_source: &str,
    ) -> Result<AutomodReview, DomainError> {
        let new_status = if applied_action == "ignore" { "ignored" } else { "applied" };
        let row: Option<Row> = sqlx::query_as(
            "UPDATE automod_reviews SET \
                status = $1, applied_action = $2, resolved_by_id = $3, \
                resolved_by_name = $4, resolved_source = $5, resolved_at = NOW() \
             WHERE id = $6 AND status IN ('pending','decided') \
             RETURNING *",
        )
        .bind(new_status)
        .bind(applied_action)
        .bind(resolved_by_id)
        .bind(resolved_by_name)
        .bind(resolved_source)
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(pg_err)?;

        match row {
            Some(r) => Ok(r.into()),
            None => {
                // Soit la review n existe pas, soit deja resolue.
                let exists: Option<(String,)> =
                    sqlx::query_as("SELECT status FROM automod_reviews WHERE id = $1")
                        .bind(id)
                        .fetch_optional(&self.pool)
                        .await
                        .map_err(pg_err)?;
                match exists {
                    None => Err(DomainError::NotFound(format!("review {id} introuvable"))),
                    Some((s,)) => Err(DomainError::Conflict(format!(
                        "review deja resolue (status={s})"
                    ))),
                }
            }
        }
    }

    async fn close_ignored(
        &self,
        id: Uuid,
        actor_id: &str,
        actor_name: &str,
        source: &str,
    ) -> Result<AutomodReview, DomainError> {
        // Clore immediatement, meme pendant le vote (statut voting inclus).
        let row: Option<Row> = sqlx::query_as(
            "UPDATE automod_reviews SET \
                status = 'ignored', applied_action = 'ignore', resolved_by_id = $2, \
                resolved_by_name = $3, resolved_source = $4, resolved_at = NOW() \
             WHERE id = $1 AND status IN ('pending','voting','decided') \
             RETURNING *",
        )
        .bind(id)
        .bind(actor_id)
        .bind(actor_name)
        .bind(source)
        .fetch_optional(&self.pool)
        .await
        .map_err(pg_err)?;

        match row {
            Some(r) => Ok(r.into()),
            None => {
                let exists: Option<(String,)> =
                    sqlx::query_as("SELECT status FROM automod_reviews WHERE id = $1")
                        .bind(id)
                        .fetch_optional(&self.pool)
                        .await
                        .map_err(pg_err)?;
                match exists {
                    None => Err(DomainError::NotFound(format!("review {id} introuvable"))),
                    Some((s,)) => Err(DomainError::Conflict(format!(
                        "review deja close (status={s})"
                    ))),
                }
            }
        }
    }

    async fn reopen(&self, id: Uuid, deadline_hours: i64) -> Result<AutomodReview, DomainError> {
        let mut tx = self.pool.begin().await.map_err(pg_err)?;

        // Repasse en 'voting' : efface la resolution + le verdict + fixe une
        // nouvelle echeance. Seules les reviews closes (applied|ignored) sont
        // rouvrables.
        let row: Option<Row> = sqlx::query_as(
            "UPDATE automod_reviews SET \
                status = 'voting', applied_action = NULL, decided_action = NULL, \
                quorum_met = FALSE, decided_at = NULL, resolved_by_id = NULL, \
                resolved_by_name = NULL, resolved_source = NULL, resolved_at = NULL, \
                voting_deadline = NOW() + make_interval(hours => $2) \
             WHERE id = $1 AND status IN ('applied','ignored') \
             RETURNING *",
        )
        .bind(id)
        .bind(deadline_hours as i32)
        .fetch_optional(&mut *tx)
        .await
        .map_err(pg_err)?;

        let review = match row {
            Some(r) => r,
            None => {
                let exists: Option<(String,)> =
                    sqlx::query_as("SELECT status FROM automod_reviews WHERE id = $1")
                        .bind(id)
                        .fetch_optional(&mut *tx)
                        .await
                        .map_err(pg_err)?;
                tx.rollback().await.map_err(pg_err)?;
                return match exists {
                    None => Err(DomainError::NotFound(format!("review {id} introuvable"))),
                    Some((s,)) => Err(DomainError::Conflict(format!(
                        "dossier non rouvrable (status={s})"
                    ))),
                };
            }
        };

        // Repart sur un vote propre : on efface les votes du tour precedent.
        sqlx::query("DELETE FROM automod_review_votes WHERE review_id = $1")
            .bind(id)
            .execute(&mut *tx)
            .await
            .map_err(pg_err)?;

        tx.commit().await.map_err(pg_err)?;
        Ok(review.into())
    }

    async fn upsert_vote(
        &self,
        review_id: Uuid,
        voter_id: &str,
        voter_name: &str,
        vote_action: &str,
    ) -> Result<(), DomainError> {
        // Refuse le vote si la review n'est plus ouverte.
        let status: Option<(String,)> =
            sqlx::query_as("SELECT status FROM automod_reviews WHERE id = $1")
                .bind(review_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(pg_err)?;
        match status {
            None => return Err(DomainError::NotFound(format!("review {review_id} introuvable"))),
            Some((s,)) if s != "voting" => {
                return Err(DomainError::Conflict(format!("vote ferme (status={s})")))
            }
            _ => {}
        }

        sqlx::query(
            "INSERT INTO automod_review_votes (review_id, voter_id, voter_name, vote_action) \
             VALUES ($1,$2,$3,$4) \
             ON CONFLICT (review_id, voter_id) \
             DO UPDATE SET vote_action = EXCLUDED.vote_action, \
                           voter_name = EXCLUDED.voter_name, updated_at = NOW()",
        )
        .bind(review_id)
        .bind(voter_id)
        .bind(voter_name)
        .bind(vote_action)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(())
    }

    async fn list_votes(&self, review_id: Uuid) -> Result<Vec<ReviewVote>, DomainError> {
        let rows: Vec<VoteRow> = sqlx::query_as(
            "SELECT * FROM automod_review_votes WHERE review_id = $1 ORDER BY created_at",
        )
        .bind(review_id)
        .fetch_all(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn decide(
        &self,
        id: Uuid,
        decided_action: &str,
        quorum_met: bool,
    ) -> Result<AutomodReview, DomainError> {
        let row: Option<Row> = sqlx::query_as(
            "UPDATE automod_reviews SET \
                status = 'decided', decided_action = $1, quorum_met = $2, decided_at = NOW() \
             WHERE id = $3 AND status = 'voting' \
             RETURNING *",
        )
        .bind(decided_action)
        .bind(quorum_met)
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(pg_err)?;

        match row {
            Some(r) => Ok(r.into()),
            None => {
                let exists: Option<(String,)> =
                    sqlx::query_as("SELECT status FROM automod_reviews WHERE id = $1")
                        .bind(id)
                        .fetch_optional(&self.pool)
                        .await
                        .map_err(pg_err)?;
                match exists {
                    None => Err(DomainError::NotFound(format!("review {id} introuvable"))),
                    Some((s,)) => Err(DomainError::Conflict(format!(
                        "vote deja cloture (status={s})"
                    ))),
                }
            }
        }
    }

    async fn list_expired_voting(&self, limit: i64) -> Result<Vec<AutomodReview>, DomainError> {
        let rows: Vec<Row> = sqlx::query_as(
            "SELECT * FROM automod_reviews \
             WHERE status = 'voting' AND voting_deadline IS NOT NULL AND voting_deadline < NOW() \
             ORDER BY voting_deadline ASC LIMIT $1",
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn find_discussion(&self, review_id: Uuid) -> Result<Option<DiscussionChannel>, DomainError> {
        let row: Option<DiscussionRow> = sqlx::query_as(
            "SELECT * FROM automod_discussion_channels WHERE review_id = $1",
        )
        .bind(review_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(row.map(Into::into))
    }

    async fn create_discussion(
        &self,
        d: NewDiscussionChannel,
    ) -> Result<(DiscussionChannel, bool), DomainError> {
        // Idempotence : UNIQUE(review_id). On tente l'insert ; en cas de
        // conflit on renvoie l'existant avec created=false.
        let inserted: Option<DiscussionRow> = sqlx::query_as(
            "INSERT INTO automod_discussion_channels \
                (review_id, guild_id, channel_id, opened_by_id, opened_by_name) \
             VALUES ($1,$2,$3,$4,$5) \
             ON CONFLICT (review_id) DO NOTHING \
             RETURNING *",
        )
        .bind(d.review_id)
        .bind(&d.guild_id)
        .bind(&d.channel_id)
        .bind(&d.opened_by_id)
        .bind(&d.opened_by_name)
        .fetch_optional(&self.pool)
        .await
        .map_err(pg_err)?;

        if let Some(row) = inserted {
            return Ok((row.into(), true));
        }
        // Conflit : un salon existait deja -> on le renvoie.
        let existing: DiscussionRow = sqlx::query_as(
            "SELECT * FROM automod_discussion_channels WHERE review_id = $1",
        )
        .bind(d.review_id)
        .fetch_one(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok((existing.into(), false))
    }

    async fn append_discussion_messages(
        &self,
        messages: &[DiscussionMessage],
    ) -> Result<u64, DomainError> {
        if messages.is_empty() {
            return Ok(0);
        }
        let mut tx = self.pool.begin().await.map_err(pg_err)?;
        let mut inserted = 0u64;
        for m in messages {
            let res = sqlx::query(
                "INSERT INTO automod_discussion_messages \
                    (review_id, discord_message_id, author_id, author_name, author_is_bot, content, sent_at) \
                 VALUES ($1,$2,$3,$4,$5,$6,$7) \
                 ON CONFLICT (review_id, discord_message_id) DO NOTHING",
            )
            .bind(m.review_id)
            .bind(&m.discord_message_id)
            .bind(&m.author_id)
            .bind(&m.author_name)
            .bind(m.author_is_bot)
            .bind(&m.content)
            .bind(m.sent_at)
            .execute(&mut *tx)
            .await
            .map_err(pg_err)?;
            inserted += res.rows_affected();
        }
        tx.commit().await.map_err(pg_err)?;
        Ok(inserted)
    }

    async fn list_discussion_messages(
        &self,
        review_id: Uuid,
    ) -> Result<Vec<DiscussionMessage>, DomainError> {
        let rows: Vec<DiscussionMsgRow> = sqlx::query_as(
            "SELECT review_id, discord_message_id, author_id, author_name, author_is_bot, content, sent_at \
             FROM automod_discussion_messages WHERE review_id = $1 ORDER BY sent_at ASC",
        )
        .bind(review_id)
        .fetch_all(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }
}

#[derive(sqlx::FromRow)]
struct DiscussionMsgRow {
    review_id: Uuid,
    discord_message_id: String,
    author_id: String,
    author_name: String,
    author_is_bot: bool,
    content: String,
    sent_at: DateTime<Utc>,
}

impl From<DiscussionMsgRow> for DiscussionMessage {
    fn from(r: DiscussionMsgRow) -> Self {
        DiscussionMessage {
            review_id: r.review_id,
            discord_message_id: r.discord_message_id,
            author_id: r.author_id,
            author_name: r.author_name,
            author_is_bot: r.author_is_bot,
            content: r.content,
            sent_at: r.sent_at,
        }
    }
}
