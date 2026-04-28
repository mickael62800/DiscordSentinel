//! Impl Postgres de `CoudeFlavorTemplatesRepository` (Phase 3 #9 audit).

use async_trait::async_trait;
use sqlx::PgPool;

use crate::domain::errors::DomainError;
use crate::ports::outbound::coude::flavor_templates_repository::CoudeFlavorTemplatesRepository;

use super::super::pg_err_ctx;
const TBL: &str = "coude_flavor_templates";
fn pg_err(e: sqlx::Error) -> DomainError { pg_err_ctx(TBL, e) }

pub struct PgCoudeFlavorTemplatesRepository {
    pool: PgPool,
}

impl PgCoudeFlavorTemplatesRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl CoudeFlavorTemplatesRepository for PgCoudeFlavorTemplatesRepository {
    async fn random_by_key(
        &self,
        key: &str,
        locale: &str,
    ) -> Result<Option<String>, DomainError> {
        // Tirage pondere : on duplique virtuellement chaque ligne `weight`
        // fois via generate_series, puis ORDER BY random() LIMIT 1. Volumetrie
        // attendue tres faible (<= ~200 lignes par key) → cout negligeable.
        let row: Option<(String,)> = sqlx::query_as(
            r#"
            SELECT content
            FROM coude_flavor_templates t
            CROSS JOIN LATERAL generate_series(1, t.weight) AS gs(_n)
            WHERE t.key = $1 AND t.locale = $2
            ORDER BY random()
            LIMIT 1
            "#,
        )
        .bind(key)
        .bind(locale)
        .fetch_optional(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(row.map(|(c,)| c))
    }
}
