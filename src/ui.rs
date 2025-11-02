//! UI elements for the app.

use std::{fmt::Display, num::ParseFloatError, ops::{Deref, DerefMut}, str::FromStr};

use iced::{
    Alignment, Border, Color, Element, Font, Length::{Fill, Shrink}, advanced::{self, widget::Text}, widget::{
        self, Container, TextInput, container, text::{self, IntoFragment}, text_input
    }
};

pub mod recipe;

/// A Minecraft item
#[derive(Default, Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Item {
    name: String,
}

impl Item {
    /// Creates a new item.
    pub fn new(name: impl ToString) -> Self {
        Self {
            name: name.to_string(),
        }
    }

    /// [`Element`] that displays an item.
    pub fn displayer<Message>(&self) -> Element<'_, Message> {
        title_text(TitleLevel::Bald, &self.name).width(Shrink).into()
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
    /// The title of a subsection.
    SubSectionTitle,
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
        TitleLevel::SubSectionTitle => widget::text(format!("---- {} ----", text.into_fragment())).font(Font {
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
    
    match title_level {
        TitleLevel::SectionTitle => text_input(placeholder, value).align_x(Alignment::Center).width(Fill).font(Font {
            weight: iced::font::Weight::Bold,
            ..Default::default()
        }),
        TitleLevel::SubSectionTitle => text_input(placeholder, format!("---- {value} ----").deref()).font(Font {
            weight: iced::font::Weight::Bold,
            ..Default::default()
        }),
        TitleLevel::Bald => text_input(placeholder, value).font(Font {
            weight: iced::font::Weight::Bold,
            ..Default::default()
        }),
    }
}

/// Provides a function that converts a theme into a color.
pub trait ThemeColor<Theme> {
    /// Converts a theme into a color.
    fn get_color(&self, theme: &Theme) -> Color;
} 

impl<Theme> ThemeColor<Theme> for Color {
    fn get_color(&self, _theme: &Theme) -> Color {
        *self
    }
}

impl<T: Fn(&Theme) -> Color, Theme> ThemeColor<Theme> for T {
    fn get_color(&self, theme: &Theme) -> Color {
        self(theme)
    }
}

/// [Container] with a contour, and padding of [SPACE].
pub fn contoured<'a, Message, Theme, Renderer>(
    content: impl Into<Element<'a, Message, Theme, Renderer>>,
    color: impl ThemeColor<Theme> + 'a,
) -> Container<'a, Message, Theme, Renderer>
where
    Theme: container::Catalog + 'a,
    Renderer: advanced::Renderer,
    Theme::Class<'a>: From<container::StyleFn<'a, Theme>>,
{
    Container::new(content)
        .style(move |theme| {
            container::transparent(theme).border(
                Border::default()
                    .color(color.get_color(theme))
                    .rounded(SPACE)
                    .width(1.),
            )
        })
        .padding(SPACE)
}

/// Positive float
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct TargetAmount {
    amount: f64
}

impl TryFrom<f64> for TargetAmount {
    type Error = ParseTargetAmountError;

    fn try_from(value: f64) -> Result<Self, Self::Error> {
        if value >= 0. {
            Ok(Self {
                amount: value
            })
        } else {
            Err(ParseTargetAmountError::Negative)
        }
    }
}

impl From<TargetAmount> for f64 {
    fn from(value: TargetAmount) -> Self {
        value.amount
    }
}

impl Deref for TargetAmount {
    type Target = f64;

    fn deref(&self) -> &Self::Target {
        &self.amount
    }
}

impl Default for TargetAmount {
    fn default() -> Self {
        Self { amount: 1. }
    }
}

impl Display for TargetAmount {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.amount.fmt(f)
    }
}

/// Parse error for a [`TargetAmount`]
#[derive(Debug, Clone)]
pub enum ParseTargetAmountError {
    /// Could not parse float.
    Parse(ParseFloatError),
    /// Parsed float was negative.
    Negative
}

impl FromStr for TargetAmount {
    type Err = ParseTargetAmountError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse::<f64>().map_err(ParseTargetAmountError::Parse)?.try_into()
        
    }
}

/// Float with special Display impl.
#[derive(Debug, Default, Clone, Copy, PartialEq, PartialOrd)]
pub struct DisplayFloat {
    v: f64
}

impl DisplayFloat {
    /// Converts a f64 to a DisplayFloat
    pub fn new(v: f64) -> Self {
        v.into()
    }
}

impl From<f64> for DisplayFloat {
    fn from(value: f64) -> Self {
        Self { v: value }
    }
}

impl From<DisplayFloat> for f64 {
    fn from(value: DisplayFloat) -> Self {
        value.v
    }
}

impl Deref for DisplayFloat {
    type Target = f64;

    fn deref(&self) -> &Self::Target {
        &self.v
    }
}

impl DerefMut for DisplayFloat {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.v
    }
}

impl FromStr for DisplayFloat {
    type Err = ParseFloatError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse().map(Self::new)
    }
}

impl Display for DisplayFloat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = format!("{:.2}", self.v);
        write!(f, "{}", str.trim_end_matches('0').trim_end_matches('.'))
    }
}

/// Use this space for separators, padding, ...
pub const SPACE: u16 = 10;
