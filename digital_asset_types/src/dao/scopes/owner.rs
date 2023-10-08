use crate::{
    dao::{
        token_accounts::{self, Entity}, full_owner::FullOwnership, Pagination
    }
};

use indexmap::IndexMap;
use sea_orm::{entity::*, query::*, ConnectionTrait, DbErr, Order};
use tokio::try_join;

pub fn paginate<'db, T>(pagination: &Pagination, limit: u64, stmt: T) -> T
where
    T: QueryFilter + QuerySelect,
{
    let mut stmt = stmt;
    match pagination {
        Pagination::Keyset { before, after } => {
            if let Some(b) = before {
                stmt = stmt.filter(token_accounts::Column::Mint.lt(b.clone()));
            }
            if let Some(a) = after {
                stmt = stmt.filter(token_accounts::Column::Mint.gt(a.clone()));
            }
        }
        Pagination::Page { page } => {
            if *page > 0 {
                stmt = stmt.offset((page - 1) * limit)
            }
        }
    }
    stmt.limit(limit)
}

pub async fn get_owners_by_asset(
    conn: &impl ConnectionTrait,
    asset_id: Vec<u8>,
    sort_by: Option<token_accounts::Column>,
    sort_direction: Order,
    pagination: &Pagination,
    limit: u64,
    enable_grand_total_query: bool,
) -> Result<(Vec<FullOwnership>, Option<u64>), DbErr> {
    let cond = Condition::all()
        .add(token_accounts::Column::Mint.eq(asset_id))
        .add(token_accounts::Column::Amount.gt(0));
    get_owners_by_condition(
        conn,
        cond,
        vec![],
        sort_by,
        sort_direction,
        pagination,
        limit,
        enable_grand_total_query,
    )
    .await
}

pub async fn get_owners_by_condition(
    conn: &impl ConnectionTrait,
    condition: Condition,
    joins: Vec<RelationDef>,
    sort_by: Option<token_accounts::Column>,
    sort_direction: Order,
    pagination: &Pagination,
    limit: u64,
    enable_grand_total_query: bool,
) -> Result<(Vec<FullOwnership>, Option<u64>), DbErr> {
    let mut stmt = token_accounts::Entity::find();
    for def in joins {
        stmt = stmt.join(JoinType::LeftJoin, def);
    }
    stmt = stmt.filter(condition);
    if let Some(col) = sort_by {
        stmt = stmt
            .order_by(col, sort_direction.clone())
            .order_by(token_accounts::Column::Mint, sort_direction);
    }

    let (assets, grand_total) =
        get_full_response(conn, stmt, pagination, limit, enable_grand_total_query).await?;
    Ok((assets, grand_total))
}

async fn get_full_response(
    conn: &impl ConnectionTrait,
    stmt: Select<Entity>,
    pagination: &Pagination,
    limit: u64,
    enable_grand_total_query: bool,
) -> Result<(Vec<FullOwnership>, Option<u64>), DbErr> {
    if enable_grand_total_query {
        let grand_total_task = get_grand_total(conn, stmt.clone());
        let asset_owners_task = paginate(pagination, limit, stmt).all(conn);

        let (asset_owners, grand_total) = try_join!(asset_owners_task, grand_total_task)?;
        let asset_owners = to_full_ownership(asset_owners)?;
        return Ok((asset_owners, grand_total));
    } else {
        let owners = paginate(pagination, limit, stmt).all(conn).await?;
        let asset_owners = to_full_ownership(owners)?;
        Ok((asset_owners, None))
    }
}

fn to_full_ownership(
    owners: Vec<token_accounts::Model>
) -> Result<Vec<FullOwnership>, DbErr> {
    let owners_map = owners.into_iter().fold(IndexMap::new(), |mut acc, owner| {
        let id = owner.owner.clone();
        let fo = FullOwnership {
            token_account: owner
        };
        acc.insert(id, fo);
        acc
    });

    Ok(owners_map.into_iter().map(|(_, v)| v).collect())
}

async fn get_grand_total(
    conn: &impl ConnectionTrait,
    stmt: Select<Entity>,
) -> Result<Option<u64>, DbErr> {
    let grand_total = stmt.count(conn).await?;
    Ok(Some(grand_total))
}
