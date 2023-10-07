use std::{env, fs};
use std::collections::{HashMap, VecDeque};
use std::fs::OpenOptions;
use std::io::Write;
use std::str::FromStr;
use std::sync::Mutex;

use log::Log;
use matching_engine::common::utils::Sigma;
use matching_engine::matchers::fifo_matcher::FIFOMatcher;
use matching_engine::matchers::matcher::Matcher;
use matching_engine::matchers::prorata_matcher::ProrataMatcher;
use matching_engine::model::domain::{OrderBook, OrderBookKey, OrderSingle};
use matching_engine::model::domain::Side::{Buy, Sell};
use rocket::FromForm;
use rocket::http::Status;
use serde::{Deserialize, Serialize};
use serde_json::{from_str, to_string};

pub static LOG_FILE: &str = "web/logs/web.log";
const ORDER_BOOK_FILE: &str = "orderbook.json";


#[derive(Debug, Clone, Serialize, FromForm)]
pub struct Order {
    symbol: String,
    qty: u32,
    price: f64,
    side: String,
    order_type: String,
    cl_ord_id: String,
    format: String,
}

impl Order {
    pub fn symbol(&self) -> &String {
        &self.symbol
    }

    pub fn qty(&self) -> u32 {
        self.qty
    }

    pub fn price(&self) -> f64 {
        self.price
    }

    pub fn side(&self) -> &String {
        &self.side
    }

    pub fn order_type(&self) -> &String {
        &self.order_type
    }

    pub fn cl_ord_id(&self) -> &String {
        &self.cl_ord_id
    }

    pub fn format(&self) -> &String {
        &self.format
    }
}



#[derive(Serialize, Debug, Deserialize)]
pub struct OB {
    pub buy_orders: HashMap<String, VecDeque<OrderSingle>>,
    pub sell_orders: HashMap<String, VecDeque<OrderSingle>>,
}

impl OB {
    ///creates an [`OB`] instance from [`OrderBook]
    pub fn from(order_book: &OrderBook) -> Self {
        let buy = order_book.get_orders_for(Buy);
        let sell = order_book.get_orders_for(Sell);
        let mut buy_orders = HashMap::new();
        add_string_keys(&mut buy_orders, &buy);
        let mut sell_orders = HashMap::new();
        add_string_keys(&mut sell_orders, &sell);
        OB {
            buy_orders,
            sell_orders,
        }
    }
 /// converts an [`OB`]  instance to  an [`OrderBook`]  instance
    pub fn to(ob: &OB) -> OrderBook {
        let mut buy = HashMap::new();
        add_order_book_keys(&mut buy, &ob.buy_orders);
        let mut sell = HashMap::new();
        add_order_book_keys(&mut sell, &ob.sell_orders);
        OrderBook::new(buy, sell)
    }
}

fn add_order_book_keys(target: &mut HashMap<OrderBookKey, VecDeque<OrderSingle>>, source: &HashMap<String, VecDeque<OrderSingle>>) {
    for (key, val) in source {
        let v: Vec<&str> = key.split('_').collect();
        let symbol = v[0].to_string();
        let price = f64::from_str(v[1]).unwrap();
        let key = OrderBookKey::new(price, symbol);
        target.insert(key, val.clone());
    }
}

///Creates an  [`OrderBook`] instance from a file having the json string representation of the order book,
/// optionally adding the supplied [`OrderSingle`] to the order book
pub fn get_order_book_from_file(order_single: Option<OrderSingle>) -> OrderBook {
    let content = match fs::read_to_string(ORDER_BOOK_FILE) {
        Ok(data) => data,
        Err(_) => String::new(),
    };
    let mut order_book = OrderBook::default();

    if !content.is_empty() {
        let ob: OB = from_str(&content).unwrap();
        order_book = OB::to(&ob);
        if let Some(order) = order_single {
            order_book.add_order_to_order_book(order);
        }
        persist_order_book(&ob);
    } else {
        if let Some(order) = order_single {
            order_book.add_order_to_order_book(order);
        }
        let ob = OB::from(&order_book);
        persist_order_book(&ob);
    }
    order_book.clone()
}
fn add_string_keys(target: &mut HashMap<String, VecDeque<OrderSingle>>, source: &HashMap<OrderBookKey, VecDeque<OrderSingle>>) {
    for (key, val) in source {
        let mut k = key.symbol().to_string();
        k.push('_');
        k.push_str(key.price().to_string().as_str());
        target.insert(k, val.clone());
    }
}
///Resets the [`OrderBook`] to an empty order book
pub fn reset() -> Result<String, Status> {
    let mut message = String::new();
    if let Err(err) = fs::remove_file(ORDER_BOOK_FILE) {
        eprintln!("Error deleting file: {}", err);
    } else {
        message.push_str("Order book deleted successfully")
    }

    Ok(message)
}

///Persists the [`OrderBook`] as a json string to file
pub fn persist_order_book(ob: &OB) {
    let mut file = OpenOptions::new()
        .write(true)
        .create(true) // Create the file if it doesn't exist
        .truncate(true) //overwrite content
        .open(ORDER_BOOK_FILE).unwrap();
    let content = to_string(&ob).unwrap();

    let mut file_lock = Mutex::new(file);
    {
        let mut file = file_lock.lock().unwrap();
        file.write_all(content.as_bytes()).expect("Error writing");
        file.flush().expect("error flushing");
    }
}


///Returns the appropriate [`Matcher`] implementation based on the supplied algo parameter
pub fn get_matcher() -> Box<dyn Matcher>{
    let algo = match env::var("ALGO") {
        Ok(algo) => algo,
        Err(_) => "FIFO".to_string()
    };

    if algo == "FIFO" {
        Box::new(FIFOMatcher)
    }else{
        Box::new(ProrataMatcher)
    }

}


#[cfg(test)]
mod tests {
    use matching_engine::common::utils::create_order_book;
    use matching_engine::common::utils::read_input;
    use matching_engine::model::domain::Side::{Buy, Sell};
    use serde_json::{from_str, to_string};

    use crate::OB;

    #[test]
    fn test_to() {
        let content = std::fs::read_to_string("test_data/ob.json").unwrap();
        let ob: OB = from_str(&content).unwrap();
        let mut order_book = OB::to(&ob);
        assert_eq!(order_book.get_orders_for(Buy).len(), 1);
        assert_eq!(order_book.get_orders_for(Sell).len(), 1);
        order_book.pretty_print_self();
    }

    #[test]
    fn test_from() {
        let mut order_book = create_order_book(read_input("test_data/orders.txt"));
        assert_eq!(order_book.get_orders_for(Buy).len(), 1);
        assert_eq!(order_book.get_orders_for(Sell).len(), 1);
        let ob = OB::from(&order_book);
        let json = to_string(&ob).unwrap();
        assert!(!json.is_empty());
        println!("{}", json);
    }
}



