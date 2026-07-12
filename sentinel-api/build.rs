//! Build script : force la recompilation du crate (et donc la ré-évaluation de
//! `sqlx::migrate!("./migrations")`) dès que le dossier `migrations/` change.
//!
//! Sans ça, `migrate!` (proc-macro) embarque les migrations à la compilation
//! mais ne peut pas déclarer lui-même de dépendance ; avec un cache cargo
//! persistant (cf. Dockerfile `--mount=type=cache,target=/app/target`), un
//! rebuild peut ré-embarquer un ANCIEN jeu de migrations → panique
//! `VersionMissing` au démarrage. Ce `rerun-if-changed` corrige la cause.

fn main() {
    println!("cargo:rerun-if-changed=migrations");
}
