use digital_asset_types::{
    dao::{
        scopes::asset::get_grouping,
        sea_orm_active_enums::{
            OwnerType, RoyaltyTargetType, SpecificationAssetClass, SpecificationVersions,
        },
        SearchAssetsQuery,
    },
    dapi::{
        get_asset, get_assets_by_authority, get_assets_by_creator, get_assets_by_group,
        get_assets_by_owner, get_multiple_by_asset, get_proof_for_asset, get_signatures_for_asset, search_assets,
    },
    rpc::{
        filter::{AssetSortBy, SearchConditionType},
        response::GetGroupingResponse,
        transform::AssetTransform,
    },
    rpc::{OwnershipModel, RoyaltyModel},
};
use open_rpc_derive::document_rpc;
use sea_orm::{sea_query::ConditionType, ConnectionTrait, DbBackend, Statement};

use crate::{
    feature_flag::{get_feature_flags, FeatureFlags},
    validation::validate_opt_pubkey,
};
use open_rpc_schema::document::OpenrpcDocument;
use {
    crate::api::*,
    crate::config::Config,
    crate::validation::validate_pubkey,
    crate::DasApiError,
    async_trait::async_trait,
    digital_asset_types::rpc::{
        response::AssetList, response::TransactionSignatureList, Asset, AssetProof,
    },
    sea_orm::{DatabaseConnection, DbErr, SqlxPostgresConnector},
    sqlx::postgres::PgPoolOptions,
};

pub struct DasApi {
    db_connection: DatabaseConnection,
    cdn_prefix: Option<String>,
    feature_flags: FeatureFlags,
}

impl DasApi {
    pub async fn from_config(config: Config) -> Result<Self, DasApiError> {
        let pool = PgPoolOptions::new()
            .max_connections(250)
            .connect(&config.database_url)
            .await?;
        let feature_flags = get_feature_flags(&config);
        let conn = SqlxPostgresConnector::from_sqlx_postgres_pool(pool);
        Ok(DasApi {
            db_connection: conn,
            cdn_prefix: config.cdn_prefix,
            feature_flags,
        })
    }

    fn validate_pagination(
        &self,
        limit: &Option<u32>,
        page: &Option<u32>,
        before: &Option<String>,
        after: &Option<String>,
    ) -> Result<(), DasApiError> {
        if page.is_none() && before.is_none() && after.is_none() {
            return Err(DasApiError::PaginationEmptyError);
        }

        if let Some(limit) = limit {
            // make config item
            if *limit > 1000 {
                return Err(DasApiError::PaginationError);
            }
        }

        if let Some(page) = page {
            if *page == 0 {
                return Err(DasApiError::PaginationEmptyError);
            }

            // make config item
            if before.is_some() || after.is_some() {
                return Err(DasApiError::PaginationError);
            }
        }

        if let Some(before) = before {
            validate_pubkey(before.clone())?;
        }

        if let Some(after) = after {
            validate_pubkey(after.clone())?;
        }

        Ok(())
    }

    fn validate_sorting_for_collection(
        &self,
        group: &String,
        collection: &String,
        sort_by: &Option<AssetSorting>,
    ) -> Result<(), DasApiError> {
        // List of collections which contain more than 300k nfts
        let collections: [&str; 17] = [
            "DRiP2Pn2K6fuMLKQmt5rZWyHiUZ6WK3GChEySUpHSS4x",
            "DGPTxgKaBPJv3Ng7dc9AFDpX6E7kgUMZEgyTm3VGWPW6",
            "VLT1ERWF2SQ51ybTGAuSBDWFZCxYth8ox6faJG9WrmG",
            "8drYRaD7csLTEqX89hyM1XpTmXkQh4Evr1xQue2XkdB5",
            "FLRxZJb7Kpd5i9Q7WdH7r5uRqDL7oJVpqW3ew8FpE336",
            "DAA1jBEYj2w4DgMRDVaXg5CWfjmTry5t8VEvLJQ9R8PY",
            "tinyVrmxcEUyVufgmFzGYe7C4mrGXDC21uLJAGVKXkg",
            "BoRKkxKPoAt7LcyVRPa9ZZT5MztkJuc4PiGrUXAgDHPH",
            "2bJpbZ5VNp48LpTh2DSwiuo6gJsTrh59TjcsAfRCLNXZ",
            "BZ3DohF6BHGkAnZAe1g8ohWVuh95bXT4FhiGw1BXJWfF",
            "MAQNiWAYh5yGCQKeWFzHLypThEjfTJfBQxwiF8P5Vax",
            "AMSNskm2RZqPXCZ6P2z6JLyHWMQF6pQ8RA8Q6x42Xufq",
            "F8FdDYD3PWndYoae9TrBcucXDWFwDvm6bZU2LQT1PwyB",
            "DASHYFhWiCoe8PNCHZJAjmvGBBj8SLtkvW2uYV2e3FrV",
            "BTDX3HWvRv16j4KUUbdegP3oazyVGCLxJpFqSQZ2bH6n",
            "8tWwfmudVrrRzACvtt18H5vHVxsYofMyeGt7L3LFPSqC",
            "WoMbiTtXKwUtf4wosoffv45khVF8yA2mPkinGosCFQ4",
        ];

        if group == "collection" && collections.contains(&collection.as_str()) {
            if sort_by.clone().map(|s| s.sort_by) == Some(AssetSortBy::None) {
                return Ok(());
            } else {
                return Err(DasApiError::ValidationError(
                    format!("Sorting is not supported for collection {}. Please set 'sortBy' to 'none' to disable sorting.", collection),
                ));
            }
        }
        Ok(())
    }
}

pub fn not_found(asset_id: &String) -> DbErr {
    DbErr::RecordNotFound(format!("Asset Proof for {} Not Found", asset_id))
}

#[document_rpc]
#[async_trait]
impl ApiContract for DasApi {
    async fn check_health(self: &DasApi) -> Result<(), DasApiError> {
        self.db_connection
            .execute(Statement::from_string(
                DbBackend::Postgres,
                "SELECT 1".to_string(),
            ))
            .await?;
        Ok(())
    }

    async fn get_asset_proof(
        self: &DasApi,
        payload: GetAssetProof,
    ) -> Result<AssetProof, DasApiError> {
        let id = validate_pubkey(payload.id.clone())?;
        let id_bytes = id.to_bytes().to_vec();
        get_proof_for_asset(&self.db_connection, id_bytes)
            .await
            .and_then(|p| {
                if p.proof.is_empty() {
                    return Err(not_found(&payload.id));
                }
                Ok(p)
            })
            .map_err(Into::into)
    }

    async fn get_asset(self: &DasApi, payload: GetAsset) -> Result<Asset, DasApiError> {
        let id = validate_pubkey(payload.id.clone())?;
        let id_bytes = id.to_bytes().to_vec();
        let transform = AssetTransform {
            cdn_prefix: self.cdn_prefix.clone(),
        };
        get_asset(&self.db_connection, id_bytes, &transform, payload.raw_data)
            .await
            .map_err(Into::into)
    }

    async fn get_assets_by_owner(
        self: &DasApi,
        payload: GetAssetsByOwner,
    ) -> Result<AssetList, DasApiError> {
        let GetAssetsByOwner {
            owner_address,
            sort_by,
            limit,
            page,
            before,
            after,
        } = payload;
        let before: Option<String> = before.filter(|before| !before.is_empty());
        let after: Option<String> = after.filter(|after| !after.is_empty());
        let owner_address = validate_pubkey(owner_address.clone())?;
        let owner_address_bytes = owner_address.to_bytes().to_vec();
        let sort_by = sort_by.unwrap_or_default();
        self.validate_pagination(&limit, &page, &before, &after)?;
        let transform = AssetTransform {
            cdn_prefix: self.cdn_prefix.clone(),
        };
        get_assets_by_owner(
            &self.db_connection,
            owner_address_bytes,
            sort_by,
            limit.map(|x| x as u64).unwrap_or(1000),
            page.map(|x| x as u64),
            before.map(|x| bs58::decode(x).into_vec().unwrap_or_default()),
            after.map(|x| bs58::decode(x).into_vec().unwrap_or_default()),
            &transform,
            self.feature_flags.enable_grand_total_query,
        )
        .await
        .map_err(Into::into)
    }

    async fn get_multiple_by_asset(
        self: &DasApi,
        payload: GetMultipleByAsset,
    ) -> Result<AssetList, DasApiError> {
        let GetMultipleByAsset {
            id,
            sort_by,
            limit,
            page,
            before,
            after,
        } = payload;
        let before: Option<String> = before.filter(|before| !before.is_empty());
        let after: Option<String> = after.filter(|after| !after.is_empty());
        let asset_id = validate_pubkey(id.clone())?;
        let asset_id_bytes = asset_id.to_bytes().to_vec();
        let sort_by = sort_by.unwrap_or_default();
        self.validate_pagination(&limit, &page, &before, &after)?;
        let transform = AssetTransform {
            cdn_prefix: self.cdn_prefix.clone(),
        };
        get_multiple_by_asset(
            &self.db_connection,
            asset_id_bytes,
            sort_by,
            limit.map(|x| x as u64).unwrap_or(5000),
            page.map(|x| x as u64),
            before.map(|x| bs58::decode(x).into_vec().unwrap_or_default()),
            after.map(|x| bs58::decode(x).into_vec().unwrap_or_default()),
            &transform,
            self.feature_flags.enable_grand_total_query,
        )
        .await
        .map_err(Into::into)
    }

    async fn get_assets_by_group(
        self: &DasApi,
        payload: GetAssetsByGroup,
    ) -> Result<AssetList, DasApiError> {
        let GetAssetsByGroup {
            group_key,
            group_value,
            sort_by,
            limit,
            page,
            before,
            after,
        } = payload;
        self.validate_sorting_for_collection(&group_key, &group_value, &sort_by)?;
        let sort_by = sort_by.unwrap_or_default();
        let before: Option<String> = before.filter(|before| !before.is_empty());
        let after: Option<String> = after.filter(|after| !after.is_empty());
        self.validate_pagination(&limit, &page, &before, &after)?;
        let transform = AssetTransform {
            cdn_prefix: self.cdn_prefix.clone(),
        };
        get_assets_by_group(
            &self.db_connection,
            group_key,
            group_value,
            sort_by,
            limit.map(|x| x as u64).unwrap_or(1000),
            page.map(|x| x as u64),
            before.map(|x| bs58::decode(x).into_vec().unwrap_or_default()),
            after.map(|x| bs58::decode(x).into_vec().unwrap_or_default()),
            &transform,
            self.feature_flags.enable_grand_total_query,
        )
        .await
        .map_err(Into::into)
    }

    async fn get_assets_by_creator(
        self: &DasApi,
        payload: GetAssetsByCreator,
    ) -> Result<AssetList, DasApiError> {
        let GetAssetsByCreator {
            creator_address,
            only_verified,
            sort_by,
            limit,
            page,
            before,
            after,
        } = payload;
        let creator_address = validate_pubkey(creator_address.clone())?;
        let creator_address_bytes = creator_address.to_bytes().to_vec();

        self.validate_pagination(&limit, &page, &before, &after)?;
        let sort_by = sort_by.unwrap_or_default();
        let only_verified = only_verified.unwrap_or_default();
        let transform = AssetTransform {
            cdn_prefix: self.cdn_prefix.clone(),
        };
        get_assets_by_creator(
            &self.db_connection,
            creator_address_bytes,
            only_verified,
            sort_by,
            limit.map(|x| x as u64).unwrap_or(1000),
            page.map(|x| x as u64),
            before.map(|x| bs58::decode(x).into_vec().unwrap_or_default()),
            after.map(|x| bs58::decode(x).into_vec().unwrap_or_default()),
            &transform,
            self.feature_flags.enable_grand_total_query,
        )
        .await
        .map_err(Into::into)
    }

    async fn get_assets_by_authority(
        self: &DasApi,
        payload: GetAssetsByAuthority,
    ) -> Result<AssetList, DasApiError> {
        let GetAssetsByAuthority {
            authority_address,
            sort_by,
            limit,
            page,
            before,
            after,
        } = payload;
        let sort_by = sort_by.unwrap_or_default();
        let authority_address = validate_pubkey(authority_address.clone())?;
        let authority_address_bytes = authority_address.to_bytes().to_vec();
        self.validate_pagination(&limit, &page, &before, &after)?;
        let transform = AssetTransform {
            cdn_prefix: self.cdn_prefix.clone(),
        };
        get_assets_by_authority(
            &self.db_connection,
            authority_address_bytes,
            sort_by,
            limit.map(|x| x as u64).unwrap_or(1000),
            page.map(|x| x as u64),
            before.map(|x| bs58::decode(x).into_vec().unwrap_or_default()),
            after.map(|x| bs58::decode(x).into_vec().unwrap_or_default()),
            &transform,
            self.feature_flags.enable_grand_total_query,
        )
        .await
        .map_err(Into::into)
    }

    async fn search_assets(&self, payload: SearchAssets) -> Result<AssetList, DasApiError> {
        let SearchAssets {
            negate,
            /// Defaults to [ConditionType,
            condition_type,
            interface,
            owner_address,
            owner_type,
            creator_address,
            creator_verified,
            authority_address,
            grouping,
            delegate,
            frozen,
            supply,
            supply_mint,
            compressed,
            compressible,
            royalty_target_type,
            royalty_target,
            royalty_amount,
            burnt,
            sort_by,
            limit,
            page,
            before,
            after,
            json_uri,
            show_collection_metadata,
        } = payload;
        // Deserialize search assets query
        self.validate_pagination(&limit, &page, &before, &after)?;
        let spec: Option<(SpecificationVersions, SpecificationAssetClass)> =
            interface.map(|x| x.into());
        let specification_version = spec.clone().map(|x| x.0);
        let specification_asset_class = spec.map(|x| x.1);
        let condition_type = condition_type.map(|x| match x {
            SearchConditionType::Any => ConditionType::Any,
            SearchConditionType::All => ConditionType::All,
        });
        let owner_address = validate_opt_pubkey(&owner_address)?;
        let creator_address = validate_opt_pubkey(&creator_address)?;
        let delegate = validate_opt_pubkey(&delegate)?;

        let authority_address = validate_opt_pubkey(&authority_address)?;
        let supply_mint = validate_opt_pubkey(&supply_mint)?;
        let royalty_target = validate_opt_pubkey(&royalty_target)?;

        let owner_type = owner_type.map(|x| match x {
            OwnershipModel::Single => OwnerType::Single,
            OwnershipModel::Token => OwnerType::Token,
        });
        let royalty_target_type = royalty_target_type.map(|x| match x {
            RoyaltyModel::Creators => RoyaltyTargetType::Creators,
            RoyaltyModel::Fanout => RoyaltyTargetType::Fanout,
            RoyaltyModel::Single => RoyaltyTargetType::Single,
        });
        let saq = SearchAssetsQuery {
            negate,
            condition_type,
            specification_version,
            specification_asset_class,
            owner_address,
            owner_type,
            creator_address,
            creator_verified,
            authority_address,
            grouping,
            delegate,
            frozen,
            supply,
            supply_mint,
            compressed,
            compressible,
            royalty_target_type,
            royalty_target,
            royalty_amount,
            burnt,
            json_uri,
        };
        let sort_by = sort_by.unwrap_or_default();
        let transform = AssetTransform {
            cdn_prefix: self.cdn_prefix.clone(),
        };
        // Execute query
        search_assets(
            &self.db_connection,
            saq,
            sort_by,
            limit.map(|x| x as u64).unwrap_or(1000),
            page.map(|x| x as u64),
            before.map(|x| bs58::decode(x).into_vec().unwrap_or_default()),
            after.map(|x| bs58::decode(x).into_vec().unwrap_or_default()),
            &transform,
            self.feature_flags.enable_grand_total_query,
            self.feature_flags.enable_collection_metadata
                && show_collection_metadata.unwrap_or(false),
        )
        .await
        .map_err(Into::into)
    }

    async fn get_grouping(
        self: &DasApi,
        payload: GetGrouping,
    ) -> Result<GetGroupingResponse, DasApiError> {
        let GetGrouping {
            group_key,
            group_value,
        } = payload;
        let gs = get_grouping(&self.db_connection, group_key.clone(), group_value.clone()).await?;
        Ok(GetGroupingResponse {
            group_key,
            group_name: group_value,
            group_size: gs.size,
        })
    }

    async fn get_signatures_for_asset(
        self: &DasApi,
        payload: GetSignaturesForAsset,
    ) -> Result<TransactionSignatureList, DasApiError> {
        let GetSignaturesForAsset {
            id,
            limit,
            page,
            before,
            after,
            tree,
            leaf_index,
        } = payload;

        if !((id.is_some() && tree.is_none() && leaf_index.is_none())
            || (id.is_none() && tree.is_some() && leaf_index.is_some()))
        {
            return Err(DasApiError::ValidationError(
                "Must provide either 'id' or both 'tree' and 'leafIndex'".to_string(),
            ));
        }
        let id = validate_opt_pubkey(&id)?;
        let tree = validate_opt_pubkey(&tree)?;

        self.validate_pagination(&limit, &page, &before, &after)?;

        get_signatures_for_asset(
            &self.db_connection,
            id,
            tree,
            leaf_index,
            limit.map(|x| x as u64).unwrap_or(1000),
            page.map(|x| x as u64),
            before.map(|x| bs58::decode(x).into_vec().unwrap_or_default()),
            after.map(|x| bs58::decode(x).into_vec().unwrap_or_default()),
        )
        .await
        .map_err(Into::into)
    }
}
