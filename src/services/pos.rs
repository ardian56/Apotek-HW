use crate::models::{Product, ProductUnit, SaleItem};

pub struct CartItem {
    pub product: Product,
    pub selected_unit: ProductUnit,
    pub qty: i32,
    pub discount_percent: f64,
}

impl CartItem {
    pub fn new(product: Product, selected_unit: ProductUnit, qty: i32) -> Self {
        Self {
            product,
            selected_unit,
            qty,
            discount_percent: 0.0,
        }
    }

    pub fn qty_in_base(&self) -> i32 {
        self.qty * self.selected_unit.conversion_qty
    }

    pub fn subtotal(&self) -> f64 {
        let gross = self.qty as f64 * self.selected_unit.sell_price;
        let disc = (gross * self.discount_percent) / 100.0;
        (gross - disc).max(0.0)
    }

    /// Validasi apakah harga jual per base unit lebih kecil dari harga modal
    pub fn is_selling_at_loss(&self) -> bool {
        let price_per_base = self.selected_unit.sell_price / (self.selected_unit.conversion_qty as f64);
        price_per_base < self.product.buy_price
    }

    pub fn to_sale_item(&self) -> SaleItem {
        SaleItem {
            id: 0,
            sale_id: 0,
            product_id: self.product.id,
            product_code: self.product.code.clone(),
            product_name: self.product.name.clone(),
            unit_name: self.selected_unit.unit_name.clone(),
            qty: self.qty,
            conversion_qty: self.selected_unit.conversion_qty,
            qty_in_base: self.qty_in_base(),
            sell_price: self.selected_unit.sell_price,
            discount: (self.qty as f64 * self.selected_unit.sell_price * self.discount_percent) / 100.0,
            subtotal: self.subtotal(),
            batch_allocations: Vec::new(),
        }
    }
}
