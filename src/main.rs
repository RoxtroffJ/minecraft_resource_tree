use iced::{
    Border, Element,
    Length::Fill,
    Subscription, Task, keyboard,
    widget::{self, Column, Container, Scrollable, button, center, container, text},
};
use iced_aw::ContextMenu;
use minecraft_resource_tree::ui::{
    SPACE,
    recipe::{self, BuilderState},
};

struct App {
    recipes: Vec<recipe::EditableContent>,
}

impl Default for App {
    fn default() -> Self {
        Self {
            recipes: Default::default(),
        }
    }
}

#[derive(Debug, Clone)]
enum Message {
    Action(usize, recipe::EditableAction),
    Build(usize),
    Edit(usize),
    Delete(usize),
    AddRecipe,
    FocusNext,
    FocusPrevious,
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
                .map(|recipe| recipe.perform(recipe::EditableAction::Build))
                .unwrap_or_default(),
            Message::Edit(index) => self
                .recipes
                .get_mut(index)
                .map(|recipe| recipe.perform(recipe::EditableAction::Edit))
                .unwrap_or_default(),
            Message::Delete(index) => {
                let recipes = &mut self.recipes;
                if index < recipes.len() {
                    recipes.remove(index);
                }
            }
            Message::AddRecipe => {
                let content = BuilderState::new();
                self.recipes.push(recipe::EditableContent::Builder(content));
            }
            Message::FocusNext => return widget::focus_next(),
            Message::FocusPrevious => return widget::focus_previous(),
        }

        Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        let recipes = self.recipes.iter().enumerate().map(|(index, recipe)| {
            Container::new(ContextMenu::new(
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
                        res = res.push(button(text("Edit recipe")).on_press(Message::Edit(index)))
                    };

                    res.push(
                        button(text("Delete recipe"))
                            .on_press(Message::Delete(index))
                            .style(button::danger),
                    )
                    .into()
                },
            ))
            .style(|theme| {
                container::transparent(theme).border(
                    Border::default()
                        .color(theme.palette().text)
                        .rounded(SPACE)
                        .width(1.),
                )
            })
            .padding(SPACE)
            .into()
        });

        let content = Scrollable::new(
            Column::with_children(recipes)
                .push(
                    button("Add recipe")
                        .on_press(Message::AddRecipe)
                        .style(button::success)
                        .width(Fill),
                )
                .spacing(SPACE),
        );

        Container::new(content)
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
