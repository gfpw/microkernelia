use std::process::Command;

fn main() {
    let status = Command::new("cargo")
        .args(["build", "--release", "--target", "x86_64-unknown-none"])
        .status()
        .expect("falló el build del kernel");
    if !status.success() {
        eprintln!("Build fallido");
        std::process::exit(1);
    }
    println!("Build kernel OK");
}
