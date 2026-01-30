use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "game")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    pub deleted_at: Option<DateTimeWithTimeZone>,
    pub owner_id: Uuid,
    pub title: String,
    #[sea_orm(unique)]
    pub slug: String,
    pub description: Option<String>,
    pub thumbnail: Option<String>,
    pub technology: String,
    pub status: String,
    pub visibility: String,
    pub min_players: i32,
    pub max_players: i32,
    pub published_version_id: Option<Uuid>,
    pub game_screen_code: Option<String>,
    pub controller_screen_code: Option<String>,
    pub play_count: i64,
    pub total_play_time: i64,
    pub avg_rating: f32,
    pub review_count: i64,
    pub forked_from_id: Option<Uuid>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::OwnerId",
        to = "super::user::Column::Id"
    )]
    Owner,
    #[sea_orm(has_many = "super::game_version::Entity")]
    GameVersions,
    #[sea_orm(has_many = "super::session::Entity")]
    Sessions,
    #[sea_orm(has_many = "super::game_asset::Entity")]
    GameAssets,
    #[sea_orm(has_many = "super::game_tag::Entity")]
    GameTags,
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Owner.def()
    }
}

impl Related<super::game_version::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::GameVersions.def()
    }
}

impl Related<super::session::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Sessions.def()
    }
}

impl Related<super::game_asset::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::GameAssets.def()
    }
}

impl Related<super::game_tag::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::GameTags.def()
    }
}

impl Related<super::tag::Entity> for Entity {
    fn to() -> RelationDef {
        super::game_tag::Relation::Tag.def()
    }

    fn via() -> Option<RelationDef> {
        Some(super::game_tag::Relation::Game.def().rev())
    }
}

impl ActiveModelBehavior for ActiveModel {}
