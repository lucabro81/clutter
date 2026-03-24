fn main() {
    let argv: Vec<String> = std::env::args().collect();
    std::process::exit(clutter_cli::run(&argv));
}
