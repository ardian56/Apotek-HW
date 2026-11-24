use crate::{
    db::operations::PurchaseLine,
    models::{Product, Sale, SaleItem},
};

pub struct CheckoutResult {
    pub sale: Sale,
    pub saved_now: bool,
    pub print_error: Option<String>,
}

pub fn finish_sale(
    draft: Option<Sale>,
    saved: Option<Sale>,
    print_requested: bool,
    persist: impl FnOnce(&Sale) -> Result<Sale, String>,
    print: impl FnOnce(&Sale) -> Result<(), String>,
) -> Result<CheckoutResult, String> {
    let (sale, saved_now) = if let Some(sale) = draft {
        crate::db::operations::validate_sale(&sale)?;
        (persist(&sale)?, true)
    } else {
        (saved.ok_or("Tambahkan obat terlebih dahulu")?, false)
    };
    let print_error = if print_requested {
        print(&sale).err()
    } else {
        None
    };
    Ok(CheckoutResult {
        sale,
        saved_now,
        print_error,
    })
}

pub fn invoice_total(subtotal: f64, discount: f64, shipping: f64) -> Result<f64, String> {
    for amount in [subtotal, discount, shipping] {
        crate::db::operations::money(&amount.to_string())?;
    }
    if discount > subtotal {
        return Err("Diskon faktur melebihi subtotal".into());
    }
    crate::db::operations::money(&(subtotal - discount + shipping).to_string())
}

pub fn add_sale_line(
    items: &mut Vec<SaleItem>,
    product: &Product,
    line: &PurchaseLine,
    today: &str,
) -> Result<(), String> {
    line.validate()?;
    if line.product_id != product.id
        || !product.is_active
        || !product
            .units
            .iter()
            .any(|u| u.unit_name == line.unit && u.conversion_qty == line.conversion)
    {
        return Err("Obat atau satuan penjualan tidak valid".into());
    }
    let requested = line
        .qty
        .checked_mul(line.conversion)
        .ok_or("Jumlah terlalu besar")?;
    let in_cart: i64 = items
        .iter()
        .filter(|i| i.product_id == product.id)
        .map(|i| i.qty_in_base as i64)
        .sum();
    if requested as i64 + in_cart > product.sellable_stock_on(today) {
        return Err("Stok layak jual tidak cukup, termasuk obat yang sudah di keranjang".into());
    }
    if line.price * (1.0 - line.discount / 100.0) / (line.conversion as f64) < product.buy_price {
        return Err("Harga setelah diskon di bawah modal. Periksa harga jual.".into());
    }
    if line.discount == 0.0 {
        if let Some(item) = items.iter_mut().find(|i| {
            i.product_id == product.id
                && i.unit_name == line.unit
                && i.conversion_qty == line.conversion
                && i.sell_price == line.price
                && i.discount == 0.0
        }) {
            let qty = item
                .qty
                .checked_add(line.qty)
                .ok_or("Jumlah terlalu besar")?;
            let base = item
                .qty_in_base
                .checked_add(requested)
                .ok_or("Jumlah terlalu besar")?;
            item.qty = qty;
            item.qty_in_base = base;
            item.subtotal = (line.price * qty as f64 * 100.0).round() / 100.0;
            return Ok(());
        }
    }
    items.push(SaleItem {
        id: 0,
        sale_id: 0,
        product_id: product.id,
        product_code: product.code.clone(),
        product_name: product.name.clone(),
        unit_name: line.unit.clone(),
        qty: line.qty,
        conversion_qty: line.conversion,
        qty_in_base: requested,
        sell_price: line.price,
        discount: (line.price * line.qty as f64 * line.discount).round() / 100.0,
        subtotal: line.total(),
        batch_allocations: vec![],
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{ProductBatch, ProductUnit};

    #[test]
    fn checkout_saves_before_print_and_retries_without_saving_again() {
        let events = std::cell::RefCell::new(Vec::new());
        let outcome = finish_sale(
            Some(crate::db::demo::sales()[0].clone()),
            None,
            true,
            |sale| {
                events.borrow_mut().push("save");
                let mut saved = sale.clone();
                saved.items[0].product_name = "Nama dari server".into();
                Ok(saved)
            },
            |sale| {
                assert_eq!(sale.items[0].product_name, "Nama dari server");
                events.borrow_mut().push("print");
                Err("Printer offline".into())
            },
        )
        .unwrap();
        assert_eq!(*events.borrow(), vec!["save", "print"]);
        assert!(outcome.saved_now);
        assert_eq!(outcome.print_error.as_deref(), Some("Printer offline"));
        let retry = finish_sale(
            None,
            Some(outcome.sale),
            true,
            |_| panic!("Printing again must not save another sale"),
            |_| {
                events.borrow_mut().push("retry");
                Ok(())
            },
        )
        .unwrap();
        assert!(!retry.saved_now);
        assert!(retry.print_error.is_none());
        let mut unpaid = crate::db::demo::sales()[0].clone();
        unpaid.paid_amount = 0.0;
        unpaid.change_amount = 0.0;
        assert!(finish_sale(
            Some(unpaid),
            None,
            true,
            |_| panic!("Unpaid cash sale must not be saved"),
            |_| panic!("Unpaid cash sale must not be printed")
        )
        .is_err());
        let only_save = finish_sale(
            Some(crate::db::demo::sales()[0].clone()),
            None,
            false,
            |sale| Ok(sale.clone()),
            |_| panic!("F3 must not print"),
        )
        .unwrap();
        assert!(only_save.saved_now);
        assert!(only_save.print_error.is_none());
    }

    #[test]
    fn scans_merge_and_respect_stock_across_units_and_expiry() {
        assert_eq!(invoice_total(10000.0, 1000.0, 2500.0).unwrap(), 11500.0);
        assert!(invoice_total(100.0, 101.0, 0.0).is_err());
        assert!(invoice_total(100.0, 0.0, f64::NAN).is_err());
        let product = Product {
            id: 1,
            code: "000123".into(),
            name: "Obat".into(),
            category: "Umum".into(),
            supplier_id: None,
            base_unit: "Tablet".into(),
            purchase_unit: "Tablet".into(),
            min_stock: 0,
            buy_price: 100.0,
            is_active: true,
            total_stock: 100,
            units: vec![
                ProductUnit {
                    id: 1,
                    product_id: 1,
                    unit_name: "Tablet".into(),
                    conversion_qty: 1,
                    sell_price: 200.0,
                },
                ProductUnit {
                    id: 2,
                    product_id: 1,
                    unit_name: "Strip".into(),
                    conversion_qty: 10,
                    sell_price: 1800.0,
                },
            ],
            batches: vec![
                ProductBatch {
                    id: 1,
                    product_id: 1,
                    batch_no: "OLD".into(),
                    expired_date: Some("2026-01-01".into()),
                    stock: 88,
                    buy_price: 100.0,
                },
                ProductBatch {
                    id: 2,
                    product_id: 1,
                    batch_no: "NEW".into(),
                    expired_date: Some("2027-01-01".into()),
                    stock: 12,
                    buy_price: 100.0,
                },
            ],
        };
        let mut line = PurchaseLine {
            product_id: 1,
            code: product.code.clone(),
            name: product.name.clone(),
            unit: "Tablet".into(),
            conversion: 1,
            qty: 1,
            price: 200.0,
            discount: 0.0,
            batch: "PENJUALAN".into(),
            expiry: None,
        };
        let mut items = Vec::new();
        for _ in 0..2 {
            add_sale_line(&mut items, &product, &line, "2026-11-22").unwrap();
        }
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].qty, 2);
        assert_eq!(items[0].product_code, "000123");
        assert_eq!(items[0].subtotal, 400.0);
        line.unit = "Strip".into();
        line.conversion = 10;
        line.price = 1800.0;
        add_sale_line(&mut items, &product, &line, "2026-11-22").unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[1].qty_in_base, 10);
        let before = serde_json::to_string(&items).unwrap();
        assert!(add_sale_line(&mut items, &product, &line, "2026-11-22").is_err());
        assert_eq!(serde_json::to_string(&items).unwrap(), before);
        assert!(add_sale_line(&mut Vec::new(), &product, &line, "2027-01-01").is_err());
        line.price = 1.0;
        assert!(add_sale_line(&mut Vec::new(), &product, &line, "2026-11-22").is_err());
        assert_eq!(product.earliest_expiry(), Some("2026-01-01"));
    }
}
