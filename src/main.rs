use std::collections::HashMap;

use iced::{
    Element,
    Length::Fill,
    Subscription, Task, keyboard,
    widget::{self, Column, Container, Scrollable, button, column, horizontal_rule, row, text},
};
use iced_aw::ContextMenu;
use minecraft_resource_tree::ui::{
    Item, SPACE, TitleLevel, contoured,
    recipe::{self, BuilderState},
    title_text,
};

#[derive(Default)]
struct App {
    recipes: Vec<recipe::EditableContent>,
    known_items: HashMap<Item, usize>,
}

#[derive(Debug, Clone)]
enum Message {
    Action(usize, recipe::EditableAction),
    Build(usize),
    Edit(usize),
    Delete(usize),
    AddRecipe,

    Compute,

    FocusNext,
    FocusPrevious,
}

macro_rules! remove_recipe_items {
    ($app:expr, $recipe:expr) => {
        match $recipe {
            recipe::EditableContent::Builder(_) => (),
            recipe::EditableContent::Built(recipe) => {
                let inputs = recipe.get_ingredients().iter().map(|(item, _)| item);
                let outputs = recipe.get_products().iter().map(|(item, _, _)| item);

                for item in inputs.chain(outputs) {
                    match $app.known_items.get_mut(item) {
                        Some(qty) => {
                            if *qty == 0 {
                                $app.known_items.remove(item);
                            } else {
                                *qty -= 1
                            }
                        }
                        None => (),
                    }
                }
            }
        }
    };
}

impl App {
    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Action(index, editable_action) => self
                .recipes
                .get_mut(index)
                .map(|reicpe| reicpe.perform(editable_action))
                .unwrap_or_default(),
            Message::Build(index) => self
                .recipes
                .get_mut(index)
                .map(|recipe| {
                    recipe.perform(recipe::EditableAction::Build);
                    match recipe {
                        recipe::EditableContent::Builder(_) => (),
                        recipe::EditableContent::Built(recipe) => {
                            let inputs = recipe.get_ingredients().iter().map(|(item, _)| item);
                            let outputs = recipe.get_products().iter().map(|(item, _, _)| item);

                            for item in inputs.chain(outputs) {
                                match self.known_items.get_mut(item) {
                                    Some(qty) => *qty += 1,
                                    None => {
                                        self.known_items.insert(item.clone(), 0);
                                    }
                                }
                            }
                        }
                    }
                })
                .unwrap_or_default(),
            Message::Edit(index) => {
                self
                .recipes
                .get_mut(index)
                .map(|recipe| {
                    remove_recipe_items!(self, recipe);
                    recipe.perform(recipe::EditableAction::Edit);
                })
                .unwrap_or_default()},
            Message::Delete(index) => {
                let recipes = &mut self.recipes;
                recipes.get_mut(index).map(|recipe| remove_recipe_items!(self, recipe));
                if index < recipes.len() {
                    recipes.remove(index);
                }
            }
            Message::AddRecipe => {
                let content = BuilderState::new();
                self.recipes.push(recipe::EditableContent::Builder(content));
            }

            Message::Compute => {}

            Message::FocusNext => return widget::focus_next(),
            Message::FocusPrevious => return widget::focus_previous(),
        }

        Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        let recipes = self.recipes.iter().enumerate().map(|(index, recipe)| {
            contoured(
                ContextMenu::new(
                    recipe::EditableWidget::new(recipe, move |a| Message::Action(index, a))
                        .build_button(Message::Build(index)),
                    move || {
                        let mut res = Column::new();
                        if self
                            .recipes
                            .get(index)
                            .map(|recipe| match recipe {
                                recipe::EditableContent::Builder(_) => false,
                                recipe::EditableContent::Built(_) => true,
                            })
                            .unwrap_or(false)
                        {
                            res =
                                res.push(button(text("Edit recipe")).on_press(Message::Edit(index)))
                        };

                        res.push(
                            button(text("Delete recipe"))
                                .on_press(Message::Delete(index))
                                .style(button::danger),
                        )
                        .into()
                    },
                ),
                |theme: &iced::Theme| theme.palette().text,
            )
            .into()
        });

        let recipes = Scrollable::new(
            Column::with_children(recipes)
                .push(
                    button(title_text(TitleLevel::SectionTitle, "Add recipe"))
                        .on_press(Message::AddRecipe)
                        //.style(button::success)
                        .width(Fill),
                )
                .spacing(SPACE),
        )
        .height(Fill)
        .spacing(SPACE);

        let details = Column::with_children(self.known_items.keys().map(|item| item.displayer()));

        let content = row![
            column![
                title_text(TitleLevel::SectionTitle, "Recipes"),
                horizontal_rule(SPACE),
                recipes
            ],
            column![
                title_text(TitleLevel::SectionTitle, "Details"),
                horizontal_rule(SPACE),
                details
            ],
        ]
        .spacing(SPACE);

        let compute_button = button(title_text(TitleLevel::SectionTitle, "Compute"))
            .width(Fill)
            .on_press_maybe(
                if self.recipes.iter().all(|recipe| match recipe {
                    recipe::EditableContent::Builder(_) => false,
                    recipe::EditableContent::Built(_) => true,
                }) {
                    Some(Message::Compute)
                } else {
                    None
                },
            );

        Container::new(column![content, compute_button].spacing(SPACE))
            .padding(SPACE)
            .width(Fill)
            .height(Fill)
            .into()
    }

    fn subscription(&self) -> Subscription<Message> {
        keyboard::on_key_press(|key, modifiers| match key {
            keyboard::Key::Named(named) => match named {
                keyboard::key::Named::Tab => {
                    if modifiers.shift() {
                        Some(Message::FocusPrevious)
                    } else {
                        Some(Message::FocusNext)
                    }
                }
                _ => None,
            },
            _ => None,
        })
    }
}

fn main() -> iced::Result {
    iced::application("Test builder", App::update, App::view)
        .subscription(App::subscription)
        .run()
}
