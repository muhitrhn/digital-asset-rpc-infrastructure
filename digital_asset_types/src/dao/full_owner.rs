use {
    crate::dao::token_accounts,
    serde::{Deserialize, Serialize}
};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct FullOwnership {
    pub token_account: token_accounts::Model
}
