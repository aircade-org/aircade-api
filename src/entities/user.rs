use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "user")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    #[sea_orm(unique)]
    pub email: String,
    #[sea_orm(unique)]
    pub username: String,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub bio: Option<String>,
    pub email_verified: bool,
    pub role: String,
    pub subscription_plan: String,
    pub subscription_expires_at: Option<DateTimeWithTimeZone>,
    pub account_status: String,
    pub suspension_reason: Option<String>,
    pub last_login_at: Option<DateTimeWithTimeZone>,
    pub last_login_ip: Option<String>,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    pub deleted_at: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::auth_provider::Entity")]
    AuthProvider,
    #[sea_orm(has_many = "super::refresh_token::Entity")]
    RefreshToken,
}

impl Related<super::auth_provider::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::AuthProvider.def()
    }
}

impl Related<super::refresh_token::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::RefreshToken.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
