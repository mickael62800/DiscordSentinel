//! Domaine security : timers de protection auto (phase 5F).
//! Pour l'instant : kick des users en quarantaine dont le captcha
//! a expire.

pub mod expire_lockdown;
pub mod expire_slowmode;
pub mod kick_expired_quarantine;
