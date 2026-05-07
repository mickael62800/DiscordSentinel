#![allow(dead_code)]
use serenity::model::user::User;

/// Verifie si un compte est suspect (trop recent).
pub struct AccountChecker {
    min_age: std::time::Duration,
}

impl AccountChecker {
    pub fn new(min_age_secs: u64) -> Self {
        Self {
            min_age: std::time::Duration::from_secs(min_age_secs),
        }
    }

    /// Retourne `true` si le compte est suspect (trop jeune).
    pub fn is_suspicious(&self, user: &User) -> bool {
        let created_at = user.created_at();
        let now = serenity::model::Timestamp::now();

        let age_secs = now.unix_timestamp() - created_at.unix_timestamp();
        if age_secs < 0 {
            return true;
        }

        (age_secs as u64) < self.min_age.as_secs()
    }

    /// Retourne l'age du compte en heures.
    pub fn account_age_hours(&self, user: &User) -> i64 {
        let created_at = user.created_at();
        let now = serenity::model::Timestamp::now();
        (now.unix_timestamp() - created_at.unix_timestamp()) / 3600
    }
}
