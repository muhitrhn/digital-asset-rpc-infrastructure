use digital_asset_types::dao::stake_accounts;
use sea_orm_migration::{
    prelude::*,
    sea_orm::{ConnectionTrait, DatabaseBackend, Statement},
};
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(
                sea_query::Index::drop()
                    .name("sa_slot_updated_idx")
                    .table(stake_accounts::Entity)
                    .to_owned(),
            )
            .await?;

        manager
            .get_connection()
            .execute(Statement::from_string(
                DatabaseBackend::Postgres,
                "ALTER TABLE stake_accounts SET (fillfactor = 70);".to_string(),
            ))
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute(Statement::from_string(
                DatabaseBackend::Postgres,
                "ALTER TABLE stake_accounts SET (fillfactor = 90);".to_string(),
            ))
            .await?;

        manager
            .create_index(
                sea_query::Index::create()
                    .name("sa_slot_updated_idx")
                    .index_type(sea_query::IndexType::BTree)
                    .table(stake_accounts::Entity)
                    .to_owned(),
            )
            .await?;
        Ok(())
    }
}

/// Learn more at https://docs.rs/sea-query#iden
#[derive(Iden)]
enum Index {
    BRIN,
}
