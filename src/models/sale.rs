use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sale {
    pub id: i32,
    pub invoice_no: String,
    pub sale_date: String,
    pub payment_type: String, // "TUNAI" atau "KREDIT"
    pub due_date: Option<String>,
    pub customer_id: Option<i32>,
    pub doctor_id: Option<i32>,
    pub operator_id: i32,
    pub subtotal: f64,
    pub discount: f64,
    pub grand_total: f64,
    pub paid_amount: f64,
    pub change_amount: f64,
    pub total_profit: f64,
    pub items: Vec<SaleItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaleItem {
    pub id: i32,
    pub sale_id: i32,
    pub product_id: i32,
    pub product_code: String,
    pub product_name: String,
    pub unit_name: String,
    pub qty: i32,
    pub conversion_qty: i32,
    pub qty_in_base: i32,
    pub sell_price: f64,
    pub discount: f64,
    pub subtotal: f64,
    pub batch_allocations: Vec<SaleItemBatchDeduction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaleItemBatchDeduction {
    pub batch_id: i32,
    pub batch_no: String,
    pub qty_deducted: i32,
    pub buy_price_snapshot: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingSale {
    pub id: i32,
    pub operator_id: i32,
    pub customer_name: String,
    pub cart_json: String,
    pub created_at: String,
}
