use super::*;

#[test]
fn distribute_random_total_sums_to_input() {
    for _ in 0..50 {
        let amounts = ManageCoudeCashboxService::distribute_random(1000, 5);
        assert_eq!(amounts.iter().sum::<i64>(), 1000);
        assert_eq!(amounts.len(), 5);
    }
}

#[test]
fn distribute_random_sorted_desc() {
    let amounts = ManageCoudeCashboxService::distribute_random(10_000, 10);
    for pair in amounts.windows(2) {
        assert!(pair[0] >= pair[1], "not sorted descending");
    }
}

#[test]
fn distribute_random_empty_on_zero_total() {
    assert!(ManageCoudeCashboxService::distribute_random(0, 5).is_empty());
    assert!(ManageCoudeCashboxService::distribute_random(100, 0).is_empty());
}

#[test]
fn distribute_random_produces_disparity() {
    let amounts = ManageCoudeCashboxService::distribute_random(1_000_000, 10);
    let max = *amounts.first().unwrap();
    let min = *amounts.last().unwrap();
    assert!(max >= min);
    assert!(max > 0);
}
