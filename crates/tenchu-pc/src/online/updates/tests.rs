use super::*;

#[test]
fn selects_newer_stable_game_versions_among_artwork_and_development_releases() {
    let releases = br#"[
        {"tag_name":"artwork-v99","draft":false,"prerelease":false},
        {"tag_name":"head","draft":false,"prerelease":true},
        {"tag_name":"v2.0.0","draft":true,"prerelease":false},
        {"tag_name":"v1.0.0","draft":false,"prerelease":true},
        {"tag_name":"v0.99.0-beta.1","draft":false,"prerelease":false},
        {"tag_name":"v0.9.9","draft":false,"prerelease":false},
        {"tag_name":"v0.10.0","draft":false,"prerelease":false}
    ]"#;
    assert_eq!(
        newer_release(releases, "0.9.8").unwrap().unwrap().tag,
        "v0.10.0"
    );
    assert!(newer_release(releases, "0.10.0").unwrap().is_none());
    assert!(newer_release(releases, "0.11.0").unwrap().is_none());
    assert_eq!(
        newer_release(releases, "0.10.0-beta.1")
            .unwrap()
            .unwrap()
            .tag,
        "v0.10.0"
    );
    assert!(newer_release(b"{\"message\":\"rate limited\"}", "0.1.0").is_err());
    assert!(newer_release(&vec![b' '; RESPONSE_LIMIT + 1], "0.1.0").is_err());
}

#[test]
fn concurrent_hosts_check_once_and_do_not_retry_failures() {
    let updates = Updates::default();
    updates.start(false);
    assert!(matches!(updates.status(), Status::NotChecked));
    let started = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..16).map(|_| scope.spawn(|| updates.begin())).collect();
        handles
            .into_iter()
            .map(|handle| usize::from(handle.join().unwrap()))
            .sum::<usize>()
    });
    assert_eq!(started, 1);
    assert!(matches!(updates.status(), Status::Checking));
    assert!(!updates.begin());
    *updates.status.lock().unwrap() = Status::Failed("offline".into());
    assert!(!updates.begin());
    *updates.status.lock().unwrap() = Status::Current;
    assert!(!updates.begin());
}

#[test]
#[ignore = "explicit live GitHub smoke check"]
fn live_release_check() {
    let bytes = fetch().unwrap();
    let release = newer_release(&bytes, "0.0.0").unwrap().unwrap();
    println!("Latest stable release: {}", release.tag);
    assert!(release.url().starts_with(super::super::REPOSITORY));
}
