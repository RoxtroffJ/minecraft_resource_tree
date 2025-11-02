use std::collections::{BTreeMap, HashMap};

use good_lp::{Expression, ProblemVariables, Solution, SolverModel, solvers, variable};
use iced::{
    Element,
    Length::*,
    Subscription, Task, keyboard,
    widget::{
        self, Checkbox, Column, Container, Scrollable, Space, button, column, horizontal_rule, row,
        text, text_input,
    },
};
use iced_aw::{ContextMenu, TypedInput};
use minecraft_resource_tree::ui::{
    DisplayFloat, Item, ParseTargetAmountError, SPACE, TargetAmount, TitleLevel, contoured,
    recipe::{self, BuilderState},
    title_text,
};
use more_iced_aw::{
    element_vec,
    grid::{self, Grid},
    parsed_input::{self, ParsedInput},
};

#[derive(Default)]
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
                return Task::none();
            }
            Message::ComputeError(msg) => {
                self.error = Some(msg);
                return Task::none();
            }

            Message::EditScale(v) => {
                self.scale = v;
                return Task::none();
            }

            Message::FocusNext => return widget::focus_next(),
            Message::FocusPrevious => return widget::focus_previous(),
        }
        self.error = None;
        self.recipe_uses = None;
        self.item_stats = None;
        Task::none()
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
            raws_rows.extend(raws.into_iter().map(|(item, cost)| {
                let mut row = element_vec![
                    item.displayer(),
                    ParsedInput::new("Cost of one", cost)
                        .on_input(|v| Message::EditRawCost((**item).clone(), v))
                        .style(parsed_input::danger_on_err(text_input::default))
                ];
                if let Some((prod, uses)) = self.item_stats.as_ref().and_then(|tbl| tbl.get(item)) {
                    row.extend(element_vec![
                        scale_field("Net required", uses - prod),
                        scale_field("Cost", ***cost * (uses - prod)),
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
            all_rows.extend(all.iter().map(|(item, (_, target, raw))| {
                let mut row = element_vec![
                    item.displayer(),
                    Checkbox::new("", target.is_some())
                        .on_toggle(|v| Message::ToggleTarget((*item).clone(), v)),
                    Checkbox::new("", raw.is_some())
                        .on_toggle(|v| Message::ToggleRaw((*item).clone(), v))
                ];
                if let Some((prod, uses)) = self.item_stats.as_ref().and_then(|tbl| tbl.get(item)) {
                    row.extend(element_vec![
                        scale_field("Uses", *uses),
                        scale_field("Produced", *prod),
                        scale_field("Net production", prod - uses),
                    ]);
                }
                row
            }));

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
        let compute_button = column![if self.error.is_some() {
            compute_button.style(button::danger)
        } else {
            compute_button
        }]
        .push_maybe(self.error.as_ref().map(|err| text(err).style(text::danger)));

        let elt: Element<'_, _> = Container::new(column![content, compute_button].spacing(SPACE))
            .padding(SPACE)
            .width(Fill)
            .height(Fill)
            .into();
        elt //.explain(iced::Color::BLACK)
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
