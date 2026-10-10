use std::net::SocketAddr;

/// THE SERVER'S OWN ARGV (v11): the host's configuration — the world it
/// serves, the port it binds, the seed it generates from, and THE REALM'S
/// MODE. This is the one place the mode authority comes from; the join
/// carries no claim of it. Flags (--world/--port/--seed/--creative) sit
/// beside the legacy positional [bind] [seed]; --world names a client
/// save slot (worlds/<slot>), so hosting a friend's world means hosting
/// its directory and its seed.
pub struct ServerArgs {
    pub bind: String,
    pub world_dir: String,
    pub seed: Option<u64>,
    pub creative: bool,
}

impl ServerArgs {
    pub fn parse<I: Iterator<Item = String>>(mut args: I) -> Self {
        let _ = args.next(); // the binary name
        let mut bind: Option<String> = None;
        let mut world_dir: Option<String> = None;
        let mut seed: Option<u64> = None;
        let mut creative = false;
        let mut positional: Vec<String> = Vec::new();
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--world" => world_dir = args.next().or_else(|| world_dir.take()),
                "--port" => {
                    if let Some(port) = args.next().and_then(|p| p.parse::<u16>().ok()) {
                        bind = Some(format!("0.0.0.0:{}", port));
                    }
                }
                "--seed" => seed = args.next().and_then(|s| s.parse().ok()).or(seed),
                "--creative" => creative = true,
                _ => positional.push(arg),
            }
        }
        // legacy positional form: [bind] [seed]
        let legacy_bind = positional.first().cloned();
        let legacy_seed = positional.get(1).and_then(|s| s.parse().ok());
        ServerArgs {
            bind: bind.or(legacy_bind)
                .unwrap_or_else(|| format!("0.0.0.0:{}", lf_server::DEFAULT_PORT)),
            world_dir: world_dir.unwrap_or_else(|| "worlds/server".into()),
            seed: seed.or(legacy_seed),
            creative,
        }
    }
}

fn main() {
    let args = ServerArgs::parse(std::env::args());
    let bind = args.bind.clone();
    // Seed: explicit argv, else load-or-create from the server's world dir so
    // the terrain stays stable across restarts.
    let world_dir = std::path::Path::new(&args.world_dir);
    let storage = lf_voxel::world::WorldStorage::open(world_dir);
    let seed: u64 = args.seed
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
    // Generator version stamp: warn if this world last ran under a different
    // terrain generator (revisited unedited chunks may differ).
    if let Some(old) = lf_worldgen::load_generator_version(world_dir) {
        let current = lf_worldgen::GENERATOR_VERSION;
        if old != current {
            eprintln!("WARNING: world was generated with gen v{}, this build is gen v{}; \
                       revisited unedited chunks may differ from their first visit",
                      old, current);
            let _ = lf_worldgen::save_generator_version(world_dir, current);
        }
    } else {
        let _ = lf_worldgen::save_generator_version(world_dir, lf_worldgen::GENERATOR_VERSION);
    }
    // Load mods/ the same way the client does so mod block ids (>= 100) are
    // registered and pass SetBlock validation (P25).
    let mut mods = lf_modapi::load_mods_dir(std::path::Path::new("mods"));
    let workshop = std::env::var("LOREFORGE_WORKSHOP_DIR").unwrap_or_else(|_| "workshop".into());
    for item in lf_steam::workshop::scan_installed(std::path::Path::new(&workshop)) {
        if let Ok(data) = lf_modapi::load_mod(std::path::Path::new(&item.path)) {
            if mods.iter().any(|m| m.manifest.id == data.manifest.id) {
                continue; // bundled copy wins
            }
            lf_modapi::apply_mod(&data);
            mods.push(data);
            println!("workshop mod loaded: {} ({})", item.title, item.id);
        }
    }
    if mods.is_empty() {
        println!("no mods loaded (mods/ dir missing or empty)");
    } else {
        let names: Vec<&str> = mods.iter().map(|m| m.manifest.id.as_str()).collect();
        println!("loaded {} mod(s): {}", names.len(), names.join(", "));
        if let Some(line) = lf_modapi::smoke_line(&mods) {
            println!("{line}");
        }
    }
    // THE REALM'S MODE (v11): the host decides; every joiner is granted it.
    let mut server = match if args.creative {
        lf_server::Server::start_creative(&bind, seed)
    } else {
        lf_server::Server::start(&bind, seed)
    } {
        Ok(s) => s,
        Err(e) => {
            eprintln!("failed to bind {}: {}", bind, e);
            std::process::exit(1);
        }
    };
    let addr: SocketAddr = server.local_addr();
    println!(
        "LOREFORGE dedicated server listening on {} (seed {}, {} realm)",
        addr, seed,
        if args.creative { "creative" } else { "survival" }
    );
    println!("press Ctrl+C to stop");
    loop {
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
    // server stops on drop
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> ServerArgs {
        let mut v = vec!["loreforge-server".to_string()];
        v.extend(list.iter().map(|s| s.to_string()));
        ServerArgs::parse(v.into_iter())
    }

    /// THE HOST'S WORD IS THE MODE AUTHORITY: --creative starts a creative
    /// realm; the default is the gated survival realm.
    #[test]
    fn the_host_decides_the_realm_s_mode() {
        assert!(!args(&[]).creative, "the default realm is survival (gated)");
        assert!(args(&["--creative"]).creative, "--creative opens the realm");
    }

    /// The client's Start Server flags parse: --world names the slot
    /// directory, --port becomes the bind, --seed pins the world.
    #[test]
    fn the_start_server_flags_parse() {
        let a = args(&["--world", "my-world", "--port", "25599", "--seed", "42"]);
        assert_eq!(a.world_dir, "my-world");
        assert_eq!(a.bind, "0.0.0.0:25599");
        assert_eq!(a.seed, Some(42));
        assert!(!a.creative);
    }

    /// The legacy positional form still binds: [bind] [seed].
    #[test]
    fn the_legacy_positional_form_survives() {
        let a = args(&["127.0.0.1:25565", "12345"]);
        assert_eq!(a.bind, "127.0.0.1:25565");
        assert_eq!(a.seed, Some(12345));
        assert_eq!(a.world_dir, "worlds/server");
    }

    /// A bare start binds the default port with no seed pin (the world
    /// dir's own seed record answers).
    #[test]
    fn a_bare_start_defaults() {
        let a = args(&[]);
        assert_eq!(a.bind, format!("0.0.0.0:{}", lf_server::DEFAULT_PORT));
        assert_eq!(a.seed, None);
    }
}
