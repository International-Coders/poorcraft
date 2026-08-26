use std::net::SocketAddr;

fn main() {
    let bind = std::env::args().nth(1).unwrap_or_else(|| format!("0.0.0.0:{}", lf_server::DEFAULT_PORT));
    // Seed: explicit argv, else load-or-create from the server's world dir so
    // the terrain stays stable across restarts.
    let world_dir = std::path::Path::new("worlds/server");
    let storage = lf_voxel::world::WorldStorage::open(world_dir);
    let seed: u64 = std::env::args().nth(2).and_then(|s| s.parse().ok())
        .or_else(|| storage.load_seed())
        .unwrap_or_else(|| {
            // fresh random seed (time ^ pid, mixed)
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos() as u64).unwrap_or(42);
            let mut z = nanos ^ (std::process::id() as u64).rotate_left(32);
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
            z ^ (z >> 31)
        });
    let _ = storage.save_seed(seed);
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
