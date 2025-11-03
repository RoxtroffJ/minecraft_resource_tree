//! UI structs to build and display a [Recipe].

pub mod builder;
pub use builder::*;

pub mod displayer;
pub use displayer::*;
use iced::Element;
use serde::{Deserialize, Serialize};

use crate::{recipes::Recipe, ui::Item};

mod layout_helpers;

/// Content of an editable [`Recipe`] widget.
#[derive(Debug, Clone)]
pub enum EditableContent {
    /// The recipe is in edit, with this state.
    Builder(BuilderState),
    /// The recipe is built
    Built(Recipe<Item>),
}

/// Actions that can be [`perform`](EditableContent::perform)ed by an [`EditableContent`].
#[derive(Debug, Clone)]
pub enum EditableAction {
    /// Builds a [`Recipe`].
    ///
    /// If the [`EditableContent`] was already built, does nothing.
    Build,
    /// Switches from a built recipe to a [`Builder`].
    Edit,
    /// If this is a [`Builder`], perform the action.
    BuilderAction(BuilderAction),
}

impl EditableContent {
    /// Performs an [`EditableAction`].
    pub fn perform(&mut self, action: EditableAction) {
        let performer = |val| match action {
            EditableAction::Build => match val {
                EditableContent::Builder(builder_state) => {
                    EditableContent::Built(builder_state.build())
                }
                EditableContent::Built(_) => val,
            },
            EditableAction::Edit => match val {
                EditableContent::Builder(_) => val,
                EditableContent::Built(recipe) => {
                    EditableContent::Builder(BuilderState::from_recipe(recipe))
                }
            },
            EditableAction::BuilderAction(builder_action) => match val {
                EditableContent::Builder(mut builder_state) => {
                    builder_state.perform(builder_action);
                    EditableContent::Builder(builder_state)
                }
                EditableContent::Built(_) => val,
            },
        };

        replace_with::replace_with_or_abort(self, performer);
    }

    /// Converts to a [`EditableContentSave`].
    ///
    /// Parsing errors are lost in the process.
    pub fn save(self) -> EditableContentSave {
        match self {
            EditableContent::Builder(builder_state) => {
                EditableContentSave::Builder(builder_state.save())
            }
            EditableContent::Built(recipe) => EditableContentSave::Built(recipe),
        }
    }
}

/// Widget that can switch from a [`Builder`] to a [`RecipeWidget`]
pub struct EditableWidget<'a, Message> {
    content: Result<RecipeWidget<'a, Message>, Builder<'a, Message>>,
}

impl<'a, Message> EditableWidget<'a, Message> {
    /// Creates a new widget.
    pub fn new(
        content: &'a EditableContent,
        on_action: impl Fn(EditableAction) -> Message + 'a,
    ) -> Self {
        match content {
            EditableContent::Builder(builder_state) => Self {
                content: Err(Builder::new(builder_state, move |action| {
                    on_action(EditableAction::BuilderAction(action))
                })),
            },
            EditableContent::Built(recipe) => Self {
                content: Ok(RecipeWidget::new(recipe)),
            },
        }
    }

    /// Adds a `build` button to the [`Builder`] widget, if this is one, which emits a message when pressed.
    pub fn build_button(mut self, on_pressed: Message) -> Self {
        self.content = self.content.map_err(|builder| builder.on_build(on_pressed));
        self
    }

    /// Adds an `edit` button to the [`RecipeWidget`], if this is one, which emits a message when pressed.
    pub fn edit_button(mut self, on_pressed: Message) -> Self {
        self.content = self.content.map(|widget| widget.on_edit(on_pressed));
        self
    }
}

impl<'a, Message: Clone + 'a> From<EditableWidget<'a, Message>> for Element<'a, Message> {
    fn from(value: EditableWidget<'a, Message>) -> Self {
        match value.content {
            Ok(display) => display.into(),
            Err(builder) => builder.into(),
        }
    }
}

/// A serde compatible version of [`EditableContent`].
#[derive(Serialize, Deserialize, Clone)]
pub enum EditableContentSave {
    /// Builder variant.
    Builder(BuilderSave),
    /// Built variant.
    Built(Recipe<Item>),
}

impl From<EditableContentSave> for EditableContent {
    fn from(value: EditableContentSave) -> Self {
        match value {
            EditableContentSave::Builder(builder_save) => EditableContent::Builder(builder_save.into()),
            EditableContentSave::Built(recipe) => EditableContent::Built(recipe),
        }
    }
}