use std::net::SocketAddr;

fn main() {
    let bind = std::env::args().nth(1).unwrap_or_else(|| format!("0.0.0.0:{}", lf_server::DEFAULT_PORT));
    let seed: u64 = std::env::args().nth(2).and_then(|s| s.parse().ok()).unwrap_or(12345);
    let mut server = match lf_server::Server::start(&bind, seed) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("failed to bind {}: {}", bind, e);
            std::process::exit(1);
        }
    };
    let addr: SocketAddr = server.local_addr();
    println!("LOREFORGE dedicated server listening on {} (seed {})", addr, seed);
    println!("press Ctrl+C to stop");
    loop {
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
    // server stops on drop
}
