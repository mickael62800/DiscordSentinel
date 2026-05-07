use super::*;

fn ch(id: u64) -> ChannelId {
    ChannelId::new(id)
}

const PUBLIC: u64 = 1000;
const PRIVATE: u64 = 1001;
const TEMP_A: u64 = 2000;
const TEMP_B: u64 = 2001;

#[test]
fn self_mute_in_same_channel_skips_handlers() {
    // Cas regression : user est dans TEMP_A, se mute. old == new.
    // Doit retourner None pour que l ownership ne soit pas degrade.
    let result = should_run_leave_handlers(
        Some(ch(TEMP_A)),
        Some(ch(TEMP_A)),
        ch(PUBLIC),
        ch(PRIVATE),
    );
    assert_eq!(result, None);
}

#[test]
fn user_leaves_voice_completely_runs_handlers() {
    // User etait dans TEMP_A, quitte tout (new = None).
    let result = should_run_leave_handlers(
        Some(ch(TEMP_A)),
        None,
        ch(PUBLIC),
        ch(PRIVATE),
    );
    assert_eq!(result, Some(ch(TEMP_A)));
}

#[test]
fn user_moves_to_another_channel_runs_handlers_on_old() {
    // User etait dans TEMP_A, va dans TEMP_B. Le handler doit tourner sur TEMP_A.
    let result = should_run_leave_handlers(
        Some(ch(TEMP_A)),
        Some(ch(TEMP_B)),
        ch(PUBLIC),
        ch(PRIVATE),
    );
    assert_eq!(result, Some(ch(TEMP_A)));
}

#[test]
fn user_joins_first_time_skips_handlers() {
    // Pas de old_channel : le user vient juste de join. Rien a faire cote leave.
    let result = should_run_leave_handlers(
        None,
        Some(ch(TEMP_A)),
        ch(PUBLIC),
        ch(PRIVATE),
    );
    assert_eq!(result, None);
}

#[test]
fn leaving_public_creator_skips_handlers() {
    // Public creator est un lobby : pas d ownership a transferer.
    let result = should_run_leave_handlers(
        Some(ch(PUBLIC)),
        Some(ch(TEMP_A)),
        ch(PUBLIC),
        ch(PRIVATE),
    );
    assert_eq!(result, None);
}

#[test]
fn leaving_private_creator_skips_handlers() {
    // Private creator est un lobby : pas d ownership a transferer.
    let result = should_run_leave_handlers(
        Some(ch(PRIVATE)),
        Some(ch(TEMP_A)),
        ch(PUBLIC),
        ch(PRIVATE),
    );
    assert_eq!(result, None);
}

#[test]
fn self_deaf_in_same_temp_channel_skips_handlers() {
    // Equivalent du self_mute : self_deaf seul ne doit pas declencher.
    let result = should_run_leave_handlers(
        Some(ch(TEMP_A)),
        Some(ch(TEMP_A)),
        ch(PUBLIC),
        ch(PRIVATE),
    );
    assert_eq!(result, None);
}

#[test]
fn moving_from_temp_to_creator_lobby_still_runs_handlers() {
    // User etait dans TEMP_A, retourne dans le lobby public (cas
    // "je veux creer un nouveau salon"). Le handler doit tourner sur
    // TEMP_A pour transferer l ownership ou supprimer si vide.
    let result = should_run_leave_handlers(
        Some(ch(TEMP_A)),
        Some(ch(PUBLIC)),
        ch(PUBLIC),
        ch(PRIVATE),
    );
    assert_eq!(result, Some(ch(TEMP_A)));
}
