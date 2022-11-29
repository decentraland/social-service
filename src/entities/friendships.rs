//! `SeaORM` Entity. Generated by sea-orm-codegen 0.10.4

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "friendships")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub address1: String,
    pub address2: String,
    pub is_active: bool,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::friendship_history::Entity")]
    FriendshipHistory,
}

impl Related<super::friendship_history::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::FriendshipHistory.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}