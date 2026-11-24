use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Product {
    pub id: i32,
    pub code: String,
    pub name: String,
    pub category: String,
    pub supplier_id: Option<i32>,
    #[serde(default)]
    pub purchase_unit: String,
    pub base_unit: String,
    pub min_stock: i32,
    pub buy_price: f64,
    pub is_active: bool,
    pub total_stock: i32, // Jumlah total stok dari semua batch
    pub units: Vec<ProductUnit>,
    pub batches: Vec<ProductBatch>,
}

impl Product {
    pub fn earliest_expiry(&self) -> Option<&str> {
        self.batches
            .iter()
            .filter(|b| b.stock > 0)
            .filter_map(|b| b.expired_date.as_deref())
            .min()
    }

    pub fn sellable_stock_on(&self, today: &str) -> i64 {
        self.batches
            .iter()
            .filter(|b| {
                b.stock > 0
                    && b.expired_date
                        .as_deref()
                        .map(|date| date > today)
                        .unwrap_or(true)
            })
            .map(|b| b.stock as i64)
            .sum()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductUnit {
    pub id: i32,
    pub product_id: i32,
    pub unit_name: String,
    pub conversion_qty: i32,
    pub sell_price: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductBatch {
    pub id: i32,
    pub product_id: i32,
    pub batch_no: String,
    pub expired_date: Option<String>, // "YYYY-MM-DD"
    pub stock: i32,                   // dalam base_unit
    pub buy_price: f64,
}
