use iced::{Element, Subscription, Task, keyboard, widget};
use minecraft_resource_tree::ui::recipe;

struct App {
    builder: recipe::EditableContent,
}

impl Default for App {
    fn default() -> Self {
        Self { builder: recipe::EditableContent::Builder(Default::default()) }
    }
}

#[derive(Debug, Clone)]
enum Message {
    Action(recipe::EditableAction),
    Build,
    Edit,
    FocusNext,
    FocusPrevious,
}

impl App {
    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Action(editable_action) => self.builder.perform(editable_action),
            Message::Build => self.builder.perform(recipe::EditableAction::Build),
            Message::Edit => self.builder.perform(recipe::EditableAction::Edit),
            Message::FocusNext => return widget::focus_next(),
            Message::FocusPrevious => return widget::focus_previous(),
        }

        Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        recipe::EditableWidget::new(&self.builder, Message::Action)
            .build_button(Message::Build)
            .edit_button(Message::Edit)
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

// type Item = String;

// mod recipe_lookup;
// mod ui;

// use iced::{widget::text, Element};
// use recipe_lookup::*;

// #[derive(Debug, Default, Clone)]
// struct App {
//     recipes: RecipeBank<Item>
// }

// #[derive(Debug, Clone)]
// enum Message {
//     ToRecipeBank()
// }

// impl App {
//     fn update(&mut self, message: Message) {
//         match message {
//             Message::ToRecipeBank() => (),
//         }
//     }

//     fn view(&self) -> Element<'_, Message> {
//         text("Hello world").into()
//     }
// }

// fn main() -> iced::Result {
//     iced::run("Resources Tree", App::update, App::view)
// }
