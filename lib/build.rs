fn main() {
    println!(
        "cargo:rustc-env=AUTHY_API_KEY={}",
        dotenv::var("AUTHY_API_KEY").expect("you must supply an authy api key")
    )
}
