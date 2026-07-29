use crate::domain::errors::DomainError; use async_trait::async_trait;
#[derive(Debug,Clone)] pub struct StealResult { pub success:bool,pub amount:i64 }
#[async_trait] pub trait CoudeStealUseCase:Send+Sync { async fn steal(&self,guild_id:&str,thief_id:&str,victim_id:&str,is_fourbe:bool)->Result<StealResult,DomainError>; }
