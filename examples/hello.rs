use hyper::{Server, StatusCode};
use routerman::{
    method::get,
    request::Request,
    router::{Router, RouterBuilder},
};

fn router() -> RouterBuilder {
    Router::builder()
        .route("/hello", get(|_req: Request| async { "Hello, World!" }))
        .default_route(|_req: Request| async { (StatusCode::IM_A_TEAPOT, "Want some tea?") })
}

#[tokio::main]
async fn main() {
    let router = router().build();

    let addr = &([127, 0, 0, 1], 8080).into();
    let server = Server::bind(addr).serve(router);

    println!("Server listening on {}", addr);
    if let Err(err) = server.await {
        eprintln!("Error: {}", err);
    }
}
