use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "tag")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    #[sea_orm(unique)]
    pub name: String,
    #[sea_orm(unique)]
    pub slug: String,
    pub category: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::game_tag::Entity")]
    GameTag,
}

impl Related<super::game_tag::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::GameTag.def()
    }
}

impl Related<super::game::Entity> for Entity {
    fn to() -> RelationDef {
        super::game_tag::Relation::Game.def()
    }

    fn via() -> Option<RelationDef> {
        Some(super::game_tag::Relation::Tag.def().rev())
    }
}

impl ActiveModelBehavior for ActiveModel {}
