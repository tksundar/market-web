#[macro_use]
extern crate rocket;


use std::env;
use matching_engine::common::utils::create_order_from_string;
use matching_engine::model::domain::{Fill, OrderSingle, OrderType, Side};
use rocket::*;
use rocket::data::ByteUnit;
use rocket::form::Form;
use rocket::fs::FileServer;
use rocket::get;
use rocket::http::Status;
use serde_json::to_string;
use market_web::{  get_matcher, get_order_book_from_file, OB, Order, persist_order_book};


#[get("/")]
fn index() -> &'static str {

    "Please fill the form"
}

#[get("/order_book/<format>")]

fn get_order_book(format: &str) -> String{
    debug!("Order Book requested");
    let mut order_book = get_order_book_from_file(None);
    let mut matcher = get_matcher(&"FIFO".to_string());
    let fills =  matcher.match_order_book(&mut order_book);
    let ob: OB = OB::from(&order_book);
    let mut res = String::new();
    if format == "json" {
        if !fills.is_empty(){
            res.push_str("Fills\n");
            res.push_str(to_string(&fills).unwrap().as_str());

        }
        res.push_str(to_string(&ob).unwrap().as_str());

    }else{
        if !fills.is_empty(){
            res.push_str(Fill::pretty_print(&fills).as_str());

        }
        res.push_str(order_book.pretty_print_self().as_str());

    }
    persist_order_book(&ob);
    res
}

#[post("/order_entry", data = "<order_form>")]
fn add_order(order_form: Form<Order> ) -> Result<String, Status> {
    let order: Order = order_form.into_inner();
    let order_single = OrderSingle::new(order.qty(),
                                        order.symbol().clone(),
                                        order.price(),
                                        Side::from(order.side()),
                                        OrderType::from(order.order_type()),
                                        order.cl_ord_id().clone());

    let algo = match env::var("ALGO") {
        Ok(algo) => algo,
        Err(_) => "FIFO".to_string()
    };
    let mut matcher = get_matcher(&algo);
    let mut order_book = get_order_book_from_file(Some(order_single));
    let fills = matcher.match_order_book(&mut order_book);
    let mut fill_str = to_string(&fills).unwrap();
    if order.format() == "pretty" {
        fill_str = Fill::pretty_print(&fills);
    }
    let ob = OB::from(&order_book);
    persist_order_book(&ob);
    Ok(fill_str)
}



#[get("/reset")]
fn reset() -> Result<String,Status>{
    market_web::reset()
}


#[post("/upload", data = "<data>")]
async fn upload<'a>(data: Data<'a>) -> Result<String, Status> {
    let ds = data.open(ByteUnit::Kilobyte(1024));
    let val = ds.into_string().await.unwrap().value;
    let raw_data: Vec<&str> = val.split("\n").collect();
    let mut orders = vec![];
    for line in raw_data {
        let temp = line.trim_end_matches('\r');
        let trimmed: Vec<&str> = temp.split(' ').collect();
        if trimmed.len() == 5 {
            orders.push(temp);
        }
    }
    let mut order_book = get_order_book_from_file(None);
    for line in orders {
        let order = create_order_from_string(line.to_string());
        order_book.add_order_to_order_book(order);
    }
    let algo = match env::var("ALGO") {
        Ok(algo) => algo,
        Err(_) => "FIFO".to_string()
    };
    let mut matcher = get_matcher(&algo);
    let fills = matcher.match_order_book(&mut order_book);
    let mut res = String::new();
    res.push_str(Fill::pretty_print(&fills).as_str());
    let ob: OB = OB::from(&order_book);
    persist_order_book(&ob);
    let s = order_book.pretty_print_self();
    res.push_str(s.as_str());
    Ok(res)
}

#[catch(404)]
fn not_found(req: &Request) -> String {
    format!("The requested path {} , is not available ", req.uri())
}

#[catch(422)]
fn malformed(req: &Request) -> String {
    // println!("{:#?}",req);
    format!("the submitted data could not be processed! '{:#?}'", req)
}

//noinspection RsMainFunctionNotFound
#[launch]
fn rocket() -> _ {
    rocket::build().
        register("/", catchers![malformed, not_found]).
        mount("/", routes![index,add_order,get_order_book,reset,upload]).
        mount("/", FileServer::from("static/"))
}


