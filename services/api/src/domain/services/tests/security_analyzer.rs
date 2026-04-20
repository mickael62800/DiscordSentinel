use super::*;

#[test]
fn levenshtein_identical() {
    assert_eq!(levenshtein("hello", "hello"), 0);
}

#[test]
fn levenshtein_one_diff() {
    assert_eq!(levenshtein("cat", "bat"), 1);
}

#[test]
fn levenshtein_empty() {
    assert_eq!(levenshtein("", "abc"), 3);
}

#[test]
fn similar_names_found() {
    let names = vec!["raider1".into(), "raider2".into(), "alice".into()];
    assert!(has_similar_usernames(&names, 2));
}

#[test]
fn similar_names_not_found() {
    let names = vec!["alice".into(), "bob".into(), "charlie".into()];
    assert!(!has_similar_usernames(&names, 1));
}

#[test]
fn clustered_creation() {
    assert!(are_creations_clustered(&[1000, 1500, 2000], 3600));
    assert!(!are_creations_clustered(&[1000, 100000], 3600));
}

#[test]
fn raid_analysis_scoring() {
    let joins = vec![
        JoinInfo { username: "raid1".into(), has_avatar: false, account_created_timestamp: 1000 },
        JoinInfo { username: "raid2".into(), has_avatar: false, account_created_timestamp: 1500 },
    ];
    let analysis = analyze_joins(&joins, 2, 3600);
    assert!(analysis.score >= 60);
}

#[test]
fn alt_detection_similar_name() {
    let bans = vec![BannedUserInfo { username: "raider".into(), account_created_timestamp: 5000 }];
    let result = check_alt_account("ra1der", 99999, &bans, 2, 3600);
    assert!(result.similar_to_banned.is_some());
}

#[test]
fn alt_detection_no_match() {
    let bans = vec![BannedUserInfo { username: "bob".into(), account_created_timestamp: 5000 }];
    let result = check_alt_account("alice", 99999, &bans, 1, 3600);
    assert!(!result.is_suspicious());
}

#[test]
fn suspicious_account_young() {
    let now = chrono::Utc::now().timestamp();
    assert!(is_account_suspicious(now - 3600, 86400));
}

#[test]
fn suspicious_account_old() {
    let now = chrono::Utc::now().timestamp();
    assert!(!is_account_suspicious(now - 100000, 86400));
}

#[test]
fn raid_analysis_single_join_no_raid() {
    let joins = vec![JoinInfo { username: "solo".into(), has_avatar: true, account_created_timestamp: 1000 }];
    assert_eq!(analyze_joins(&joins, 2, 3600).score, 0);
}

#[test]
fn similar_names_single_name() {
    assert!(!has_similar_usernames(&["only".into()], 2));
}

#[test]
fn alt_detection_creation_near() {
    let bans = vec![BannedUserInfo { username: "zzzzz".into(), account_created_timestamp: 5000 }];
    let result = check_alt_account("completely_different", 5500, &bans, 1, 3600);
    assert!(result.creation_near_banned.is_some());
}

#[test]
fn suspicious_account_future_timestamp() {
    let future = chrono::Utc::now().timestamp() + 3600;
    assert!(is_account_suspicious(future, 86400));
}
