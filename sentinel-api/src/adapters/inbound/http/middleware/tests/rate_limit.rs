use super::*;
use std::net::Ipv4Addr;

use axum::body::Body;
use axum::http::Request;

// ── client_ip ──

fn make_req(headers: &[(&str, &str)]) -> Request<Body> {
    let mut b = Request::builder();
    for (k, v) in headers {
        b = b.header(*k, *v);
    }
    b.body(Body::empty()).unwrap()
}

#[test]
fn client_ip_uses_rightmost_trusted_hop() {
    // TRUST_PROXY_HOPS defaut = 1 : on prend l'IP inseree par le dernier proxy
    // de confiance (la plus a DROITE), non falsifiable par le client.
    let req = make_req(&[("x-forwarded-for", "1.1.1.1, 2.2.2.2, 3.3.3.3")]);
    let ip = client_ip(&req, IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)));
    assert_eq!(ip, IpAddr::V4(Ipv4Addr::new(3, 3, 3, 3)));
}

#[test]
fn client_ip_falls_back_to_x_real_ip_when_no_xff() {
    let req = make_req(&[("x-real-ip", "192.168.1.5")]);
    let ip = client_ip(&req, IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)));
    assert_eq!(ip, IpAddr::V4(Ipv4Addr::new(192, 168, 1, 5)));
}

#[test]
fn client_ip_falls_back_to_socket_when_no_headers() {
    let req = make_req(&[]);
    let ip = client_ip(&req, IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)));
    assert_eq!(ip, IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)));
}

#[test]
fn client_ip_invalid_xff_falls_through_to_fallback() {
    // Le hop de confiance (le plus a droite) n'est pas parseable -> fallback
    // socket (on ne "remonte" pas vers la gauche, qui est controlee par le client).
    let req = make_req(&[("x-forwarded-for", "10.0.0.1, not-an-ip")]);
    let ip = client_ip(&req, IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)));
    assert_eq!(ip, IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)));
}

#[test]
fn client_ip_xff_trims_whitespace() {
    let req = make_req(&[("x-forwarded-for", "  10.0.0.42  ")]);
    let ip = client_ip(&req, IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)));
    assert_eq!(ip, IpAddr::V4(Ipv4Addr::new(10, 0, 0, 42)));
}

#[test]
fn client_ip_invalid_x_real_ip_falls_through() {
    let req = make_req(&[("x-real-ip", "garbage")]);
    let ip = client_ip(&req, IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)));
    assert_eq!(ip, IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)));
}

#[test]
fn client_ip_prefers_xff_over_real_ip() {
    let req = make_req(&[("x-forwarded-for", "1.2.3.4"), ("x-real-ip", "5.6.7.8")]);
    let ip = client_ip(&req, IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)));
    assert_eq!(ip, IpAddr::V4(Ipv4Addr::new(1, 2, 3, 4)));
}

// ── RateLimiter::check ──

fn ip(s: &str) -> IpAddr {
    s.parse().unwrap()
}

#[tokio::test]
async fn check_allows_up_to_burst_then_blocks() {
    let rl = RateLimiter::new(2); // burst = 2*10 = 20
    for _ in 0..20 {
        assert!(rl.check(ip("10.0.0.1")).await);
    }
    // 21eme refuse (pas encore refill).
    assert!(!rl.check(ip("10.0.0.1")).await);
}

#[tokio::test]
async fn check_independent_per_ip() {
    let rl = RateLimiter::new(1); // burst = 10
    for _ in 0..10 {
        assert!(rl.check(ip("10.0.0.1")).await);
    }
    assert!(!rl.check(ip("10.0.0.1")).await);
    // Une autre IP a son propre bucket.
    assert!(rl.check(ip("10.0.0.2")).await);
}

#[tokio::test]
async fn check_refills_over_time() {
    let rl = RateLimiter::new(100); // burst = 1000
                                    // Consomme 999 tokens rapidement.
    for _ in 0..999 {
        rl.check(ip("10.0.0.1")).await;
    }
    // On a encore au moins 1, puis on doit pouvoir refill.
    assert!(rl.check(ip("10.0.0.1")).await);
    // Attend ~100ms, attend refill ~10 tokens.
    tokio::time::sleep(std::time::Duration::from_millis(150)).await;
    assert!(rl.check(ip("10.0.0.1")).await);
}

#[tokio::test]
async fn cleanup_does_not_panic() {
    let rl = RateLimiter::new(5);
    rl.check(ip("10.0.0.1")).await;
    rl.cleanup().await;
}
