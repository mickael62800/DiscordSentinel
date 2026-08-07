fn main() {
    // `sqlx::migrate!` embarque les migrations a la compilation : recompiler
    // si une migration Atrium est ajoutee ou modifiee.
    println!("cargo:rerun-if-changed=migrations");
}
