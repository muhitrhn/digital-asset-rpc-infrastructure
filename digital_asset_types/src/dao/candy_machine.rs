//! SeaORM Entity. Generated by sea-orm-codegen 0.9.2

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Default, Debug, DeriveEntity)]
pub struct Entity;

impl EntityName for Entity {
    fn table_name(&self) -> &str {
        "candy_machine"
    }
}

#[derive(Clone, Debug, PartialEq, DeriveModel, DeriveActiveModel, Serialize, Deserialize)]
pub struct Model {
    pub id: Vec<u8>,
    pub features: Option<u64>,
    pub authority: Vec<u8>,
    pub mint_authority: Option<Vec<u8>>,
    pub wallet: Option<Vec<u8>>,
    pub token_mint: Option<Vec<u8>>,
    pub items_redeemed: i32,
    pub candy_guard_pda: Option<Vec<u8>>,
    pub version: u8,
    pub collection_mint: Option<Vec<u8>>,
    pub allow_thaw: Option<bool>,
    pub frozen_count: Option<u64>,
    pub mint_start: Option<i64>,
    pub freeze_time: Option<i64>,
    pub freeze_fee: Option<u64>,
    pub created_at: Option<DateTimeWithTimeZone>,
    pub last_minted: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveColumn)]
pub enum Column {
    Id,
    Features,
    Authority,
    MintAuthority,
    Wallet,
    TokenMint,
    ItemsRedeemed,
    CandyGuardPda,
    Version,
    CollectionMint,
    AllowThaw,
    FrozenCount,
    MintStart,
    FreezeTime,
    FreezeFee,
    CreatedAt,
    LastMinted,
}

#[derive(Copy, Clone, Debug, EnumIter, DerivePrimaryKey)]
pub enum PrimaryKey {
    Id,
}

impl PrimaryKeyTrait for PrimaryKey {
    type ValueType = Vec<u8>;
    fn auto_increment() -> bool {
        false
    }
}

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {
    CandyMachineData,
    CandyGuard,
    CandyMachineCreators,
}

impl ColumnTrait for Column {
    type EntityName = Entity;
    fn def(&self) -> ColumnDef {
        match self {
            Self::Id => ColumnType::Binary.def(),
            Self::Features => ColumnType::Integer.def().null(),
            Self::Authority => ColumnType::Binary.def(),
            Self::MintAuthority => ColumnType::Binary.def().null(),
            Self::Wallet => ColumnType::Binary.def().null(),
            Self::TokenMint => ColumnType::Binary.def().null(),
            Self::ItemsRedeemed => ColumnType::Integer.def(),
            Self::CandyGuardPda => ColumnType::Binary.def().null(),
            Self::Version => ColumnType::Integer.def(),
            Self::CollectionMint => ColumnType::Binary.def().null(),
            Self::AllowThaw => ColumnType::Boolean.def().null(),
            Self::FrozenCount => ColumnType::Integer.def().null(),
            Self::MintStart => ColumnType::Integer.def().null(),
            Self::FreezeTime => ColumnType::Integer.def().null(),
            Self::FreezeFee => ColumnType::Integer.def().null(),
            Self::CreatedAt => ColumnType::TimestampWithTimeZone.def().null(),
            Self::LastMinted => ColumnType::TimestampWithTimeZone.def().null(),
        }
    }
}

impl RelationTrait for Relation {
    fn def(&self) -> RelationDef {
        match self {
            Self::CandyMachineCreators => {
                Entity::has_many(super::candy_machine_creators::Entity).into()
            }
            Self::CandyMachineData => Entity::has_one(super::candy_machine_data::Entity).into(),
            Self::CandyGuard => Entity::belongs_to(super::candy_guard::Entity)
                .from(Column::CandyGuardPda)
                .to(super::candy_guard::Column::Base)
                .into(),
        }
    }
}

impl Related<super::candy_machine_data::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::CandyMachineData.def()
    }
}

impl Related<super::candy_guard::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::CandyGuard.def()
    }
}

impl Related<super::candy_machine_creators::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::CandyMachineCreators.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}