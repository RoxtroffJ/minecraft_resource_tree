use crate::{
    recipe_lookup::Recipe,
    ui::{ERROR_COLOR, Item, SPACE, TitleLevel, recipe::helpers::layout, title_text},
};

use iced::{
    Alignment, Element,
    widget::{Button, button, column, horizontal_rule, row, text, text_input},
};
use more_iced_aw::{
    helpers::filter_background,
    parsed_input::{self, Parsed, ParsedInput, color_on_err},
};

mod helpers;
use helpers::*;

/// The state of a recipe builder widget.
#[derive(Debug, Default)]
pub struct BuilderState {
    products: Vec<(
        Item,
        parsed_input::Content<Quantity, ParseQuantityError>,
        parsed_input::Content<Probability, ParseProbaError>,
    )>,
    ingredients: Vec<(Item, parsed_input::Content<Quantity, ParseQuantityError>)>,
    empty_qty: parsed_input::Content<Quantity, ParseQuantityError>,
    empty_proba: parsed_input::Content<Probability, ParseProbaError>,
}

/// Actions that a [`BuilderState`] can perform.
///
/// For all the edits, if the index is invalid or probability isn't between 0 and 1,
/// nothing will happen when [`perform`](BuilderState::perform) will be called.
#[derive(Debug, Clone)]
pub enum BuilderAction {
    /// Adds a product with given item, quantity and probability.
    AddProduct(Item, Quantity, Probability),
    /// Adds an ingredient with given item and quantity.
    AddIngredient(Item, Quantity),
    /// Changes the item of the given production line.
    EditProdItem(usize, Item),
    /// Changes the quantity of items produced in the given production line.
    EditProdQty(usize, Parsed<Quantity, ParseQuantityError>),
    /// Changes the probability of success of the given production line. Proba must be between 0 and 1.
    EditProdProba(usize, Parsed<Probability, ParseProbaError>),
    /// Changes the item of the given ingredient line.
    EditIngrItem(usize, Item),
    /// Changes the quantity required of the item of the given ingredient line.
    EditIngrQty(usize, Parsed<Quantity, ParseQuantityError>),
    /// Deletes the given production line.
    DelProd(usize),
    /// Deletes the given ingredient line.
    DelIngr(usize),
}

impl BuilderState {
    /// Creates a new [`BuilderState`] that displays an empty recipe.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a [`BuilderState`] initialised with the given [`Recipe<Item>`].
    pub fn from_recipe(recipe: Recipe<Item>) -> Self {
        let (ingredients, products) = recipe.take();

        Self {
            products: products
                .into_iter()
                .map(|(item, qty, prob)| {
                    (
                        item,
                        parsed_input::Content::new(Quantity::new(qty).unwrap_or_default()),
                        parsed_input::Content::new(
                            Probability::new((prob * 100.0).round().clamp(1.0, 100.0) as _)
                                .unwrap_or_default(),
                        ),
                    )
                })
                .collect(),
            ingredients: ingredients
                .into_iter()
                .map(|(item, qty)| {
                    (
                        item,
                        parsed_input::Content::new(Quantity::new(qty).unwrap_or_default()),
                    )
                })
                .collect(),
            empty_qty: Default::default(),
            empty_proba: Default::default(),
        }
    }

    /// Builds the [`Recipe`].
    pub fn build(self) -> Recipe<Item> {
        Recipe::new(
            self.ingredients
                .into_iter()
                .map(|(item, qty)| (item, **qty))
                .collect(),
            self.products
                .into_iter()
                .map(|(item, qty, proba)| (item, **qty, **proba as f32 / 100.0))
                .collect(),
        )
    }

    /// Performs a [`BuilderAction`]
    pub fn perform(&mut self, action: BuilderAction) {
        match action {
            BuilderAction::AddProduct(item, qty, prob) => self.products.push((
                item,
                parsed_input::Content::new(qty),
                parsed_input::Content::new(prob),
            )),
            BuilderAction::AddIngredient(item, qty) => self
                .ingredients
                .push((item, parsed_input::Content::new(qty))),
            BuilderAction::EditProdItem(index, item) => self
                .products
                .get_mut(index)
                .map(|(i, _, _)| *i = item)
                .unwrap_or_default(),
            BuilderAction::EditProdQty(index, qty) => self
                .products
                .get_mut(index)
                .map(|(_, q, _)| q.update(qty))
                .unwrap_or_default(),
            BuilderAction::EditProdProba(index, proba) => self
                .products
                .get_mut(index)
                .map(|(_, _, p)| p.update(proba))
                .unwrap_or_default(),
            BuilderAction::EditIngrItem(index, item) => self
                .ingredients
                .get_mut(index)
                .map(|(i, _)| *i = item)
                .unwrap_or_default(),
            BuilderAction::EditIngrQty(index, qty) => self
                .ingredients
                .get_mut(index)
                .map(|(_, q)| q.update(qty))
                .unwrap_or_default(),
            BuilderAction::DelProd(index) => {
                let products = &mut self.products;
                if index < products.len() {self.products.remove(index);}
            }
            BuilderAction::DelIngr(index) => {
                let ingredients = &mut self.ingredients;
                if index < ingredients.len() {self.ingredients.remove(index);}
            }
        }
    }
}

/// Displays a [`BuilderState`]
pub struct Builder<'a, Message> {
    state: &'a BuilderState,
    on_action: Box<dyn Fn(BuilderAction) -> Message + 'a>,
    on_build: Option<Message>,
}

impl<'a, Message> Builder<'a, Message> {
    /// Creates a new [`Builder`] with a given [`BuilderState`],
    /// and a function to create messages.
    pub fn new(state: &'a BuilderState, on_action: impl Fn(BuilderAction) -> Message + 'a) -> Self {
        Self {
            state,
            on_action: Box::new(on_action),
            on_build: None,
        }
    }

    /// Adds a `build` button, which emits a message when pressed.
    pub fn on_build(mut self, on_build: Message) -> Self {
        self.on_build = Some(on_build);
        self
    }
}

impl<'a, Message: Clone + 'a> From<Builder<'a, Message>> for Element<'a, Message> {
    fn from(value: Builder<'a, Message>) -> Self {
        let state = value.state;
        let ingredients_vec = &state.ingredients;
        let products_vec = &state.products;

        let ingredients = recipe_column_iter(
            ingredients_vec,
            |(item, qty)| (item, qty),
            |qty| **qty,
            &value.state.empty_qty,
            &BuilderAction::DelIngr,
            &BuilderAction::EditIngrItem,
            &BuilderAction::AddIngredient,
            |index, last, qty| {
                ParsedInput::new("Quantity", qty)
                    .on_input_maybe(if !last {
                        Some(move |parsed| BuilderAction::EditIngrQty(index, parsed))
                    } else {
                        None
                    })
                    .style(color_on_err(text_input::default, ERROR_COLOR))
                    .into()
            },
        );

        let products = recipe_column_iter(
            products_vec,
            |(item, qty, proba)| (item, (qty, proba)),
            |(qty, proba)| (**qty, **proba),
            (&value.state.empty_qty, &value.state.empty_proba),
            &BuilderAction::DelProd,
            &BuilderAction::EditProdItem,
            &|item, (qty, proba)| BuilderAction::AddProduct(item, qty, proba),
            |index, last, (qty, proba)| {
                row![
                    ParsedInput::new("Quantity", qty)
                        .on_input_maybe(if !last {
                            Some(move |parsed| BuilderAction::EditProdQty(index, parsed))
                        } else {
                            None
                        })
                        .style(color_on_err(text_input::default, ERROR_COLOR)),
                    text(" ("),
                    ParsedInput::new("Odds", proba)
                        .on_input_maybe(if !last {
                            Some(move |parsed| BuilderAction::EditProdProba(index, parsed))
                        } else {
                            None
                        })
                        .style(color_on_err(text_input::default, ERROR_COLOR)),
                    text("%)"),
                ]
                .align_y(Alignment::Center)
                .into()
            },
        );

        let mut content = column![
            Element::<'_, BuilderAction>::from(layout(ingredients, products)).map(value.on_action)
        ];

        let has_invalid = ingredients_vec.iter().any(|(_, qty)| !qty.is_valid())
            || products_vec
                .iter()
                .any(|(_, qty, prob)| !qty.is_valid() || !prob.is_valid());

        if let Some(on_build) = value.on_build {
            content = content.push(horizontal_rule(SPACE));
            content = content.push(
                Button::new(title_text(TitleLevel::SectionTitle, "Build"))
                    .on_press_maybe(if has_invalid { None } else { Some(on_build) })
                    .style(move |theme, status| {
                        let style = button::success(theme, status);
                        if has_invalid {
                            match style.background {
                                Some(background) => button::Style {
                                    background: Some(filter_background(background, ERROR_COLOR)),
                                    ..style
                                },
                                None => style,
                            }
                        } else {
                            style
                        }
                    }),
            )
        }

        content.into()
    }
}
