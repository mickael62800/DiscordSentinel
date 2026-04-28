// Enums du domain regroupes par bounded context.
pub mod community;
pub mod coude;
pub mod moderation;
pub mod system;

// Re-exports plats (chaque enum etait avant accessible via
// `crate::domain::value_objects::Type` -- on garde le meme racourci ici).
