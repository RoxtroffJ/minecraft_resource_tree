//! Everything to do with [`RecipeWidget`].

use iced::{
    Element,
    Length::{self, Shrink},
    widget::{button, column, row, text},
};

use crate::{
    recipes::Recipe,
    ui::{Item, SPACE, TitleLevel, recipe::layout_helpers::layout, title_text},
};

/// A widget that displays a recipe
pub struct RecipeWidget<'a, Message> {
    recipe: &'a Recipe<Item>,
    on_edit: Option<Message>,
    height: Length,
}

impl<'a, Message> RecipeWidget<'a, Message> {
    /// Builds a new [`RecipeWidget`]
    pub fn new(recipe: &'a Recipe<Item>) -> Self {
        Self {
            recipe,
            on_edit: None,
            height: Shrink,
        }
    }

    /// Adds an Edit button that sends a message.
    pub fn on_edit(mut self, on_edit: Message) -> Self {
        self.on_edit = Some(on_edit);
        self
    }
}

impl<'a, Message: Clone + 'a> From<RecipeWidget<'a, Message>> for Element<'a, Message> {
    fn from(value: RecipeWidget<'a, Message>) -> Self {
        let ingredients = value
            .recipe
            .get_ingredients()
            .iter()
            .map(|(item, quantity)| {
                row![item.displayer(), text(quantity.to_string())]
                    .spacing(SPACE)
                    .into()
            });
        let products = value
            .recipe
            .get_products()
            .iter()
            .map(|(item, quantity, proba)| {
                row![
                    item.displayer(),
                    text!("{quantity} ({:.0}%)", proba * 100.0)
                ]
                .spacing(SPACE)
                .into()
            });

        let mut content = column![layout(ingredients, products, value.height)];

        if let Some(on_edit) = value.on_edit {
            content =
                content.push(button(title_text(TitleLevel::SectionTitle, "Edit")).on_press(on_edit))
        }

        content.into()
    }
}
