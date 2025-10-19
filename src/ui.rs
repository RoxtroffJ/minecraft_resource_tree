//! UI elements for the app.

use iced::{
    Alignment, Color, Element, Font,
    Length::Fill,
    advanced::{self, widget::Text},
    color,
    widget::{
        self, TextInput,
        text::{self, IntoFragment},
        text_input,
    },
};

pub mod recipe;

/// A Minecraft item
#[derive(Default, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Item {
    name: String,
}

impl Item {
    /// Creates a new item.
    pub fn new(name: impl ToString) -> Self {
        Self { name: name.to_string() }
    }

    /// [`Element`] that displays an item.
    pub fn displayer<Message>(&self) -> Element<'_, Message> {
        title_text(TitleLevel::Bald, &self.name).into()
    }

    /// [`Element`] that builds an item.
    pub fn builder<'a, Message: Clone + 'a>(
        &'a self,
        on_build: impl Fn(Self) -> Message + 'a,
    ) -> Element<'a, Message> {
        title_text_input(TitleLevel::Bald, "Item", &self.name)
            .on_input(move |name| on_build(Self::new(name)))
            .into()
    }

    /// Returns the name of the item.
    pub fn get_name(&self) -> &String {
        &self.name
    }
}

/// A title level
#[derive(Debug, Clone, Copy)]
pub enum TitleLevel {
    /// The title of a section.
    SectionTitle,
    /// Bald text.
    Bald,
}

/// [Text] formatted as title of a given level.
pub fn title_text<'a, Theme, Renderer>(
    title_level: TitleLevel,
    text: impl IntoFragment<'a>,
) -> Text<'a, Theme, Renderer>
where
    Theme: text::Catalog + 'a,
    Renderer: advanced::text::Renderer,
    Renderer::Font: From<iced::Font>,
{
    match title_level {
        TitleLevel::SectionTitle => widget::text(text).center().width(Fill).font(Font {
            weight: iced::font::Weight::Bold,
            ..Default::default()
        }),
        TitleLevel::Bald => widget::text(text).font(Font {
            weight: iced::font::Weight::Bold,
            ..Default::default()
        }),
    }
}

/// [TextInput] formatted as title of a given level.
pub fn title_text_input<'a, Message: Clone, Theme, Renderer>(
    title_level: TitleLevel,
    placeholder: &str,
    value: &str,
) -> TextInput<'a, Message, Theme, Renderer>
where
    Theme: text_input::Catalog + 'a,
    Renderer: advanced::text::Renderer<Font = iced::Font>,
{
    let input = text_input(placeholder, value);
    match title_level {
        TitleLevel::SectionTitle => input.align_x(Alignment::Center).width(Fill).font(Font {
            weight: iced::font::Weight::Bold,
            ..Default::default()
        }),
        TitleLevel::Bald => input.font(Font {
            weight: iced::font::Weight::Bold,
            ..Default::default()
        }),
    }
}

/// Use this color for warnings.
pub const ERROR_COLOR: Color = color!(0xff0000, 0.2);

/// Use this space for separators, padding, ...
pub const SPACE: u16 = 10;
