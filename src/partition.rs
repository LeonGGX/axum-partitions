// partition.rs
use std::fmt::{Debug,};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "partitions")]
pub struct Model {
    #[sea_orm(primary_key)]
    #[serde(skip_deserializing)]
    pub id: i32,
    pub title: String,
    pub person_id: i32,
    pub genre_id: i32
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::persons::Entity",
        from = "Column::PersonId",
        to = "super::persons::Column::Id"
    )]
    Person,
    #[sea_orm(
    belongs_to = "super::genre::Entity",
    from = "Column::GenreId",
    to = "super::genre::Column::Id"
    )]
    Genre,
}

impl ActiveModelBehavior for ActiveModel {}
