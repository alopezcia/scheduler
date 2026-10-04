// Fuerza la recompilación al añadir/cambiar migraciones: `sqlx::migrate!` las embebe
// en tiempo de compilación y cargo no detecta ficheros nuevos por sí solo.
fn main() {
    println!("cargo:rerun-if-changed=migrations");
}
