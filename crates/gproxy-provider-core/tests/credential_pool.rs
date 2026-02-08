use std::time::Duration;

use gproxy_provider_core::credential::ApiKeyCredential;
use gproxy_provider_core::{
    Credential, CredentialPool, CredentialState, Event, EventHub, OperationalEvent,
    UnavailableReason,
};
use tokio::time::timeout;

#[tokio::test]
async fn unavailable_recovers_via_queue() {
    let hub = EventHub::new(16);
    let mut rx = hub.subscribe();
    let pool = CredentialPool::new(hub.clone());

    pool.insert(
        "test",
        1,
        Credential::Custom(ApiKeyCredential {
            api_key: "k".to_string(),
        }),
    )
    .await;

    pool.mark_unavailable(1, Duration::from_millis(50), UnavailableReason::RateLimit)
        .await;

    let ev = timeout(Duration::from_millis(200), rx.recv())
        .await
        .unwrap()
        .unwrap();
    assert!(matches!(
        ev,
        Event::Operational(OperationalEvent::UnavailableStart(_))
    ));

    let ev = timeout(Duration::from_millis(500), rx.recv())
        .await
        .unwrap()
        .unwrap();
    assert!(matches!(
        ev,
        Event::Operational(OperationalEvent::UnavailableEnd(_))
    ));

    let state = pool.state(1).await.unwrap();
    assert!(matches!(state, CredentialState::Active));
}

#[tokio::test]
async fn stale_queue_entry_does_not_recover_early() {
    let hub = EventHub::new(32);
    let pool = CredentialPool::new(hub);

    pool.insert(
        "test",
        1,
        Credential::Custom(ApiKeyCredential {
            api_key: "k".to_string(),
        }),
    )
    .await;

    pool.mark_unavailable(1, Duration::from_millis(80), UnavailableReason::Timeout)
        .await;
    pool.mark_unavailable(1, Duration::from_millis(200), UnavailableReason::Timeout)
        .await;

    tokio::time::sleep(Duration::from_millis(120)).await;
    let state = pool.state(1).await.unwrap();
    assert!(matches!(state, CredentialState::Unavailable { .. }));

    tokio::time::sleep(Duration::from_millis(150)).await;
    let state = pool.state(1).await.unwrap();
    assert!(matches!(state, CredentialState::Active));
}
