//! SeaORM Entity. Generated by sea-orm-codegen 0.9.2

use super::sea_orm_active_enums::{EndSettingType, WhitelistMintMode};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Default, Debug, DeriveEntity)]
pub struct Entity;

impl EntityName for Entity {
    fn table_name(&self) -> &str {
        "candy_machine_data"
    }
}

#[derive(Clone, Debug, PartialEq, DeriveModel, DeriveActiveModel, Serialize, Deserialize)]
pub struct Model {
    pub id: i64,
    pub uuid: Option<String>,
    pub price: Option<u64>,
    pub symbol: String,
    pub seller_fee_basis_points: u16,
    pub max_supply: u64,
    pub is_mutable: bool,
    pub retain_authority: Option<bool>,
    pub go_live_date: Option<i64>,
    pub items_available: u64,
    pub candy_machine_id: i64,
    pub mode: Option<WhitelistMintMode>,
    pub whitelist_mint: Option<Vec<u8>>,
    pub presale: Option<bool>,
    pub discount_price: Option<u64>,
    pub gatekeeper_network: Option<Vec<u8>>,
    pub expire_on_use: Option<bool>,
    pub prefix_name: Option<String>,
    pub name_length: Option<u32>,
    pub prefix_uri: Option<String>,
    pub uri_length: Option<u32>,
    pub is_sequential: Option<bool>,
    pub number: Option<u64>,
    pub end_setting_type: Option<EndSettingType>,
    pub name: Option<String>,
    pub uri: Option<String>,
    pub hash: Option<[u8; 32]>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveColumn)]
pub enum Column {
    Id,
    Uuid,
    Price,
    Symbol,
    SellerFeeBasisPoints,
    MaxSupply,
    IsMutable,
    RetainAuthority,
    GoLiveDate,
    ItemsAvailable,
    CandyMachineId,
    Mode,
    WhitelistMint,
    Presale,
    DiscountPrice,
    GatekeeperNetwork,
    ExpireOnUse,
    PrefixName,
    NameLength,
    PrefixUri,
    UriLength,
    IsSequential,
    Number,
    EndSettingType,
    Name,
    Uri,
    Hash,
}

#[derive(Copy, Clone, Debug, EnumIter, DerivePrimaryKey)]
pub enum PrimaryKey {
    Id,
}

impl PrimaryKeyTrait for PrimaryKey {
    type ValueType = i64;
    fn auto_increment() -> bool {
        true
    }
}

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {
    CandyMachine,
}

impl ColumnTrait for Column {
    type EntityName = Entity;
    fn def(&self) -> ColumnDef {
        match self {
            Self::Id => ColumnType::BigInteger.def(),
            Self::Uuid => ColumnType::BigInteger.def().null(),
            Self::Price => ColumnType::Binary.def().null(),
            Self::Symbol => ColumnType::Binary.def(),
            Self::SellerFeeBasisPoints => ColumnType::Binary.def(),
            Self::MaxSupply => ColumnType::Integer.def(),
            Self::IsMutable => ColumnType::Boolean.def().null(),
            Self::RetainAuthority => ColumnType::Boolean.def(),
            Self::GoLiveDate => ColumnType::Integer.def().null(),
            Self::ItemsAvailable => ColumnType::Integer.def(),
            Self::CandyMachineId => ColumnType::BigInteger.def(),
            Self::Mode => WhitelistMintMode::db_type().null(),
            Self::WhitelistMint => ColumnType::Binary.def().null(),
            Self::Presale => ColumnType::Boolean.def().null(),
            Self::DiscountPrice => ColumnType::Integer.def().null(),
            Self::GatekeeperNetwork => ColumnType::Binary.def().null(),
            Self::ExpireOnUse => ColumnType::Boolean.def().null(),
            Self::PrefixName => ColumnType::String.def().null(),
            Self::NameLength => ColumnType::Integer.def().null(),
            Self::PrefixUri => ColumnType::String.def().null(),
            Self::UriLength => ColumnType::Integer.def().null(),
            Self::IsSequential => ColumnType::Boolean.def().null(),
            Self::Number => ColumnType::Integer.def().null(),
            Self::EndSettingType => EndSettingType::db_type().null(),
            Self::Name => ColumnType::String.def().null(),
            Self::Uri => ColumnType::String.def().null(),
            Self::Hash => ColumnType::Binary.def().null(),
        }
    }
}
impl RelationTrait for Relation {
    fn def(&self) -> RelationDef {
        match self {
            Self::CandyMachine => Entity::belongs_to(super::candy_machine::Entity)
                .from(Column::CandyMachineId)
                .to(super::candy_machine::Column::Id)
                .into(),
        }
    }
}

impl Related<super::candy_machine::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::CandyMachine.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
