use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Product {
    pub id: i32,
    pub code: String,
    pub name: String,
    pub category: String,
    pub supplier_id: Option<i32>,
    pub base_unit: String,
    pub min_stock: i32,
    pub buy_price: f64,
    pub is_active: bool,
    pub total_stock: i32, // Jumlah total stok dari semua batch
    pub units: Vec<ProductUnit>,
    pub batches: Vec<ProductBatch>,
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
    pub stock: i32,                  // dalam base_unit
    pub buy_price: f64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExpiryStatus {
    Normal,
    ExpiringSoon, // <= 30 hari
    Expired,
}

impl ProductBatch {
    pub fn check_expiry_status(&self, today: &str) -> ExpiryStatus {
        if let Some(exp) = &self.expired_date {
            if exp <= today {
                return ExpiryStatus::Expired;
            }
            // Simple check approx <= 30 days
            // Real logic can use chrono NaiveDate
        }
        ExpiryStatus::Normal
    }
}
