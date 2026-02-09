use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "session")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    pub ended_at: Option<DateTimeWithTimeZone>,
    pub host_id: Uuid,
    pub game_id: Option<Uuid>,
    pub game_version_id: Option<Uuid>,
    #[sea_orm(unique)]
    pub session_code: String,
    pub status: String,
    pub max_players: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::HostId",
        to = "super::user::Column::Id"
    )]
    Host,
    #[sea_orm(
        belongs_to = "super::game::Entity",
        from = "Column::GameId",
        to = "super::game::Column::Id"
    )]
    Game,
    #[sea_orm(
        belongs_to = "super::game_version::Entity",
        from = "Column::GameVersionId",
        to = "super::game_version::Column::Id"
    )]
    GameVersion,
    #[sea_orm(has_many = "super::player::Entity")]
    Players,
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Host.def()
    }
}

impl Related<super::game::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Game.def()
    }
}

impl Related<super::game_version::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::GameVersion.def()
    }
}

impl Related<super::player::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Players.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
