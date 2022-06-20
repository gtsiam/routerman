use hyper::{Server, StatusCode};
use routerman::{method::get, HyperRouter as Router};

type Request = hyper::Request<hyper::Body>;

fn router() -> Router {
    Router::builder()
        .route("/hello", get(|_req: Request| async { "Hello, World!" }))
        .default_route(|_req: Request| async { StatusCode::NOT_FOUND })
        .build()
}

#[tokio::main]
async fn main() {
    let router = router();

    let addr = &([127, 0, 0, 1], 8080).into();
    let server = Server::bind(addr).serve(router);

    println!("Server listening on {}", addr);
    if let Err(err) = server.await {
        eprintln!("Error: {}", err);
    }
}
