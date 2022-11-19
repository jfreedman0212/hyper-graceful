use http_body_util::Full;
use hyper::body::{self, Bytes};
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response};
use hyper_graceful::ConnectionManager;
use std::convert::Infallible;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::signal::ctrl_c;

// Use `curl http://localhost:3000`
// Meanwhile use `ctrl-c` on the process while the request(s) is still pending

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let address = SocketAddr::from(([127, 0, 0, 1], 3000));
    let listener = TcpListener::bind(&address).await.unwrap();
    println!("Listening on: {}", listener.local_addr().unwrap());
    let manager = ConnectionManager::default();

    let svc = service_fn(wait_ten_seconds);

    loop {
        tokio::select! {
            conn = listener.accept() => {
                if let Ok((tcp_stream, socket)) = conn {
                    println!("Receiving connection from: {}", socket);
                    let conn = http1::Builder::new().serve_connection(tcp_stream, svc);
                    let future = manager.manage_connection(conn, move |conn| {
                        println!("Gracefully shutting down connection: {}", socket);
                        conn.graceful_shutdown();
                    });
                    tokio::task::spawn(future);
                }
            },
            _ = ctrl_c() => {
                println!("Received Ctrl-C signal, attempting graceful shutdown");
                break;
            }
        }
    }
    let (graceful, forced) = manager.graceful_shutdown(Duration::from_secs(15)).await;
    println!("Gracefully shutdown {} connections", graceful);
    println!("Forcefully shutdown {} connections", forced);
}

async fn wait_ten_seconds(
    _req: Request<body::Incoming>,
) -> Result<Response<Full<Bytes>>, Infallible> {
    tokio::time::sleep(Duration::from_secs(10)).await;
    Ok(Response::new(Full::new(Bytes::from(
        "I waited 10 seconds for this?",
    ))))
}
