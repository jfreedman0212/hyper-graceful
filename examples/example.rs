use hyper::server::conn::Http;
use hyper::service::service_fn;
use hyper::{Body, Response};
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
    let http = Http::new();
    let manager = ConnectionManager::default();

    let svc = service_fn(|_| async {
        for _ in 0..10 {
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
        Ok::<_, Infallible>(Response::new(Body::from("Hello, World!")))
    });

    loop {
        tokio::select! {
            conn = listener.accept() => {
                if let Ok((tcp_stream, socket)) = conn {
                    println!("Receiving connection from: {}", socket);
                    let future = manager.manage_connection(http.serve_connection(tcp_stream, svc), move |conn| {
                        println!("Gracefully shutting down connection: {}", socket);
                        conn.graceful_shutdown();
                    });
                    tokio::task::spawn(future);
                }
            },
            _ = ctrl_c() => {
                break;
            }
        }
    }
    let (graceful, forced) = manager.graceful_shutdown(Duration::from_secs(5)).await;
    println!("Gracefully shutdown {} connections", graceful);
    println!("Forcefully shutdown {} connections", forced);
}
