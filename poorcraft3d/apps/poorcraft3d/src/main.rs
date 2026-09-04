//! POORCRAFT 3D executable — a stub that can answer "who am I" (P3D-001).
//! The first real runtime loop arrives with P3D-005.

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("--identity") | None => {
            println!("{}", pc3d_core::identity_block());
            if args.len() == 1 {
                println!("\n(no runtime yet — P3D-005 builds the first empty-world loop)");
            }
        }
        Some(other) => {
            eprintln!("unknown argument: {other}\nusage: poorcraft3d [--identity]");
            std::process::exit(2);
        }
    }
}
