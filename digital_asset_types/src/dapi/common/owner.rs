use crate::dao::FullOwnership;
use crate::dao::Pagination;
use crate::dao::{token_accounts};
use crate::rpc::filter::{OwnerSortBy, OwnerSortDirection, OwnerSorting};
use crate::rpc::response::{OwnerError, OwnerList};
use crate::rpc::{
    TokenOwnership
};

use sea_orm::DbErr;

pub fn build_owner_response(
    owners: Vec<FullOwnership>,
    limit: u64,
    grand_total: Option<u64>,
    pagination: &Pagination,
) -> OwnerList {
    let total = owners.len() as u32;
    let (page, before, after) = match pagination {
        Pagination::Keyset { before, after } => {
            let bef = before.clone().and_then(|x| String::from_utf8(x).ok());
            let aft = after.clone().and_then(|x| String::from_utf8(x).ok());
            (None, bef, aft)
        }
        Pagination::Page { page } => (Some(*page), None, None),
    };
    let (items, errors) = owner_list_to_rpc(owners);
    OwnerList {
        // grand_total: grand_total.unwrap(), // TODO
        total,
        limit: limit as u32,
        page: page.map(|x| x as u32),
        before,
        after,
        items,
        errors,
    }
}

pub fn create_sorting_for_ta(sorting: OwnerSorting) -> (sea_orm::query::Order, Option<token_accounts::Column>) {
    let sort_column = match sorting.sort_by {
        OwnerSortBy::Updated => Some(token_accounts::Column::SlotUpdated),
        OwnerSortBy::RecentAction => Some(token_accounts::Column::SlotUpdated),
        OwnerSortBy::None => None,
    };
    let sort_direction = match sorting.sort_direction.unwrap_or_default() {
        OwnerSortDirection::Desc => sea_orm::query::Order::Desc,
        OwnerSortDirection::Asc => sea_orm::query::Order::Asc,
    };
    (sort_direction, sort_column)
}

//TODO -> impl custom error type
pub fn owner_to_rpc(
    owner: FullOwnership,
    raw_data: Option<bool>,
) -> Result<TokenOwnership, DbErr> {
    let FullOwnership {
        token_account
    } = owner;
    Ok(TokenOwnership {
        mint: bs58::encode(token_account.mint).into_string(),
        frozen: token_account.frozen,
        delegated: token_account.delegate.is_some(),
        delegate: token_account.delegate.map(|s| bs58::encode(s).into_string()),
        owner: bs58::encode(token_account.owner).into_string(),
        amount: token_account.amount as u64,
    })
}

pub fn owner_list_to_rpc(
    owner_list: Vec<FullOwnership>,
) -> (Vec<TokenOwnership>, Vec<OwnerError>) {
    owner_list
        .into_iter()
        .fold((vec![], vec![]), |(mut owners, mut errors), owner| {
            let owner_str = bs58::encode(owner.token_account.owner.clone()).into_string();
            match owner_to_rpc(owner, None) {
                Ok(rpc_owner) => owners.push(rpc_owner),
                Err(e) => errors.push(OwnerError {
                    id: owner_str,
                    error: e.to_string(),
                }),
            }
            (owners, errors)
        })
}
