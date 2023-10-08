use crate::{error::IngesterError, tasks::TaskData};
use blockbuster::programs::stake_account::{StakeProgramAccount};
use digital_asset_types::dao::{asset, stake_accounts};
use plerkle_serialization::{AccountInfo};
use sea_orm::{
    query::*, sea_query::OnConflict, ActiveValue::Set, ConnectionTrait,
    DatabaseConnection, DbBackend, EntityTrait, ColumnTrait, ActiveModelTrait,
};
use tokio::sync::mpsc::UnboundedSender;

pub async fn handle_stake_program_account<'a, 'b, 'c>(
    account_update: &'a AccountInfo<'a>,
    parsing_result: &'b StakeProgramAccount,
    db: &'c DatabaseConnection,
    _task_manager: &UnboundedSender<TaskData>,
) -> Result<(), IngesterError> {
    let owner_program = account_update.owner().unwrap().0.to_vec();

    match &parsing_result {
        StakeProgramAccount::SacStakeAccount(sa) => {
            let authority = sa.authority.to_bytes().to_vec();
            let token = sa.token.to_bytes().to_vec();
            let model = stake_accounts::ActiveModel {
                token: Set(token.clone()),
                authority: Set(authority.clone()),
                owner_program: Set(owner_program),
                slot_updated: Set(account_update.slot() as i64)
            };

            let mut query = stake_accounts::Entity::insert(model)
                .on_conflict(
                    OnConflict::columns([stake_accounts::Column::Token])
                        .update_columns([
                            stake_accounts::Column::Token,
                            stake_accounts::Column::Authority,
                            stake_accounts::Column::OwnerProgram,
                            stake_accounts::Column::SlotUpdated,
                        ])
                        .to_owned(),
                )
                .build(DbBackend::Postgres);
            query.sql = format!(
                "{} WHERE excluded.slot_updated > stake_accounts.slot_updated",
                query.sql
            );
            db.execute(query).await?;
            let txn = db.begin().await?;
            let asset_update: Option<asset::Model> = asset::Entity::find_by_id(token)
                .filter(asset::Column::OwnerType.eq("single"))
                .one(&txn)
                .await?;
            if let Some(asset) = asset_update {
                let mut active: asset::ActiveModel = asset.into();
                active.owner = Set(Some(authority));
                active.save(&txn).await?;
            }
            txn.commit().await?;
            Ok(())
        }
        _ => Err(IngesterError::NotImplemented),
    }?;
    Ok(())
}
