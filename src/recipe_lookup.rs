//! Stores recipes in such a way that we can quickly get all the recipes that allow to make a given item.

use std::collections::HashMap;
use std::hash::Hash;

/// A recipe with ingredients that produces stuff.
#[derive(Debug, Clone)]
pub struct Recipe<T> {
    ingredients: Vec<(T, u8)>,
    products: Vec<(T, u8, f32)>, // Item, nb produced, proba of success.
}

impl<T> Recipe<T> {
    /// Creates a new recipe from an (ingerdient, quantity, probability of success) and a (product, quantity) list.
    /// 
    /// The probability should be between 0 and 1.
    pub fn new(ingredients: Vec<(T, u8)>, products: Vec<(T, u8, f32)>) -> Self {
        Self {
            ingredients,
            products,
        }
    }

    /// Retrieves the ingredients of the recipe.
    pub fn get_ingredients(&self) -> &Vec<(T, u8)> {
        &self.ingredients
    }

    /// Retrieves the products of the recipe.
    pub fn get_products(&self) -> &Vec<(T, u8, f32)> {
        &self.products
    }

    /// Same as [`get_ingredients`](Self::get_ingredients) but mutable.
    pub fn get_mut_ingredients(&mut self) -> &mut Vec<(T, u8)> {
        &mut self.ingredients
    }

    /// Same as [get_products](Self::get_products) but mutable.
    pub fn get_mut_products(&mut self) -> &mut Vec<(T, u8, f32)> {
        &mut self.products
    }

    /// Deconstructs the [`Recipe`] and returns two vectors:
    /// * The first contains the ingreditents (item, quantity)
    /// * The second contains products (item, quantity, probability of success).
    pub fn take(self) -> (Vec<(T, u8)>, Vec<(T, u8, f32)>) {
        (self.ingredients, self.products)
    }
}

/// Struct allowing to store recipes and quickly access those which produce a given item
#[derive(Debug, Clone)]
pub struct RecipeBank<T> {
    table: HashMap<T, Vec<usize>>,
    recipes: Vec<Recipe<T>>,
}

impl<T> RecipeBank<T> {
    /// Creates a recipe bank.
    pub fn new() -> Self {
        Self::default()
    }
}
impl<T: Eq + Hash + Clone> RecipeBank<T> {
    /// Adds a [`Recipe`] to the bank.
    pub fn add(&mut self, recipe: Recipe<T>) {
        // Push the recipe and get the index
        let recipes = &mut self.recipes;
        let id = recipes.len();
        recipes.push(recipe);

        // Add the bindings in the table
        let table = &mut self.table;
        recipes
            .last()
            .unwrap()
            .get_products()
            .iter()
            .map(|(item, _, _)| {
                match table.get_mut(item) {
                    Some(vec) => vec.push(id),
                    None => {
                        table.insert(item.clone(), vec![id]);
                    }
                };
            })
            .collect()
    }
}

impl<T> Default for RecipeBank<T> {
    fn default() -> Self {
        Self {
            table: HashMap::new(),
            recipes: Vec::new(),
        }
    }
}
