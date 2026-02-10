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

// #[zero::main]
fn main() -> Result<(), ()> {
    let mut am = db::page_table::AddressMap::new("testing.db")?;
    println!("init");
    am.insert_allocation(64);
    am.insert_allocation(64);
    Ok(())
}
