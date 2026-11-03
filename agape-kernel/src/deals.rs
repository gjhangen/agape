//! What deals are paid in, and what a party offers to do. A deal itself is an Agreement on one of
//! these offers (see `exchange`); nothing about it lives here.

use crate::value::Unit;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Pay { Points, Money }

/// Something a party offers to do, at a stated price. Each becomes an offer on their page.
#[derive(Clone, Debug, Serialize)]
pub struct Rate { pub kind: String, pub title: String, pub blurb: String, pub pay: Pay, pub price: i64,
    /// How many they'll take tonight (None: no limit).
    pub limit: Option<i64> }

pub fn unit_of(p: &Pay) -> Unit { match p { Pay::Points => crate::social::points(), Pay::Money => Unit::usd() } }
