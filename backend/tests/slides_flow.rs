mod common;

use common::Peers;
use serde_json::{json, Value};
use wiremock::{
    matchers::{method, path},
    Mock, MockServer, ResponseTemplate,
};

async fn mock_not_owner(auth: &MockServer, user_id: &str) {
    Mock::given(method("GET"))
        .and(path(format!("/auth/users/{user_id}")))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "role": "member" })))
        .mount(auth)
        .await;
}

async fn no_paid_access(payments: &MockServer) {
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "hasAccess": false })))
        .mount(payments)
        .await;
}

async fn no_event_registration(community: &MockServer) {
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "isRegistered": false })))
        .mount(community)
        .await;
}

fn new_deck_body(spoofed_owner_id: &str) -> Value {
    json!({
        "name": "Rust Basics",
        "ownerId": spoofed_owner_id,
        "slides": [
            {"id": "s1", "elements": [{"id": "e1", "type": "text", "content": "Intro slide"}]},
            {"id": "s2", "elements": [{"id": "e2", "type": "text", "content": "Locked slide 2"}]},
            {"id": "s3", "elements": [{"id": "e3", "type": "text", "content": "Locked slide 3"}]},
        ],
        "paywallStartSlideIndex": 1,
    })
}

/// The Java version took the entire request body verbatim, including
/// `ownerId` -- a client could set someone else's id as the new deck's
/// owner. Fixed: always the authenticated caller.
#[tokio::test]
async fn creating_a_deck_always_uses_the_callers_own_identity() {
    let auth = MockServer::start().await;
    let payments = MockServer::start().await;
    let community = MockServer::start().await;
    let app = common::spawn(Peers { auth: &auth.uri(), payments: &payments.uri(), community: &community.uri() }).await;

    let alice = common::object_id_hex(1);
    let eve = common::object_id_hex(2);
    let token = common::mint_token(&alice, "alice", "MEMBER");

    let created = app
        .http
        .post(app.url("/book"))
        .bearer_auth(&token)
        .json(&new_deck_body(&eve))
        .send()
        .await
        .expect("create deck request");
    assert_eq!(created.status(), 201);
    let body: Value = created.json().await.unwrap();
    assert_eq!(body["ownerId"], alice, "ownerId must be the caller, not the spoofed value");
}

/// The Java version had no auth requirement at all on this endpoint and
/// returned full, un-redacted decks -- including unpublished drafts and
/// paywalled content -- for any owner id, to anyone. This is the most
/// serious finding in this domain's port.
#[tokio::test]
async fn owner_deck_listing_requires_being_that_owner_or_platform_owner() {
    let auth = MockServer::start().await;
    let payments = MockServer::start().await;
    let community = MockServer::start().await;
    let app = common::spawn(Peers { auth: &auth.uri(), payments: &payments.uri(), community: &community.uri() }).await;

    let alice = common::object_id_hex(1);
    let eve = common::object_id_hex(2);
    let alice_token = common::mint_token(&alice, "alice", "MEMBER");
    let eve_token = common::mint_token(&eve, "eve", "MEMBER");
    mock_not_owner(&auth, &eve).await;

    app.http
        .post(app.url("/book"))
        .bearer_auth(&alice_token)
        .json(&new_deck_body(&alice))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .expect("create deck should succeed");

    // Anonymous: was fully public before, must now be rejected outright.
    let anon = app.http.get(app.url(&format!("/book/owner/{alice}"))).send().await.expect("owner listing request");
    assert_eq!(anon.status(), 401);

    // An unrelated authenticated member cannot dump alice's decks either.
    let eve_views = app
        .http
        .get(app.url(&format!("/book/owner/{alice}")))
        .bearer_auth(&eve_token)
        .send()
        .await
        .expect("owner listing request");
    assert_eq!(eve_views.status(), 403);

    // alice can see her own decks, with full unredacted content (not the
    // paywall-preview redaction non-owners get).
    let alice_views: Value = app
        .http
        .get(app.url(&format!("/book/owner/{alice}")))
        .bearer_auth(&alice_token)
        .send()
        .await
        .expect("owner listing request")
        .json()
        .await
        .expect("owner listing body");
    let decks = alice_views.as_array().expect("array response");
    assert_eq!(decks.len(), 1);
    let slide2_elements = decks[0]["slides"][1]["elements"].as_array().expect("elements array");
    assert_eq!(slide2_elements.len(), 1, "the real owner should see full, unredacted slide content");
}

#[tokio::test]
async fn non_entitled_readers_see_paywall_redacted_slides() {
    let auth = MockServer::start().await;
    let payments = MockServer::start().await;
    let community = MockServer::start().await;
    no_paid_access(&payments).await;
    no_event_registration(&community).await;
    let app = common::spawn(Peers { auth: &auth.uri(), payments: &payments.uri(), community: &community.uri() }).await;

    let alice = common::object_id_hex(1);
    let eve = common::object_id_hex(2);
    let alice_token = common::mint_token(&alice, "alice", "MEMBER");
    let eve_token = common::mint_token(&eve, "eve", "MEMBER");
    mock_not_owner(&auth, &eve).await;

    let deck: Value = app
        .http
        .post(app.url("/book"))
        .bearer_auth(&alice_token)
        .json(&new_deck_body(&alice))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let deck_id = deck["id"].as_str().unwrap();

    // eve, with no payment and no event registration, only sees slide 1
    // in full; slides 2-3 (>= paywallStartSlideIndex) are redacted.
    let eve_view: Value = app
        .http
        .get(app.url(&format!("/book/{deck_id}")))
        .bearer_auth(&eve_token)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let slides = eve_view["slides"].as_array().unwrap();
    assert_eq!(slides[0]["elements"].as_array().unwrap().len(), 1);
    assert_eq!(slides[1]["elements"].as_array().unwrap().len(), 0, "slide 2 should be redacted");
    assert_eq!(slides[2]["elements"].as_array().unwrap().len(), 0, "slide 3 should be redacted");

    // Fully anonymous gets the same redaction.
    let anon_view: Value = app.http.get(app.url(&format!("/book/{deck_id}"))).send().await.unwrap().json().await.unwrap();
    let anon_slides = anon_view["slides"].as_array().unwrap();
    assert_eq!(anon_slides[1]["elements"].as_array().unwrap().len(), 0);
}

/// Cross-service check: a completed payment recorded in `payments` should
/// unlock the full deck here, without this service duplicating any
/// payment records locally.
#[tokio::test]
async fn a_completed_payment_unlocks_the_full_deck() {
    let auth = MockServer::start().await;
    let payments = MockServer::start().await;
    let community = MockServer::start().await;
    no_event_registration(&community).await;
    let app = common::spawn(Peers { auth: &auth.uri(), payments: &payments.uri(), community: &community.uri() }).await;

    let alice = common::object_id_hex(1);
    let eve = common::object_id_hex(2);
    let alice_token = common::mint_token(&alice, "alice", "MEMBER");
    let eve_token = common::mint_token(&eve, "eve", "MEMBER");
    mock_not_owner(&auth, &eve).await;

    let deck: Value = app
        .http
        .post(app.url("/book"))
        .bearer_auth(&alice_token)
        .json(&new_deck_body(&alice))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let deck_id = deck["id"].as_str().unwrap();

    Mock::given(method("GET"))
        .and(path(format!("/payments/access/slide-deck/{deck_id}/user/{eve}")))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "hasAccess": true })))
        .mount(&payments)
        .await;

    let eve_view: Value = app
        .http
        .get(app.url(&format!("/book/{deck_id}")))
        .bearer_auth(&eve_token)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let slides = eve_view["slides"].as_array().unwrap();
    assert_eq!(slides[1]["elements"].as_array().unwrap().len(), 1, "a paid-up user should see the full deck, no redaction");
}

#[tokio::test]
async fn editor_prefs_default_and_update_with_padding_clamped() {
    let auth = MockServer::start().await;
    let payments = MockServer::start().await;
    let community = MockServer::start().await;
    let app = common::spawn(Peers { auth: &auth.uri(), payments: &payments.uri(), community: &community.uri() }).await;
    let alice = common::object_id_hex(1);
    let token = common::mint_token(&alice, "alice", "MEMBER");

    let deck: Value = app
        .http
        .post(app.url("/book"))
        .bearer_auth(&token)
        .json(&new_deck_body(&alice))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let deck_id = deck["id"].as_str().unwrap();

    let default_prefs: Value = app
        .http
        .get(app.url(&format!("/book/{deck_id}/editor-prefs")))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(default_prefs["gridDensity"], "medium");
    assert_eq!(default_prefs["paddingTop"], 64);

    let updated: Value = app
        .http
        .put(app.url(&format!("/book/{deck_id}/editor-prefs")))
        .bearer_auth(&token)
        .json(&json!({ "showGrid": true, "gridDensity": "bogus", "paddingTop": 9999, "paddingLeft": -50 }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(updated["showGrid"], true);
    assert_eq!(updated["gridDensity"], "medium", "an invalid density should fall back to the default");
    assert_eq!(updated["paddingTop"], 400, "padding should be clamped to the max");
    assert_eq!(updated["paddingLeft"], 0, "padding should be clamped to the min");
}

#[tokio::test]
async fn poll_votes_aggregate_and_report_the_callers_own_choice() {
    let auth = MockServer::start().await;
    let payments = MockServer::start().await;
    let community = MockServer::start().await;
    let app = common::spawn(Peers { auth: &auth.uri(), payments: &payments.uri(), community: &community.uri() }).await;
    let alice = common::object_id_hex(1);
    let eve = common::object_id_hex(2);
    let alice_token = common::mint_token(&alice, "alice", "MEMBER");
    let eve_token = common::mint_token(&eve, "eve", "MEMBER");

    let deck: Value = app
        .http
        .post(app.url("/book"))
        .bearer_auth(&alice_token)
        .json(&new_deck_body(&alice))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let deck_id = deck["id"].as_str().unwrap();

    let vote: Value = app
        .http
        .post(app.url(&format!("/book/{deck_id}/slides/s1/elements/e1/poll-votes")))
        .bearer_auth(&eve_token)
        .json(&json!({ "choiceIds": ["opt-a", "opt-a", "opt-b"] }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(vote["totalVoters"], 1);
    assert_eq!(vote["myChoiceIds"].as_array().unwrap().len(), 2, "duplicate choice should be de-duped");

    // Anonymous aggregate view: totals visible, but no myChoiceIds.
    let anon_view: Value = app
        .http
        .get(app.url(&format!("/book/{deck_id}/slides/s1/elements/e1/poll-votes")))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(anon_view["totalVoters"], 1);
    assert!(anon_view["myChoiceIds"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn anonymous_book_sessions_are_created_and_persisted() {
    let auth = MockServer::start().await;
    let payments = MockServer::start().await;
    let community = MockServer::start().await;
    let app = common::spawn(Peers { auth: &auth.uri(), payments: &payments.uri(), community: &community.uri() }).await;
    let alice = common::object_id_hex(1);
    let token = common::mint_token(&alice, "alice", "MEMBER");

    let deck: Value = app
        .http
        .post(app.url("/book"))
        .bearer_auth(&token)
        .json(&new_deck_body(&alice))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let deck_id = deck["id"].as_str().unwrap();

    let first: Value = app
        .http
        .get(app.url(&format!("/book/{deck_id}/session?anonSessionId=anon-xyz")))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(first["anonSessionId"], "anon-xyz");
    let session_id = first["id"].as_str().unwrap().to_string();

    app.http
        .post(app.url(&format!("/book/{deck_id}/session?anonSessionId=anon-xyz")))
        .json(&json!({ "currentSlideId": "s2", "history": ["s1", "s2"] }))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .expect("session upsert should succeed");

    let second: Value = app
        .http
        .get(app.url(&format!("/book/{deck_id}/session?anonSessionId=anon-xyz")))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(second["id"], session_id, "same anon session should be reused, not recreated");
    assert_eq!(second["currentSlideId"], "s2");
}
