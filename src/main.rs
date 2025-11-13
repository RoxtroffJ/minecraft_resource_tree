use std::{
    collections::{BTreeMap, HashMap},
    env::current_dir,
    fmt::Display,
    fs::File,
    path::{Path, PathBuf},
};

use craft_tree_optimizer::ui::{
    DisplayFloat, GRAY, Item, ParseTargetAmountError, SPACE, TargetAmount, TitleLevel, contoured,
    recipe::{self, BuilderState, EditableContentSave},
    title_text,
};
use good_lp::{Expression, ProblemVariables, Solution, SolverModel, solvers, variable};
use iced::{
    Element,
    Length::*,
    Padding, Subscription, Task, keyboard,
    widget::{
        self, Checkbox, Column, Container, Scrollable, Space, Stack, button, center, column,
        container, horizontal_rule, horizontal_space, opaque, row, text, text_input,
    },
    window,
};
use iced_aw::{ContextMenu, TypedInput};
use more_iced_aw::{
    element_vec,
    grid::{self, Grid},
    parsed_input::{self, ParsedInput},
};
use rfd::FileDialog;
use serde::{Deserialize, Serialize};

const EXTENSION: &'static str = "crtr";

struct App {
    recipes: Vec<recipe::EditableContent>,
    known_items: BTreeMap<
        Item,
        (
            usize,
            Option<parsed_input::Content<TargetAmount, ParseTargetAmountError>>,
            Option<parsed_input::Content<TargetAmount, ParseTargetAmountError>>,
        ),
    >, // Quantity, target, raw
    error: Option<String>,

    recipe_uses: Option<Vec<f64>>,
    item_stats: Option<HashMap<Item, (f64, f64)>>, // produced used
    scale: TargetAmount,

    unsaved_changes: bool,
    path: Option<PathBuf>,
    import_error: Option<ImportError>,
    save_error: Option<SaveError>,

    save_popup: Option<Message>,
}

#[derive(Debug, Clone)]
enum Message {
    Action(usize, recipe::EditableAction),
    Build(usize),
    Edit(usize),
    Delete(usize),
    AddRecipe,

    ToggleTarget(Item, bool),
    EditTargetAmount(
        Item,
        parsed_input::Parsed<TargetAmount, ParseTargetAmountError>,
    ),
    ToggleRaw(Item, bool),
    EditRawCost(
        Item,
        parsed_input::Parsed<TargetAmount, ParseTargetAmountError>,
    ),

    Compute,
    ComputeError(String),

    EditScale(TargetAmount),

    FocusNext,
    FocusPrevious,

    OpenButton,
    SaveButton,
    SaveAsButton,

    Open(PathBuf),
    Save(PathBuf),

    CloseRequest(window::Id),
    Close(window::Id),

    SaveDone,
    PopupCancel,
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
                        Some((qty, _, _)) => {
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
        self.import_error = None;
        self.save_error = None;
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
                                    Some((qty, _, _)) => *qty += 1,
                                    None => {
                                        self.known_items.insert(item.clone(), (0, None, None));
                                    }
                                }
                            }
                        }
                    }
                })
                .unwrap_or_default(),
            Message::Edit(index) => self
                .recipes
                .get_mut(index)
                .map(|recipe| {
                    remove_recipe_items!(self, recipe);
                    recipe.perform(recipe::EditableAction::Edit);
                })
                .unwrap_or_default(),
            Message::Delete(index) => {
                let recipes = &mut self.recipes;
                recipes
                    .get_mut(index)
                    .map(|recipe| remove_recipe_items!(self, recipe));
                if index < recipes.len() {
                    recipes.remove(index);
                }
            }
            Message::AddRecipe => {
                let content = BuilderState::new();
                self.recipes.push(recipe::EditableContent::Builder(content));
            }
            Message::ToggleTarget(item, toggle) => {
                self.known_items.get_mut(&item).map(|(_, target, _)| {
                    if toggle {
                        target.get_or_insert(parsed_input::Content::default());
                    } else {
                        *target = None
                    }
                });
            }
            Message::EditTargetAmount(item, val) => {
                self.known_items
                    .get_mut(&item)
                    .and_then(|(_, c, _)| c.as_mut())
                    .map(|c| c.update(val));
            }
            Message::ToggleRaw(item, toggle) => {
                self.known_items.get_mut(&item).map(|(_, _, raw)| {
                    if toggle {
                        raw.get_or_insert(parsed_input::Content::default());
                    } else {
                        *raw = None
                    }
                });
            }
            Message::EditRawCost(item, val) => {
                self.known_items
                    .get_mut(&item)
                    .and_then(|(_, _, c)| c.as_mut())
                    .map(|c| c.update(val));
            }
            Message::Compute => {
                // One variable per recipes, all superior to 0.
                let mut problem = ProblemVariables::new();
                let variables = problem.add_vector(variable().min(0), self.recipes.len());

                // Item expressions. Hash map with (expr_prod expr_uses)
                let mut item_expressions: HashMap<Item, (Expression, Expression)> = self
                    .known_items
                    .iter()
                    .map(|(item, _)| (item.clone(), Default::default()))
                    .collect();

                // For each recipe, edit the expressions of the items.
                for (index, recipe) in self.recipes.iter().enumerate() {
                    match recipe {
                        recipe::EditableContent::Builder(_) => {
                            return Task::done(Message::ComputeError(
                                "One of the recipies is not build.".to_string(),
                            ));
                        }
                        recipe::EditableContent::Built(recipe) => {
                            for (item, qty, product) in
                                recipe
                                    .get_ingredients()
                                    .iter()
                                    .map(|(item, qty)| (item, *qty as f64, false))
                                    .chain(recipe.get_products().iter().map(|(item, qty, prob)| {
                                        (item, (*qty as f64) * prob, true)
                                    }))
                            {
                                match item_expressions.get_mut(item) {
                                    Some((prod_expr, uses_expr)) => {
                                        if product {
                                            prod_expr.add_mul(qty, variables[index])
                                        } else {
                                            uses_expr.add_mul(qty, variables[index])
                                        }
                                    }
                                    None => {
                                        return Task::done(Message::ComputeError(format!(
                                            "Internal error: no expression found for item {} during expression update with recipe",
                                            item.get_name()
                                        )));
                                    }
                                }
                            }
                        }
                    }
                }

                // Go through the item list and build the constraints / targets / costs
                let mut total_cost = Expression::default();
                let mut constraints = Vec::new();
                for (item, (_, target, raw)) in self.known_items.iter() {
                    let Some((prod_expr, uses_expr)) = item_expressions.get(&item) else {
                        return Task::done(Message::ComputeError(format!(
                            "Internal error: no expression found for item {} during constraint build",
                            item.get_name()
                        )));
                    };

                    let expression = prod_expr.clone() - uses_expr.clone();

                    if let Some(target) = target.as_deref() {
                        constraints.push((expression >> **target).set_name(item.get_name().clone()))
                    } else {
                        match raw {
                            Some(cost) => total_cost.add_mul(-(***cost), expression),
                            None => constraints
                                .push((expression >> 0).set_name(item.get_name().clone())),
                        }
                    }
                }

                // Solve
                match problem
                    .minimise(total_cost)
                    .using(solvers::clarabel::clarabel)
                    .with_all(constraints)
                    .solve()
                {
                    Ok(solution) => {
                        self.recipe_uses = Some(
                            variables
                                .into_iter()
                                .map(|var| solution.value(var))
                                .collect(),
                        );

                        let item_stats = self.item_stats.insert(HashMap::new());

                        for (item, (prod_expr, uses_expr)) in item_expressions {
                            let prod = solution.eval(prod_expr);
                            let uses = solution.eval(uses_expr);
                            item_stats.insert(item, (prod, uses));
                        }

                        match solution.status() {
                            good_lp::SolutionStatus::Optimal => (),
                            status => {
                                return Task::done(Message::ComputeError(format!(
                                    "Solution is not optimal. {status:?}"
                                )));
                            }
                        }
                    }
                    Err(err) => {
                        return Task::done(Message::ComputeError(format!(
                            "Could not solve: {err}"
                        )));
                    }
                }
                self.unsaved_changes = true;
                return Task::none();
            }
            Message::ComputeError(msg) => {
                self.error = Some(msg);
                return Task::none();
            }
            Message::EditScale(v) => {
                self.scale = v;
                self.unsaved_changes = true;
                return Task::none();
            }
            Message::FocusNext => return widget::focus_next(),
            Message::FocusPrevious => return widget::focus_previous(),

            Message::OpenButton => {
                let message = {
                    let path = file_select_win_builder(
                        "Open file ...",
                        self.path
                            .as_ref()
                            .cloned()
                            .unwrap_or_else(|| current_dir().unwrap_or_default()),
                        self.path
                            .as_ref()
                            .and_then(|path| path.file_name())
                            .and_then(|str| str.to_str()),
                    )
                    .pick_file();
                    match path {
                        Some(path) => Message::Open(path),
                        None => return Task::none(),
                    }
                };

                return self.save_popup(message);
            }
            Message::SaveButton => match &self.path {
                Some(path) => return Task::done(Message::Save(path.clone())),
                None => return Task::done(Message::SaveAsButton),
            },
            Message::SaveAsButton => {
                let path = file_select_win_builder(
                    "Save as ...",
                    self.path
                        .as_ref()
                        .cloned()
                        .unwrap_or_else(|| current_dir().unwrap_or_default()),
                    self.path
                        .as_ref()
                        .and_then(|path| path.file_name())
                        .and_then(|str| str.to_str()),
                )
                .save_file();
                match path {
                    Some(path) => return Task::done(Message::Save(path)),
                    None => return Task::none(),
                }
            }
            Message::Open(path_buf) => {
                match Self::from_file(path_buf) {
                    Ok(app) => *self = app,
                    Err(err) => self.import_error = Some(err),
                }
                return Task::none();
            }
            Message::Save(mut path_buf) => {
                path_buf.set_extension(EXTENSION);
                return self.save_and_set_err(path_buf);
            }

            Message::SaveDone => {
                return self
                    .save_popup
                    .take()
                    .map(Task::done)
                    .unwrap_or(Task::none());
            }
            Message::PopupCancel => {
                self.save_popup = None;
                return Task::none();
            }

            Message::CloseRequest(id) => return self.save_popup(Message::Close(id)),
            Message::Close(id) => return window::close(id),
        }
        self.unsaved_changes = true;
        self.error = None;
        self.recipe_uses = None;
        self.item_stats = None;
        Task::none()
    }

    fn save_popup(&mut self, msg: Message) -> Task<Message> {
        if self.unsaved_changes {
            self.save_popup = Some(msg);
            Task::none()
        } else {
            Task::done(msg)
        }
    }

    fn view(&self) -> Element<'_, Message> {
        // Utilities

        let widther = || Space::new(75, Shrink);
        let scale_field = |placeholder: &str, value: f64| {
            TypedInput::new(placeholder, &DisplayFloat::new(value * *self.scale))
                .on_input(move |n| {
                    Message::EditScale((f64::from(n) / value).try_into().unwrap_or(self.scale))
                })
                .width(Fill)
        };

        // Recipes

        let recipes = self.recipes.iter().enumerate().map(|(index, recipe)| {
            contoured(
                ContextMenu::new(
                    row![
                        recipe::EditableWidget::new(recipe, move |a| Message::Action(index, a))
                            .build_button(Message::Build(index))
                    ]
                    .push_maybe(
                        self.recipe_uses
                            .as_ref()
                            .and_then(|vec| vec.get(index))
                            .map(|nb| {
                                column![
                                    title_text(TitleLevel::SubSectionTitle, "Uses"),
                                    widther(),
                                    horizontal_rule(SPACE),
                                    scale_field("nb", *nb)
                                ]
                                .width(Shrink)
                            }),
                    )
                    .spacing(SPACE),
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
        .width(Fill)
        .spacing(SPACE);

        let all = self.known_items.iter().collect::<Vec<_>>();
        let mut targets = Vec::new();
        let mut raws = Vec::new();
        for (item, (_, target, raw)) in all.iter() {
            raw.as_ref().map(|cost| raws.push((item, cost)));
            target.as_ref().map(|t| targets.push((item, t)));
        }

        let all_targets_ok = targets.len() > 0 && targets.iter().all(|(_, c)| (*c).is_valid());
        let all_raws_ok = raws.len() > 0 && raws.iter().all(|(_, c)| (*c).is_valid());

        // Item details

        let details = Scrollable::new({
            let mut targets_rows = vec![
                {
                    let mut row = element_vec![
                        title_text(TitleLevel::SubSectionTitle, "Targets"),
                        text("Target amount")
                    ];
                    if self.item_stats.is_some() {
                        row.push("Net production")
                    }
                    row
                },
                {
                    let mut row = element_vec![Space::new(Shrink, Shrink), widther(),];
                    if self.item_stats.is_some() {
                        row.push(widther())
                    }
                    row
                },
                {
                    let mut row = element_vec![horizontal_rule(SPACE), horizontal_rule(SPACE)];
                    if self.item_stats.is_some() {
                        row.push(horizontal_rule(SPACE))
                    }
                    row
                },
            ];
            targets_rows.extend(targets.into_iter().map(|(item, amount)| {
                let mut row = element_vec![
                    item.displayer(),
                    ParsedInput::new("Amount per batch", amount)
                        .on_input(|v| Message::EditTargetAmount((**item).clone(), v))
                        .style(parsed_input::danger_on_err(text_input::default))
                ];
                if let Some((prod, uses)) = self.item_stats.as_ref().and_then(|tbl| tbl.get(item)) {
                    row.push(scale_field("Net production", prod - uses))
                }
                row
            }));

            let mut raws_rows = vec![
                {
                    let mut row = element_vec![
                        title_text(TitleLevel::SubSectionTitle, "Raw materials").width(Shrink),
                        text("Cost of one")
                    ];
                    if self.item_stats.is_some() {
                        row.push("Required");
                        row.push("Cost");
                        row.push("Used");
                        row.push("Produced");
                    }
                    row
                },
                {
                    let mut row = element_vec![Space::new(Shrink, Shrink), widther(),];
                    if self.item_stats.is_some() {
                        row.extend([widther(), widther(), widther(), widther()])
                    }
                    row
                },
                {
                    let mut row = element_vec![horizontal_rule(SPACE), horizontal_rule(SPACE)];
                    if self.item_stats.is_some() {
                        row.push(horizontal_rule(SPACE));
                        row.push(horizontal_rule(SPACE));
                        row.push(horizontal_rule(SPACE));
                        row.push(horizontal_rule(SPACE));
                    }
                    row
                },
            ];
            
            let mut total_required = 0.;
            let mut total_cost = 0.;
            let mut total_used = 0.;
            let mut total_produced = 0.;

            raws_rows.extend(raws.into_iter().map(|(item, cost)| {
                let mut row = element_vec![
                    item.displayer(),
                    ParsedInput::new("Cost of one", cost)
                        .on_input(|v| Message::EditRawCost((**item).clone(), v))
                        .style(parsed_input::danger_on_err(text_input::default))
                ];
                if let Some((prod, uses)) = self.item_stats.as_ref().and_then(|tbl| tbl.get(item)) {
                    let cost_items = ***cost * (uses - prod);

                    total_required += uses - prod;
                    total_cost += cost_items;
                    total_used += *uses;
                    total_produced += *prod;
                    
                    row.extend(element_vec![
                        scale_field("Net required", uses - prod),
                        scale_field("Cost", cost_items),
                        scale_field("Used", *uses),
                        scale_field("Produced", *prod)
                    ]);
                }
                row
            }));



            let mut all_rows = vec![
                {
                    let mut row = element_vec![
                        title_text(TitleLevel::SubSectionTitle, "All"),
                        text("target"),
                        text("raw material")
                    ];
                    if self.item_stats.is_some() {
                        row.extend(element_vec!["Uses", "Produced", "Net production"])
                    }
                    row
                },
                {
                    let mut row = element_vec![
                        Space::new(Shrink, Shrink),
                        Space::new(Shrink, Shrink),
                        Space::new(Shrink, Shrink)
                    ];
                    if self.item_stats.is_some() {
                        row.extend([widther(), widther(), widther()])
                    }
                    row
                },
                {
                    let mut row = element_vec![
                        horizontal_rule(SPACE),
                        horizontal_rule(SPACE),
                        horizontal_rule(SPACE),
                    ];
                    if self.item_stats.is_some() {
                        row.extend([
                            horizontal_rule(SPACE),
                            horizontal_rule(SPACE),
                            horizontal_rule(SPACE),
                        ])
                    }
                    row
                },
            ];

            let mut total_uses = 0.;
            let mut total_prod = 0.;
            let mut total_net = 0.;

            all_rows.extend(all.iter().map(|(item, (_, target, raw))| {
                let mut row = element_vec![
                    item.displayer(),
                    Checkbox::new("", target.is_some())
                        .on_toggle(|v| Message::ToggleTarget((*item).clone(), v)),
                    Checkbox::new("", raw.is_some())
                        .on_toggle(|v| Message::ToggleRaw((*item).clone(), v))
                ];
                if let Some((prod, uses)) = self.item_stats.as_ref().and_then(|tbl| tbl.get(item)) {
                    total_uses += *uses;
                    total_prod += *prod;
                    total_net += prod - uses;
                    
                    row.extend(element_vec![
                        scale_field("Uses", *uses),
                        scale_field("Produced", *prod),
                        scale_field("Net production", prod - uses),
                    ]);
                }
                row
            }));

            if self.item_stats.is_some() {
                raws_rows.push(element_vec!(Space::new(Shrink, SPACE)));
                raws_rows.push(element_vec![
                    Space::new(Shrink, Shrink),
                    "Totals:",
                    scale_field("Required total", total_required),
                    scale_field("Cost total", total_cost),
                    scale_field("Used total", total_used),
                    scale_field("Produced total", total_produced)
                ]);
                
                all_rows.push(element_vec!(Space::new(Shrink, SPACE)));
                all_rows.push(element_vec![
                    Space::new(Shrink, Shrink),
                    Space::new(Shrink, Shrink),
                    "Totals:",
                    scale_field("Uses total", total_uses),
                    scale_field("Produced total", total_prod),
                    scale_field("Net production total", total_net)
                ]);
            }

            let raws_elt = Grid::with_rows(raws_rows)
                .column_spacing(SPACE)
                .main_axis(grid::Axis::Vertical)
                .width(Shrink);
            let targets_elt = Grid::with_rows(targets_rows)
                .column_spacing(SPACE)
                .main_axis(grid::Axis::Vertical)
                .width(Shrink);
            let all_elt = Grid::with_rows(all_rows)
                .column_spacing(SPACE)
                .main_axis(grid::Axis::Vertical)
                .width(Shrink);

            column![
                contoured(targets_elt, |theme: &iced::Theme| theme.palette().text),
                contoured(raws_elt, |theme: &iced::Theme| theme.palette().text),
                contoured(all_elt, |theme: &iced::Theme| theme.palette().text)
            ]
            .spacing(SPACE)
        })
        .height(Fill)
        .width(Fill)
        .spacing(SPACE);

        // Main content

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

        // Compute button

        let all_recipes_ok = self.recipes.len() > 0
            && self.recipes.iter().all(|recipe| match recipe {
                recipe::EditableContent::Builder(_) => false,
                recipe::EditableContent::Built(_) => true,
            });

        let compute_button = button(title_text(TitleLevel::SectionTitle, "Compute"))
            .width(Fill)
            .on_press_maybe(
                if self.error.is_none() && all_recipes_ok && all_targets_ok && all_raws_ok {
                    Some(Message::Compute)
                } else {
                    None
                },
            );
        let compute_button = if self.error.is_some() {
            compute_button.style(button::danger)
        } else {
            compute_button
        };

        // Menu bar
        let menu_bar = row![
            button("Open").on_press(Message::OpenButton),
            button("Save").on_press(Message::SaveButton),
            button("Save as").on_press(Message::SaveAsButton)
        ]
        .spacing(SPACE);

        // Main window

        let main_window = column![
            menu_bar.padding(Padding {
                top: SPACE as f32 / 2.,
                right: SPACE.into(),
                bottom: 0.,
                left: SPACE.into()
            }),
            horizontal_rule(SPACE),
            Container::new(
                column![content, compute_button]
                    .push_maybe(
                        self.save_error
                            .as_ref()
                            .map(|err| text(err.to_string()))
                            .or(self.error.as_ref().map(text))
                            .or(self.import_error.as_ref().map(|err| text(err.to_string())))
                            .map(|elt| elt.style(text::danger)),
                    )
                    .spacing(SPACE),
            )
            .padding(SPACE)
            .width(Fill)
            .height(Fill)
        ];

        // Add save popup or build element
        let elt = if self.save_popup.is_some() {
            let popup = center(container(
                Grid::new().push_row(
                    [title_text(TitleLevel::SectionTitle, "Save changes before close?")])
                .push_row(
                    ["You have unsaved changes. If you don't save now, these changes will be lost."]
                ).push_row(
                    [row![
                        button("Cancel").on_press(Message::PopupCancel),
                        horizontal_space(),
                        button(title_text(TitleLevel::Bald, "Don't save"))
                            .on_press(Message::SaveDone)
                            .style(button::danger),
                        button(title_text(TitleLevel::Bald, "Save"))
                            .on_press(Message::SaveButton)
                            .style(button::success),
                ].spacing(SPACE)])
                .row_spacing(SPACE)
            ).width(Shrink).padding(SPACE).style(|theme| container::background(theme.palette().background)));

            Stack::new()
                .push(main_window)
                .push(opaque(
                    center(Space::new(Fill, Fill)).style(|_| container::background(GRAY)),
                ))
                .push(popup)
                .into()
        } else {
            main_window.into()
        };

        elt //.explain(iced::Color::BLACK)
    }

    fn subscription(&self) -> Subscription<Message> {
        let tab = keyboard::on_key_press(|key, modifiers| match key {
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
        });

        let close_request = window::close_requests().map(|id| Message::CloseRequest(id));

        Subscription::batch([tab, close_request])
    }

    // fn into_save(self) -> AppSave {
    //     AppSave {
    //         recipes: self.recipes.into_iter().map(|v| v.save()).collect(),
    //         known_items: self
    //             .known_items
    //             .into_iter()
    //             .map(|(k, (i, a, b))| {
    //                 (k, (i, a.map(|a| a.into_value()), b.map(|b| b.into_value())))
    //             })
    //             .collect(),
    //         error: self.error,
    //         recipe_uses: self.recipe_uses,
    //         item_stats: self.item_stats,
    //         scale: self.scale,
    //     }
    // }

    fn clone_into_save(&self) -> AppSave {
        AppSave {
            recipes: self.recipes.iter().map(|v| v.clone().save()).collect(),
            known_items: self
                .known_items
                .iter()
                .map(|(k, (i, a, b))| {
                    (
                        k.clone(),
                        (
                            *i,
                            a.as_ref().map(|a| a.clone().into_value()),
                            b.as_ref().map(|b| b.clone().into_value()),
                        ),
                    )
                })
                .collect(),
            error: self.error.clone(),
            recipe_uses: self.recipe_uses.clone(),
            item_stats: self.item_stats.clone(),
            scale: self.scale,
        }
    }

    fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, ImportError> {
        let path = path.as_ref();
        let file = File::open(path).map_err(ImportError::FileError)?;

        let saved: AppSave = rmp_serde::from_read(file).map_err(ImportError::ParseError)?;

        Ok((saved, path).into())
    }

    fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), SaveError> {
        let path = path.as_ref();
        let mut file = File::create(path).map_err(SaveError::FileError)?;

        rmp_serde::encode::write(&mut file, &self.clone_into_save()).map_err(SaveError::WriteError)
    }

    fn save_and_set_err(&mut self, path_buf: PathBuf) -> Task<Message> {
        match self.save(path_buf.clone()) {
            Ok(_) => {
                self.path = Some(path_buf);
                self.unsaved_changes = false;
                return Task::done(Message::SaveDone);
            }
            Err(err) => {
                self.save_error = Some(err);
                return Task::none();
            }
        }
    }

    fn new() -> (Self, Task<Message>) {
        let path: Option<PathBuf> = std::env::args().nth(1).map(Into::into);

        let app = match path {
            Some(path) => App::from_file(path).unwrap_or_else(|err| App {
                import_error: Some(err),
                ..Default::default()
            }),
            None => App::default(),
        };

        (
            app,
            window::get_latest().then(|id| {
                let Some(id) = id else { return Task::none() };
                window::maximize(id, true)
            }),
        )
    }

    fn title(&self) -> String {
        let mut title = "Craft Tree Optimizer".to_string();
        if let Some(file_name) = self
            .path
            .as_ref()
            .and_then(|path| path.file_name())
            .and_then(|name| name.to_str())
        {
            title += &format!(" ({})", file_name);
        }

        if self.unsaved_changes {
            title += " *"
        }
        title
    }
}

#[derive(Debug)]
enum ImportError {
    FileError(std::io::Error),
    ParseError(rmp_serde::decode::Error),
}

impl Display for ImportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ImportError::FileError(err) => write!(f, "{}", err),
            ImportError::ParseError(err) => write!(f, "{}", err),
        }
    }
}

#[derive(Debug)]
enum SaveError {
    FileError(std::io::Error),
    WriteError(rmp_serde::encode::Error),
}

impl Display for SaveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SaveError::FileError(err) => write!(f, "COULD NOT SAVE: {}", err),
            SaveError::WriteError(err) => write!(f, "COULD NOT SAVE: {}", err),
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
struct AppSave {
    recipes: Vec<EditableContentSave>,
    known_items: BTreeMap<Item, (usize, Option<TargetAmount>, Option<TargetAmount>)>, // Quantity, target, raw
    error: Option<String>,

    recipe_uses: Option<Vec<f64>>,
    item_stats: Option<HashMap<Item, (f64, f64)>>, // produced used
    scale: TargetAmount,
}

impl<P: Into<PathBuf>> From<(AppSave, P)> for App {
    fn from((value, path): (AppSave, P)) -> Self {
        Self {
            recipes: value.recipes.into_iter().map(|v| v.into()).collect(),
            known_items: value
                .known_items
                .into_iter()
                .map(|(k, (i, a, b))| {
                    (
                        k,
                        (
                            i,
                            a.map(|a| parsed_input::Content::new(a)),
                            b.map(|b| parsed_input::Content::new(b)),
                        ),
                    )
                })
                .collect(),
            error: value.error,
            recipe_uses: value.recipe_uses,
            item_stats: value.item_stats,
            scale: value.scale,

            unsaved_changes: false,

            path: Some(path.into()),
            import_error: None,
            save_error: None,
            save_popup: None,
        }
    }
}

impl Default for App {
    fn default() -> Self {
        Self {
            recipes: Default::default(),
            known_items: Default::default(),
            error: Default::default(),
            recipe_uses: Default::default(),
            item_stats: Default::default(),
            scale: Default::default(),
            unsaved_changes: false,
            path: Default::default(),
            import_error: Default::default(),
            save_error: Default::default(),
            save_popup: None,
        }
    }
}

// fn file_select_win_builder(title: impl Into<String>, path: impl AsRef<Path>) -> FileDialog {
//     FileDialog::new()
//         .set_directory(path.as_ref().to_path_buf())
//         .set_title(title)
//         .add_filter("Craft tree", &[EXTENSION])
// }

fn file_select_win_builder(
    title: impl Into<String>,
    path: impl AsRef<Path>,
    file_name: Option<impl Into<String>>,
) -> FileDialog {
    let default_path = current_dir().unwrap_or_default();
    let result = FileDialog::new()
        .set_directory({
            let path = path.as_ref().to_path_buf();
            if path.is_dir() {
                path
            } else {
                path.parent()
                    .map(|p| p.to_path_buf())
                    .unwrap_or(default_path)
            }
        })
        .set_title(title)
        .add_filter("Craft tree", &[EXTENSION]);
    match file_name {
        Some(name) => result.set_file_name(name),
        None => result,
    }
}

fn main() -> iced::Result {
    iced::application(App::title, App::update, App::view)
        .subscription(App::subscription)
        .exit_on_close_request(false)
        .run_with(App::new)
}
