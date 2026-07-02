//! Mesure des faux positifs de l'automod (lecture seule, agregation locale).
//!
//! On ne capture aucune donnee nouvelle : le signal vit deja dans
//! `automod_reviews`. Une review est un "faux positif" (over-block) quand
//! l'automod a SUGGERE une vraie sanction mais que la decision humaine
//! terminale est plus clemente (downgrade ou "ignore").
//!
//! Aggregation en Rust sur un echantillon borne de reviews terminales de la
//! fenetre (voir `MAX_ROWS`) : le shape JSONB des `flags` (objet de booleens)
//! rend l'explosion en Rust plus lisible que du `jsonb_each` + CASE SQL, et le
//! volume par guild/fenetre reste modeste.

use std::collections::BTreeMap;

use axum::extract::Query;
use axum::extract::State;
use axum::Json;
use serde::Deserialize;
use serde::Serialize;

use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::extractors::ValidatedGuild;
use crate::adapters::inbound::http::state::AppState;
use sentinel_core::domain::entities::moderation::review::automod::AppliedAction;
use sentinel_core::domain::errors::DomainError;

/// Plafond d'echantillon : au-dela on tronque et on signale `capped=true`.
const MAX_ROWS: i64 = 5000;

#[derive(Debug, Deserialize)]
pub struct FpStatsQuery {
    /// Fenetre en jours (defaut 30, borne 1..=365).
    pub days: Option<i64>,
}

/// Stat globale ou par cat/action.
#[derive(Debug, Serialize)]
pub struct FpBucketDto {
    pub total: i64,
    pub overturned: i64,
    pub ignored: i64,
    pub fp_rate: f64,
}

impl FpBucketDto {
    fn from(acc: &Acc) -> Self {
        Self {
            total: acc.total,
            overturned: acc.overturned,
            ignored: acc.ignored,
            fp_rate: if acc.total > 0 {
                acc.overturned as f64 / acc.total as f64
            } else {
                0.0
            },
        }
    }
}

#[derive(Debug, Serialize)]
pub struct FpFlagStatDto {
    pub flag: String,
    pub total: i64,
    pub overturned: i64,
    pub ignored: i64,
    pub fp_rate: f64,
}

#[derive(Debug, Serialize)]
pub struct FpActionStatDto {
    pub suggested_action: String,
    pub total: i64,
    pub overturned: i64,
    pub ignored: i64,
    pub fp_rate: f64,
}

#[derive(Debug, Serialize)]
pub struct FpStatsDto {
    pub days: i64,
    /// True si l'echantillon a ete tronque a `MAX_ROWS` (stats approximatives).
    pub capped: bool,
    pub overall: FpBucketDto,
    pub by_flag: Vec<FpFlagStatDto>,
    pub by_suggested_action: Vec<FpActionStatDto>,
}

#[derive(Default, Clone)]
struct Acc {
    total: i64,
    overturned: i64,
    ignored: i64,
}

impl Acc {
    fn add(&mut self, overturned: bool, ignored: bool) {
        self.total += 1;
        if overturned {
            self.overturned += 1;
        }
        if ignored {
            self.ignored += 1;
        }
    }
}

#[derive(sqlx::FromRow)]
struct TerminalRow {
    suggested_action: String,
    applied_action: Option<String>,
    decided_action: Option<String>,
    flags: serde_json::Value,
}

/// Severite unifiee (echelle AppliedAction : ignore=0 < prevention < warn <
/// delete < mute < ban). Une valeur absente/inconnue vaut 0 (= aucune sanction).
fn severity(action: Option<&str>) -> u8 {
    action
        .and_then(AppliedAction::from_str)
        .map(|a| a.severity())
        .unwrap_or(0)
}

/// GET /api/automod/{guild_id}/fp-stats?days=30
///
/// Agrege les reviews terminales (applied/ignored/decided) de la fenetre et
/// mesure le taux de faux positifs (over-block) global, par flag detecteur, et
/// par action suggeree.
pub async fn fp_stats(
    State(state): State<AppState>,
    ValidatedGuild { guild_id }: ValidatedGuild,
    Query(params): Query<FpStatsQuery>,
) -> Result<Json<FpStatsDto>, ApiError> {
    let days = params.days.unwrap_or(30).clamp(1, 365);

    let rows: Vec<TerminalRow> = sqlx::query_as(
        "SELECT suggested_action, applied_action, decided_action, flags \
         FROM automod_reviews \
         WHERE guild_id = $1 \
           AND status IN ('applied','ignored','decided') \
           AND created_at >= NOW() - make_interval(days => $2) \
         ORDER BY created_at DESC \
         LIMIT $3",
    )
    .bind(guild_id.as_str())
    .bind(days as i32)
    .bind(MAX_ROWS)
    .fetch_all(&state.pg_pool)
    .await
    .map_err(|e| ApiError::from(DomainError::Internal(format!("fp-stats query : {e}"))))?;

    let capped = rows.len() as i64 >= MAX_ROWS;
    if capped {
        tracing::warn!(
            guild_id = %guild_id,
            days,
            max = MAX_ROWS,
            "fp-stats : echantillon tronque, stats approximatives"
        );
    }

    let mut overall = Acc::default();
    // Ordre stable (alpha) pour un rendu deterministe.
    let mut by_flag: BTreeMap<String, Acc> = BTreeMap::new();
    let mut by_action: BTreeMap<String, Acc> = BTreeMap::new();

    for r in &rows {
        let suggested_sev = severity(Some(&r.suggested_action));
        // Action humaine terminale : la resolution (applied_action) prime, sinon
        // le verdict de vote (decided_action). Absente => aucune sanction (0).
        let terminal = r.applied_action.as_deref().or(r.decided_action.as_deref());
        let terminal_sev = severity(terminal);

        // Over-block : l'automod a suggere une vraie sanction ET l'humain a
        // tranche plus clement (downgrade ou ignore).
        let overturned = suggested_sev > 0 && terminal_sev < suggested_sev;
        let ignored = terminal == Some("ignore") || terminal.is_none();

        overall.add(overturned, ignored);
        by_action
            .entry(r.suggested_action.clone())
            .or_default()
            .add(overturned, ignored);

        // Explose les flags detecteurs actifs (objet JSONB de booleens).
        if let Some(map) = r.flags.as_object() {
            for (flag, val) in map {
                if val.as_bool() == Some(true) {
                    by_flag
                        .entry(flag.clone())
                        .or_default()
                        .add(overturned, ignored);
                }
            }
        }
    }

    let mut by_flag_dto: Vec<FpFlagStatDto> = by_flag
        .into_iter()
        .map(|(flag, a)| FpFlagStatDto {
            flag,
            total: a.total,
            overturned: a.overturned,
            ignored: a.ignored,
            fp_rate: if a.total > 0 {
                a.overturned as f64 / a.total as f64
            } else {
                0.0
            },
        })
        .collect();
    // Tri par taux de FP decroissant (les detecteurs les plus "bruyants" en tete).
    by_flag_dto.sort_by(|a, b| {
        b.fp_rate
            .partial_cmp(&a.fp_rate)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(b.total.cmp(&a.total))
    });

    let by_action_dto: Vec<FpActionStatDto> = by_action
        .into_iter()
        .map(|(suggested_action, a)| FpActionStatDto {
            suggested_action,
            total: a.total,
            overturned: a.overturned,
            ignored: a.ignored,
            fp_rate: if a.total > 0 {
                a.overturned as f64 / a.total as f64
            } else {
                0.0
            },
        })
        .collect();

    Ok(Json(FpStatsDto {
        days,
        capped,
        overall: FpBucketDto::from(&overall),
        by_flag: by_flag_dto,
        by_suggested_action: by_action_dto,
    }))
}
