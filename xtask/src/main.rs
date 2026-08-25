fn main() {
    println!("LOREFORGE xtask automation running...");
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 {
        match args[1].as_str() {
            "vistest" => println!("Running vistest headless scenes..."),
            "package" => println!("Packaging release artifacts..."),
            _ => println!("Unknown task: {}", args[1]),
        }
    }
}
