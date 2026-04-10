use std::sync::Arc;

use async_trait::async_trait;

use crate::domain::errors::DomainError;
use crate::ports::inbound::manage_coude_economy::ManageCoudeEconomyUseCase;
use crate::ports::outbound::CoudeEconomyRepository;

pub struct ManageCoudeEconomyService {
    repo: Arc<dyn CoudeEconomyRepository>,
}

impl ManageCoudeEconomyService {
    pub fn new(repo: Arc<dyn CoudeEconomyRepository>) -> Self {
        Self { repo }
    }
}

fn require_positive(amount: i64) -> Result<(), DomainError> {
    if amount <= 0 {
        Err(DomainError::ValidationError(
            "Le montant doit etre positif".into(),
        ))
    } else {
        Ok(())
    }
}

#[async_trait]
impl ManageCoudeEconomyUseCase for ManageCoudeEconomyService {
    async fn transfer(
        &self,
        guild_id: &str,
        from_id: &str,
        to_id: &str,
        amount: i64,
    ) -> Result<(), DomainError> {
        require_positive(amount)?;
        if from_id == to_id {
            return Err(DomainError::ValidationError(
                "Impossible de se transferer des coins a soi-meme".into(),
            ));
        }
        self.repo.transfer(guild_id, from_id, to_id, amount).await
    }

    async fn steal(
        &self,
        guild_id: &str,
        thief_id: &str,
        victim_id: &str,
        amount: i64,
    ) -> Result<i64, DomainError> {
        require_positive(amount)?;
        if thief_id == victim_id {
            return Err(DomainError::ValidationError(
                "Impossible de se voler soi-meme".into(),
            ));
        }
        let stolen = self.repo.steal(guild_id, thief_id, victim_id, amount).await?;
        if stolen <= 0 {
            return Err(DomainError::ValidationError(
                "La victime n'a pas de coins a voler".into(),
            ));
        }
        Ok(stolen)
    }

    async fn record_casino_win(
        &self,
        guild_id: &str,
        user_id: &str,
        gain: i64,
    ) -> Result<(), DomainError> {
        if gain < 0 {
            return Err(DomainError::ValidationError(
                "Le gain ne peut pas etre negatif".into(),
            ));
        }
        self.repo.record_casino_win(guild_id, user_id, gain).await
    }

    async fn record_casino_loss(
        &self,
        guild_id: &str,
        user_id: &str,
        lost: i64,
    ) -> Result<(), DomainError> {
        if lost < 0 {
            return Err(DomainError::ValidationError(
                "La perte ne peut pas etre negative".into(),
            ));
        }
        self.repo.record_casino_loss(guild_id, user_id, lost).await
    }

    async fn record_casino_faillite(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<i64, DomainError> {
        self.repo.record_casino_faillite(guild_id, user_id).await
    }

    async fn count_casino_today(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<i64, DomainError> {
        self.repo.count_casino_today(guild_id, user_id).await
    }

    async fn sum_casino_gains_today(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<i64, DomainError> {
        self.repo.sum_casino_gains_today(guild_id, user_id).await
    }

    async fn count_steal_today(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<i64, DomainError> {
        self.repo.count_steal_today(guild_id, user_id).await
    }
}
