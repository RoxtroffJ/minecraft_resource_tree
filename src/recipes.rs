//! Everything about [Recipe]s.


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