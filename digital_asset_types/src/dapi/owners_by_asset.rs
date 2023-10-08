use crate::dao::scopes;
use crate::rpc::filter::OwnerSorting;
use crate::rpc::response::OwnerList;
use sea_orm::DatabaseConnection;
use sea_orm::DbErr;

use super::common::{build_owner_response, create_pagination, create_sorting_for_ta as create_sorting};

pub async fn get_owners_by_asset(
    db: &DatabaseConnection,
    asset_id: Vec<u8>,
    sort_by: OwnerSorting,
    limit: u64,
    page: Option<u64>,
    before: Option<Vec<u8>>,
    after: Option<Vec<u8>>,
    enable_grand_total_query: bool,
) -> Result<OwnerList, DbErr> {
    let pagination = create_pagination(before, after, page)?;
    let (sort_direction, sort_column) = create_sorting(sort_by);
    let (assets, grand_total) = scopes::owner::get_owners_by_asset(
        db,
        asset_id,
        sort_column,
        sort_direction,
        &pagination,
        limit,
        enable_grand_total_query,
    )
    .await?;
    Ok(build_owner_response(
        assets,
        limit,
        grand_total,
        &pagination,
    ))
}
