use crate::{error::IngesterError, metric, tasks::TaskData};
use blockbuster::programs::token_account::TokenProgramAccount;
use cadence_macros::{is_global_default_set, statsd_count};
use digital_asset_types::dao::{asset, token_accounts, tokens, stake_accounts};
use plerkle_serialization::AccountInfo;
use sea_orm::{
    entity::*, query::*, sea_query::OnConflict, ActiveValue::Set, ConnectionTrait,
    DatabaseConnection, DbBackend, EntityTrait,
};
use solana_sdk::program_option::COption;
use spl_token::state::AccountState;
use tokio::sync::mpsc::UnboundedSender;

pub async fn handle_token_program_account<'a, 'b, 'c>(
    account_update: &'a AccountInfo<'a>,
    parsing_result: &'b TokenProgramAccount,
    db: &'c DatabaseConnection,
    _task_manager: &UnboundedSender<TaskData>,
) -> Result<(), IngesterError> {
    let key = *account_update.pubkey().unwrap();
    let key_bytes = key.0.to_vec();
    let spl_token_program = account_update.owner().unwrap().0.to_vec();
    match &parsing_result {
        TokenProgramAccount::TokenAccount(ta) => {
            let mint = ta.mint.to_bytes().to_vec();
            let delegate: Option<Vec<u8>> = match ta.delegate {
                COption::Some(d) => Some(d.to_bytes().to_vec()),
                COption::None => None,
            };
            let frozen = match ta.state {
                AccountState::Frozen => true,
                _ => false,
            };
            let mut owner = ta.owner.to_bytes().to_vec();

            if !ta.owner.is_on_curve() {
                let stake_account: Option<stake_accounts::Model> = stake_accounts::Entity::find_by_id(mint.clone())
                    .filter(stake_accounts::Column::Pubkey.eq(owner.clone()))
                    .one(db)
                    .await?;

                if stake_account.is_some() {
                    owner = stake_account.unwrap().authority;
                }
            }

            let model = token_accounts::ActiveModel {
                pubkey: Set(key_bytes),
                mint: Set(mint.clone()),
                delegate: Set(delegate.clone()),
                owner: Set(owner.clone()),
                frozen: Set(frozen),
                delegated_amount: Set(ta.delegated_amount as i64),
                token_program: Set(spl_token_program),
                slot_updated: Set(account_update.slot() as i64),
                amount: Set(ta.amount as i64),
                close_authority: Set(None),
            };

            let mut query = token_accounts::Entity::insert(model)
                .on_conflict(
                    OnConflict::columns([token_accounts::Column::Pubkey])
                        .update_columns([
                            token_accounts::Column::Mint,
                            token_accounts::Column::DelegatedAmount,
                            token_accounts::Column::Delegate,
                            token_accounts::Column::Amount,
                            token_accounts::Column::Frozen,
                            token_accounts::Column::TokenProgram,
                            token_accounts::Column::Owner,
                            token_accounts::Column::CloseAuthority,
                            token_accounts::Column::SlotUpdated,
                        ])
                        .to_owned(),
                )
                .build(DbBackend::Postgres);
            query.sql = format!(
                "{} WHERE excluded.slot_updated > token_accounts.slot_updated",
                query.sql
            );
            db.execute(query).await?;

            // Metrics
            let mut token_owner_update = false;
            let mut token_delegate_update = false;
            let mut token_freeze_update = false;

            let txn = db.begin().await?;
            let asset_update = asset::Entity::find_by_id(mint)
                .filter(asset::Column::OwnerType.eq("single"))
                .one(&txn)
                .await?;

            if let Some(asset) = asset_update {
                // Only handle token account updates for NFTs (supply=1)
                // TODO: Support fungible tokens
                let asset_clone = asset.clone();
                if asset_clone.supply == 1 {
                    let mut save_required = false;
                    let mut active: asset::ActiveModel = asset.into();

                    // Handle ownership updates
                    let old_owner = asset_clone.owner.clone();
                    let new_owner = owner.clone();
                    if ta.amount > 0 && Some(new_owner) != old_owner {
                        active.owner = Set(Some(owner.clone()));
                        token_owner_update = true;
                        save_required = true;
                    }

                    // Handle delegate updates
                    if ta.amount > 0 && delegate.clone() != asset_clone.delegate {
                        active.delegate = Set(delegate.clone());
                        token_delegate_update = true;
                        save_required = true;
                    }

                    // Handle freeze updates
                    if ta.amount > 0 && frozen != asset_clone.frozen {
                        active.frozen = Set(frozen);
                        token_freeze_update = true;
                        save_required = true;
                    }

                    if save_required {
                        active.save(&txn).await?;
                    }
                }
            }
            txn.commit().await?;

            // Publish metrics outside of the txn to reduce txn latency.
            if token_owner_update {
                metric! {
                    statsd_count!("token_account.owner_update", 1);
                }
            }
            if token_delegate_update {
                metric! {
                    statsd_count!("token_account.delegate_update", 1);
                }
            }
            if token_freeze_update {
                metric! {
                    statsd_count!("token_account.freeze_update", 1);
                }
            }

            Ok(())
        }
        TokenProgramAccount::Mint(m) => {
            let freeze_auth: Option<Vec<u8>> = match m.freeze_authority {
                COption::Some(d) => Some(d.to_bytes().to_vec()),
                COption::None => None,
            };
            let mint_auth: Option<Vec<u8>> = match m.mint_authority {
                COption::Some(d) => Some(d.to_bytes().to_vec()),
                COption::None => None,
            };
            let model = tokens::ActiveModel {
                mint: Set(key_bytes.clone()),
                token_program: Set(spl_token_program),
                slot_updated: Set(account_update.slot() as i64),
                supply: Set(m.supply as i64),
                decimals: Set(m.decimals as i32),
                close_authority: Set(None),
                extension_data: Set(None),
                mint_authority: Set(mint_auth),
                freeze_authority: Set(freeze_auth),
            };

            let mut query = tokens::Entity::insert(model)
                .on_conflict(
                    OnConflict::columns([tokens::Column::Mint])
                        .update_columns([
                            tokens::Column::Supply,
                            tokens::Column::TokenProgram,
                            tokens::Column::MintAuthority,
                            tokens::Column::CloseAuthority,
                            tokens::Column::ExtensionData,
                            tokens::Column::SlotUpdated,
                            tokens::Column::Decimals,
                            tokens::Column::FreezeAuthority,
                        ])
                        .to_owned(),
                )
                .build(DbBackend::Postgres);
            query.sql = format!(
                "{} WHERE excluded.slot_updated > tokens.slot_updated",
                query.sql
            );
            db.execute(query).await?;
            let asset_update: Option<asset::Model> = asset::Entity::find_by_id(key_bytes.clone())
                .filter(asset::Column::OwnerType.eq("single"))
                .one(db)
                .await?;
            if let Some(asset) = asset_update {
                let mut active: asset::ActiveModel = asset.into();
                active.supply = Set(m.supply as i64);
                active.supply_mint = Set(Some(key_bytes));
                active.save(db).await?;
            }
            Ok(())
        }
        _ => Err(IngesterError::NotImplemented),
    }?;
    Ok(())
}
