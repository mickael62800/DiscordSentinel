//! DTOs HTTP du jeu Influence.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use sentinel_core::domain::entities::influence::org_membership::OrgMemberView;
use sentinel_core::domain::entities::influence::organization::Organization;
use sentinel_core::ports::inbound::influence::manage_organizations::OrgInfo;
use sentinel_core::ports::inbound::influence::view_profile::{CapitalView, ProfileView};

#[derive(Debug, Serialize)]
pub struct CapitalViewDto {
    pub tier: String,
    pub stars: String,
    /// Valeur exacte, presente uniquement quand on consulte son propre profil.
    pub exact: Option<i64>,
}

impl From<CapitalView> for CapitalViewDto {
    fn from(v: CapitalView) -> Self {
        Self {
            tier: v.tier.to_string(),
            stars: v.stars.to_string(),
            exact: v.exact,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ProfileViewDto {
    pub username: String,
    pub is_self: bool,
    pub influence: CapitalViewDto,
    pub money: CapitalViewDto,
    pub reputation_tier: String,
    pub reputation_exact: Option<i64>,
    pub information: CapitalViewDto,
    pub network: CapitalViewDto,
    pub joined_at: DateTime<Utc>,
}

impl From<ProfileView> for ProfileViewDto {
    fn from(p: ProfileView) -> Self {
        Self {
            username: p.username,
            is_self: p.is_self,
            influence: p.influence.into(),
            money: p.money.into(),
            reputation_tier: p.reputation_tier.to_string(),
            reputation_exact: p.reputation_exact,
            information: p.information.into(),
            network: p.network.into(),
            joined_at: p.joined_at,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct OrganizationDto {
    pub id: Uuid,
    pub guild_id: String,
    pub kind: String,
    pub kind_label: String,
    pub emoji: String,
    pub name: String,
    pub motto: String,
    pub treasury: i64,
    pub reputation: i64,
    pub influence: i64,
    pub discord_role_id: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl From<Organization> for OrganizationDto {
    fn from(o: Organization) -> Self {
        Self {
            id: o.id,
            guild_id: o.guild_id,
            kind: o.kind.as_str().to_string(),
            kind_label: o.kind.label().to_string(),
            emoji: o.kind.emoji().to_string(),
            name: o.name,
            motto: o.motto,
            treasury: o.treasury,
            reputation: o.reputation,
            influence: o.influence,
            discord_role_id: o.discord_role_id,
            created_at: o.created_at,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct OrgRelationDto {
    pub other: String,
    pub relation: String,
    pub emoji: String,
}

#[derive(Debug, Serialize)]
pub struct OrgInfoDto {
    #[serde(flatten)]
    pub org: OrganizationDto,
    pub member_count: i64,
    pub relations: Vec<OrgRelationDto>,
    #[serde(default)]
    pub collective_influence: i64,
    #[serde(default)]
    pub collective_reputation: i64,
}

impl From<OrgInfo> for OrgInfoDto {
    fn from(i: OrgInfo) -> Self {
        Self {
            org: i.org.into(),
            member_count: i.member_count,
            relations: i
                .relations
                .into_iter()
                .map(|r| OrgRelationDto {
                    other: r.other_org_name,
                    relation: r.relation.label().to_string(),
                    emoji: r.relation.emoji().to_string(),
                })
                .collect(),
            collective_influence: i.collective_influence,
            collective_reputation: i.collective_reputation,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct TreasuryOpDto {
    pub name: String,
    #[serde(default)]
    pub actor_user_id: String,
    #[serde(default)]
    pub actor_username: String,
    pub amount: i64,
}

#[derive(Debug, Deserialize)]
pub struct PayMemberDto {
    pub name: String,
    #[serde(default)]
    pub actor_user_id: String,
    #[serde(default)]
    pub actor_username: String,
    #[serde(default)]
    pub beneficiary_user_id: String,
    #[serde(default)]
    pub beneficiary_username: String,
    pub amount: i64,
}

#[derive(Debug, Serialize)]
pub struct TreasuryMovementDto {
    pub kind: String,
    pub kind_label: String,
    pub amount: i64,
    pub treasury_after: i64,
    pub actor_username: String,
}

#[derive(Debug, Serialize)]
pub struct OrgRankDto {
    pub name: String,
    pub treasury: i64,
    pub member_count: i64,
    #[serde(default)]
    pub collective_influence: i64,
}

impl From<sentinel_core::ports::inbound::influence::manage_organizations::OrgRankEntry>
    for OrgRankDto
{
    fn from(e: sentinel_core::ports::inbound::influence::manage_organizations::OrgRankEntry) -> Self {
        Self {
            name: e.name,
            treasury: e.treasury,
            member_count: e.member_count,
            collective_influence: e.collective_influence,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct DividendResultDto {
    pub paid_count: i64,
    pub per_member: i64,
    pub total: i64,
    pub treasury_left: i64,
}

impl From<sentinel_core::ports::inbound::influence::manage_organizations::DividendResult>
    for DividendResultDto
{
    fn from(
        r: sentinel_core::ports::inbound::influence::manage_organizations::DividendResult,
    ) -> Self {
        Self {
            paid_count: r.paid_count,
            per_member: r.per_member,
            total: r.total,
            treasury_left: r.treasury_left,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct TreasuryViewDto {
    pub org_name: String,
    pub balance: i64,
    pub movements: Vec<TreasuryMovementDto>,
}

impl From<sentinel_core::domain::entities::influence::treasury::TreasuryView> for TreasuryViewDto {
    fn from(v: sentinel_core::domain::entities::influence::treasury::TreasuryView) -> Self {
        Self {
            org_name: v.org_name,
            balance: v.balance,
            movements: v
                .movements
                .into_iter()
                .map(|m| TreasuryMovementDto {
                    kind: m.kind.as_str().to_string(),
                    kind_label: m.kind.label().to_string(),
                    amount: m.amount,
                    treasury_after: m.treasury_after,
                    actor_username: m.actor_username,
                })
                .collect(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct OrgMemberDto {
    pub username: String,
    pub role: String,
    pub role_label: String,
    pub joined_at: DateTime<Utc>,
}

impl From<OrgMemberView> for OrgMemberDto {
    fn from(m: OrgMemberView) -> Self {
        Self {
            username: m.username,
            role: m.role.as_str().to_string(),
            role_label: m.role.label().to_string(),
            joined_at: m.joined_at,
        }
    }
}

use sentinel_core::ports::inbound::influence::manage_votes::MotionState;

#[derive(Debug, Serialize)]
pub struct MotionStateDto {
    pub motion_id: Uuid,
    pub org_name: String,
    pub title: String,
    pub status: String,
    pub status_label: String,
    pub pour: i64,
    pub contre: i64,
    pub abstention: i64,
    #[serde(default)]
    pub pour_weight: i64,
    #[serde(default)]
    pub contre_weight: i64,
}

impl From<MotionState> for MotionStateDto {
    fn from(s: MotionState) -> Self {
        Self {
            motion_id: s.motion.id,
            org_name: s.org_name,
            title: s.motion.title,
            status: s.motion.status.as_str().to_string(),
            status_label: s.motion.status.label().to_string(),
            pour: s.tally.pour,
            contre: s.tally.contre,
            abstention: s.tally.abstention,
            pour_weight: s.tally.pour_weight,
            contre_weight: s.tally.contre_weight,
        }
    }
}

use sentinel_core::ports::inbound::influence::manage_capital::{CapitalOverview, ConversionOutcome};

#[derive(Debug, Serialize)]
pub struct CapitalLineDto {
    pub capital: String,
    pub label: String,
    pub emoji: String,
    pub value: i64,
}

#[derive(Debug, Serialize)]
pub struct MovementDto {
    pub capital: String,
    pub emoji: String,
    pub delta: i64,
    pub reason: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct CapitalOverviewDto {
    pub lines: Vec<CapitalLineDto>,
    pub movements: Vec<MovementDto>,
}

impl From<CapitalOverview> for CapitalOverviewDto {
    fn from(o: CapitalOverview) -> Self {
        Self {
            lines: o
                .lines
                .into_iter()
                .map(|l| CapitalLineDto {
                    capital: l.capital.as_str().to_string(),
                    label: l.capital.label().to_string(),
                    emoji: l.capital.emoji().to_string(),
                    value: l.value,
                })
                .collect(),
            movements: o
                .movements
                .into_iter()
                .map(|m| MovementDto {
                    capital: m.capital.as_str().to_string(),
                    emoji: m.capital.emoji().to_string(),
                    delta: m.delta,
                    reason: m.reason,
                    created_at: m.created_at,
                })
                .collect(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ConversionOutcomeDto {
    pub source_label: String,
    pub target_label: String,
    pub spent: i64,
    pub gained: i64,
    pub new_source: i64,
    pub new_target: i64,
}

impl From<ConversionOutcome> for ConversionOutcomeDto {
    fn from(o: ConversionOutcome) -> Self {
        Self {
            source_label: o.kind.source().label().to_string(),
            target_label: o.kind.target().label().to_string(),
            spent: o.spent,
            gained: o.gained,
            new_source: o.new_source,
            new_target: o.new_target,
        }
    }
}

use sentinel_core::domain::entities::influence::law::law_effect_label;
use sentinel_core::ports::inbound::influence::manage_laws::LawState;

#[derive(Debug, Serialize)]
pub struct LawStateDto {
    pub law_id: Uuid,
    pub title: String,
    pub body: String,
    pub status: String,
    pub status_label: String,
    pub channel_id: Option<String>,
    pub message_id: Option<String>,
    pub pour: i64,
    pub contre: i64,
    pub abstention: i64,
    #[serde(default)]
    pub pour_weight: i64,
    #[serde(default)]
    pub contre_weight: i64,
    /// Effet mecanique lisible si la loi est adoptee (None = loi narrative).
    pub effect_label: Option<String>,
    pub effect_value: Option<i64>,
}

impl From<LawState> for LawStateDto {
    fn from(s: LawState) -> Self {
        Self {
            law_id: s.law.id,
            title: s.law.title,
            body: s.law.body,
            status: s.law.status.as_str().to_string(),
            status_label: s.law.status.label().to_string(),
            channel_id: s.law.channel_id,
            message_id: s.law.message_id,
            pour: s.tally.pour,
            contre: s.tally.contre,
            abstention: s.tally.abstention,
            pour_weight: s.tally.pour_weight,
            contre_weight: s.tally.contre_weight,
            effect_label: s
                .law
                .effect_key
                .as_deref()
                .and_then(|k| law_effect_label(k).map(str::to_string)),
            effect_value: s.law.effect_value,
        }
    }
}

use sentinel_core::domain::entities::influence::information::{Information, Investigation};
use sentinel_core::ports::inbound::influence::manage_information::RevealOutcome;

#[derive(Debug, Serialize)]
pub struct InvestigationDto {
    pub id: Uuid,
    pub target_username: String,
    pub subject: String,
    pub resolves_at: DateTime<Utc>,
}

impl From<Investigation> for InvestigationDto {
    fn from(i: Investigation) -> Self {
        Self {
            id: i.id,
            target_username: i.target_username,
            subject: i.subject,
            resolves_at: i.resolves_at,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct InformationDto {
    pub id: Uuid,
    pub target_username: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
}

impl From<Information> for InformationDto {
    fn from(i: Information) -> Self {
        Self {
            id: i.id,
            target_username: i.target_username,
            content: i.content,
            created_at: i.created_at,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct RevealOutcomeDto {
    pub content: String,
    pub target_user_id: String,
    pub target_username: String,
    pub reputation_loss: i64,
}

impl From<RevealOutcome> for RevealOutcomeDto {
    fn from(o: RevealOutcome) -> Self {
        Self {
            content: o.content,
            target_user_id: o.target_user_id,
            target_username: o.target_username,
            reputation_loss: o.reputation_loss,
        }
    }
}

use sentinel_core::domain::entities::influence::archive::ArchiveEntry;

#[derive(Debug, Serialize)]
pub struct ArchiveEntryDto {
    pub event_type: String,
    pub summary: String,
    pub occurred_at: DateTime<Utc>,
}

impl From<ArchiveEntry> for ArchiveEntryDto {
    fn from(e: ArchiveEntry) -> Self {
        let p = &e.payload;
        let s = |k: &str| p.get(k).and_then(|v| v.as_str()).unwrap_or("").to_string();
        let summary = match e.event_type.as_str() {
            "org_created" => format!("🏛️ {} a fondé « {} » ({})", s("founder"), s("name"), s("kind")),
            "org_relation" => format!("🔗 {} → {} : {}", s("org"), s("other"), s("relation")),
            "law_adopted" => format!("📜 Loi adoptée : « {} »", s("title")),
            "law_rejected" => format!("📜 Loi rejetée : « {} »", s("title")),
            "scandal" => {
                let target = s("target");
                if target.is_empty() {
                    format!("💥 Scandale : {}", s("content"))
                } else {
                    format!("💥 Scandale visant {} : {}", target, s("content"))
                }
            }
            other => format!("• {other}"),
        };
        Self {
            event_type: e.event_type,
            summary,
            occurred_at: e.occurred_at,
        }
    }
}
