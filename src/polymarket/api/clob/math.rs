use super::{
    schemas::{AccumulatedOrder, Order, OrderBookData},
    typedefs::Side,
};

pub trait ClobPrecision {
    fn round_normal(self, decimals: u32) -> Self;

    fn round_down(self, decimals: u32) -> Self;

    fn round_up(self, decimals: u32) -> Self;

    fn decimal_places(self) -> u32;
}

impl ClobPrecision for f64 {
    fn round_normal(self, decimals: u32) -> Self {
        if self.decimal_places() <= decimals {
            return self;
        }
        let factor = 10_f64.powi(decimals as i32);
        (self * factor).round() / factor
    }

    fn round_down(self, decimals: u32) -> Self {
        if self.decimal_places() <= decimals {
            return self;
        }
        let factor = 10_f64.powi(decimals as i32);
        (self * factor).floor() / factor
    }

    fn round_up(self, decimals: u32) -> Self {
        if self.decimal_places() <= decimals {
            return self;
        }
        let factor = 10_f64.powi(decimals as i32);
        (self * factor).ceil() / factor
    }

    fn decimal_places(self) -> u32 {
        if self.fract() == 0.0 {
            return 0;
        }

        let num_str = self.to_string();
        if let Some(pos) = num_str.find('.') {
            return (num_str.len() - pos - 1) as u32;
        }

        0
    }
}

pub fn adjust_amount(mut amount: f64, allowed_decimals: u32) -> f64 {
    if amount.decimal_places() > allowed_decimals {
        amount = amount.round_up(allowed_decimals + 4);
        if amount.decimal_places() > allowed_decimals {
            amount = amount.round_down(allowed_decimals);
        }
    }
    amount
}

fn sort_orders(side: Side, orders: &mut [Order]) -> Vec<Order> {
    match side {
        Side::Buy => {
            orders.sort_by(|a, b| b.price.partial_cmp(&a.price).unwrap());
        }
        Side::Sell => {
            orders.sort_by(|a, b| a.price.partial_cmp(&b.price).unwrap());
        }
    }
    orders.to_vec()
}

fn calculate_accumulated_values(orders: &[Order]) -> Vec<AccumulatedOrder> {
    let mut accumulated_orders = vec![];

    for current_order in orders.iter() {
        if current_order.size > 0.0 {
            let value = (current_order.size * current_order.price).round_normal(2);

            let previous_order = accumulated_orders.last();

            let net_value = (value
                + previous_order.map_or(0.0, |prev: &AccumulatedOrder| prev.net_value))
            .round_normal(2);
            let net_size = (current_order.size + previous_order.map_or(0.0, |prev| prev.net_size))
                .round_normal(2);

            accumulated_orders.push(AccumulatedOrder {
                price: current_order.price,
                size: current_order.size,
                value,
                net_value,
                net_size,
            });
        }
    }

    accumulated_orders
}

fn find_buy_price(asks: &[AccumulatedOrder], unit_amount: f64, slippage: f64) -> f64 {
    let adjusted_value = unit_amount * (1.0 + slippage / 100.0);

    asks.iter()
        .find(|order| order.net_value >= adjusted_value)
        .map_or(0.0, |order| order.price)
}

fn find_sell_price(bids: &[AccumulatedOrder], usdc_amount: f64, slippage: f64) -> f64 {
    let adjusted_size = usdc_amount * (1.0 + slippage / 100.0);

    bids.iter()
        .find(|order| order.net_size >= adjusted_size)
        .map_or(0.0, |order| order.price)
}

pub fn calculate_market_price(
    side: Side,
    mut book_data: OrderBookData,
    unit_amount: f64,
    slippage: Option<f64>,
) -> f64 {
    let slippage = slippage.unwrap_or(0.1);

    let sorted_asks = sort_orders(Side::Sell, &mut book_data.asks);
    let accumulated_asks = calculate_accumulated_values(&sorted_asks);

    let sorted_bids = sort_orders(Side::Buy, &mut book_data.bids);
    let accumulated_bids = calculate_accumulated_values(&sorted_bids);

    match side {
        Side::Buy => find_buy_price(&accumulated_asks, unit_amount, slippage),
        Side::Sell => find_sell_price(&accumulated_bids, unit_amount, slippage),
    }
}
