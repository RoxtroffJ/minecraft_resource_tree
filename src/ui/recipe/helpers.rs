use iced::{widget::{column, horizontal_rule, row, vertical_rule}, Element};

use crate::ui::{title_text, TitleLevel, SPACE};

/// Builds the general layout of a recipe widget
pub fn layout<'a, Message: 'a>(
    ingredient_iter: impl Iterator<Item = Element<'a, Message>>,
    product_iter: impl Iterator<Item = Element<'a, Message>>,
) -> Element<'a, Message> {
    let ingredient_col = column![
        title_text(TitleLevel::SectionTitle, "Ingredients"),
        horizontal_rule(SPACE),

        column(ingredient_iter)
    ];

    let products_col = column![
        title_text(TitleLevel::SectionTitle, "Products"),
        horizontal_rule(SPACE),

        column(product_iter)
    ];

    row![ingredient_col, vertical_rule(SPACE), products_col].into()
}
