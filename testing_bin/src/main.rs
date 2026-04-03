// use std::time::Duration;

// // use std::net::TcpListener;
// // use zero::http::response::Response;
// use rand::Random;
// use zero::http::{
//     Body, Query,
//     request::Method,
//     routing::{ResponseResult, Router},
//     server::HttpServer,
// };
// use zero::{Deserialize, html};

// #[derive(Deserialize, Debug)]
// pub struct Usize {
//     inner: usize,
//     inner2: String,
// }

// #[derive(Deserialize, Debug)]
// pub struct Demo {
//     foo: String,
// }

// pub struct TestStruct<'a, T> {
//     some: &'a Vec<T>,
// }

// pub async fn content(Query(i): Query<Usize>) -> ResponseResult {
//     let i = (i.inner + 1).to_string();
//     // let inner = i.inner2;

//     Ok(html! {
//         BUTTON(
//             id:"output",
//             fx-action:(format!("/content?inner={}&inner2=3",i)),
//             fx-method:"get",
//             fx-trigger:"click",
//             fx-target:"#output",
//             fx-swap:"outerHTML",
//         ){ (i) }
//     }
//     .into())
// }

// pub async fn index() -> ResponseResult {
//     Ok(html! {
//         BUTTON(
//             id:"output",
//             fx-action:"/content?inner=0&inner2=test",
//             fx-method:"get",
//             fx-trigger:"click",
//             fx-target:"#output",
//             fx-swap:"outerHTML",
//         ){ "0" }
//         FORM(fx-action:"/demo", fx-trigger:"submit", fx-method:"post"){
//             INPUT(
//                 type:"text",
//                 name:"foo",
//             ){}
//             INPUT(
//                 type:"text",
//                 name:"bar",
//             ){}
//             BUTTON(type:"submit", value:"Submit"){
//                 "submit"
//             }
//         }
//         SCRIPT( src:"/zero.js" )
//     }
//     .into())
// }

// pub async fn demo(Body(s): Body<Demo>) -> ResponseResult { eprintln!("{:#?}", s);
//     Ok(html! {}.into())
// }

use uuid::UUID;

// #[zero::main]
fn main() -> Result<(), ()> {
    let uuid = UUID::rand_v7().unwrap();
    let uuid = UUID::from_u128(2143632338105341657670967034281843051);

    // let db_file = db::Database::open_db_file("./testing.zero_db").map_err(|_| ())?;
    // let mut am = db::Database::new(&db_file).map_err(|_| ())?;
    // am.append_entry(uuid, "just a test".to_string()).unwrap();
    // eprintln!("{:#?}", am.read_entry::<String>(uuid).unwrap());
    // let entry = am.insert_allocation(64).expect("entry").clone();
    // eprintln!("{:#?}", entry);
    // eprintln!("=====================================");
    // // eprintln!("{:#?}", am);
    // eprintln!("{:#?}", am.remove_allocation(&entry.uuid));
    // eprintln!("=====================================");
    // eprintln!("{:#?}", am.get(&entry.uuid));
    // eprintln!("=====================================");
    // eprintln!("{:#?}", am);
    Ok(())
}
