//! Helpers for the [`Builder`](super::Builder).

use std::{fmt::Display, iter, num::ParseIntError, ops::Deref, str::FromStr, sync::LazyLock};

use iced::{
    Alignment, Element,
    Length::*,
    widget::{horizontal_space, row},
};
use serde::{Deserialize, Serialize};

use crate::ui::{recipe::BuilderAction, Item, SPACE};

static EMPTY_ITEM: LazyLock<Item> = LazyLock::new(|| Item::new(""));

pub(crate) fn recipe_column_iter<'a, T, U: Copy, V: Copy + 'a>(
    vec: &'a Vec<T>,
    item_sep: impl Fn(&'a T) -> (&'a Item, U) + 'a,
    parsed_deref: impl Fn(U) -> V,
    empty_content: U,
    del_row: &'a impl Fn(usize) -> BuilderAction,
    edit_item: &'a impl Fn(usize, Item) -> BuilderAction,
    add_row: &'a impl Fn(Item, V) -> BuilderAction,
    row_builder: impl Fn(usize, bool, U) -> Element<'a, BuilderAction>,
) -> impl Iterator<Item = Element<'a, BuilderAction>> {
    vec.iter()
        .map(move |x| (false, item_sep(x)))
        .chain(iter::once((true, (EMPTY_ITEM.deref(), empty_content))))
        .enumerate()
        .map(move |(index, (last, (item, x)))| {
            let y = parsed_deref(x);
            row![
                item.builder(move |i| {
                    if i.get_name().is_empty() {
                        del_row(index)
                    } else if !last {
                        edit_item(index, i)
                    } else {
                        add_row(i, y)
                    }
                }, Some(BuilderAction::Sumbit)),
                horizontal_space().width(Fixed(SPACE as _)),
                row_builder(index, last, x)
            ]
            .align_y(Alignment::Center)
            .into()
        })
}

/// An item quatity.
/// It's a positive integer that won't accept 0 when parsed.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Quantity {
    n: u8,
}

impl Quantity {
    /// Creates a new quantity from a number if non zero.
    pub fn new(n: u8) -> Option<Self> {
        if n == 0 { None } else { Some(Quantity { n }) }
    }

    /// Consumes the quantity and returns it's number.
    pub fn take(self) -> u8 {
        self.n
    }
}

impl Default for Quantity {
    fn default() -> Self {
        Quantity { n: 1 }
    }
}

impl Display for Quantity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.deref().fmt(f)
    }
}

impl FromStr for Quantity {
    type Err = ParseQuantityError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.parse() {
            Ok(qty) => Quantity::new(qty).ok_or(ParseQuantityError::Zero),
            Err(error) => Err(ParseQuantityError::Parse(error)),
        }
    }
}

impl Deref for Quantity {
    type Target = u8;

    fn deref(&self) -> &Self::Target {
        &self.n
    }
}

#[derive(Debug, Clone)]
/// Non zero [`ParseIntError`].
pub enum ParseQuantityError {
    /// Standard [`ParseIntError`]
    Parse(ParseIntError),
    /// Got 0
    Zero,
}

impl Display for ParseQuantityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseQuantityError::Parse(parse_int_error) => parse_int_error.fmt(f),
            ParseQuantityError::Zero => write!(f, "Can't be zero."),
        }
    }
}

/// A probability.
/// It's an integer between 0 and 100 (%).
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Probability {
    p: u8,
}

impl Probability {
    /// Creates a new probability, if the given int is a valid strictly positive percentage.
    pub fn new(p: u8) -> Option<Self> {
        if p <= 0 || p > 100 {
            None
        } else {
            Some(Self { p })
        }
    }
}

impl Default for Probability {
    fn default() -> Self {
        Self { p: 100 }
    }
}

impl Display for Probability {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.p.fmt(f)
    }
}

impl FromStr for Probability {
    type Err = ParseProbaError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.parse() {
            Ok(p) => Self::new(p).ok_or(ParseProbaError::Range),
            Err(err) => Err(ParseProbaError::Parse(err)),
        }
    }
}

impl Deref for Probability {
    type Target = u8;

    fn deref(&self) -> &Self::Target {
        &self.p
    }
}

/// A proba parse error.
#[derive(Debug, Clone)]
pub enum ParseProbaError {
    /// Standard [`ParseIntError`].
    Parse(ParseIntError),
    /// Not a non zero percentage
    Range,
}

impl Display for ParseProbaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseProbaError::Parse(parse_int_error) => parse_int_error.fmt(f),
            ParseProbaError::Range => write!(f, "Value has to be between 1 and 100"),
        }
    }
}
