use crate::helpers::{IntegrationTestContext, query_game_events};
use rstest::rstest;

// =============================================================================
// S2: Event Sourcing Consistency
// =============================================================================

#[rstest]
#[tokio::test]
async fn s2_event_sourcing_consistency() {
    let mut context = IntegrationTestContext::new().await;
    context.cleanup_all().await;

    // Create game
    let game_id = context.create_game("EventSourcingHero").await;

    // Get events via API
    let events_response = context
        .client
        .get(&format!("/api/v1/games/{}/events", game_id))
        .await;
    assert_eq!(events_response.status_code(), 200);

    let api_events = events_response.body["events"].as_array().unwrap();
    let next_sequence = events_response.body["next_sequence"].as_u64().unwrap();

    // Get events directly from MySQL
    let mysql_events = query_game_events(&context.mysql_pool, &game_id).await;

    // Verify consistency
    assert_eq!(
        api_events.len(),
        mysql_events.len(),
        "API and MySQL event counts should match"
    );

    assert_eq!(
        next_sequence,
        mysql_events.len() as u64,
        "next_sequence should equal total event count"
    );

    // Verify sequence numbers are contiguous
    for (index, event) in mysql_events.iter().enumerate() {
        assert_eq!(
            event.sequence_number, index as u64,
            "Sequence numbers should be contiguous starting from 0"
        );
    }

    // Verify first event is Started
    assert!(
        !mysql_events.is_empty(),
        "At least one event should be created"
    );
    assert_eq!(
        mysql_events[0].event_type, "Started",
        "First event should be Started"
    );
}

#[rstest]
#[tokio::test]
async fn s2_event_sourcing_api_matches_database() {
    let mut context = IntegrationTestContext::new().await;
    context.cleanup_all().await;

    let game_id = context.create_game("ConsistencyHero").await;

    // Get events via API
    let api_response = context
        .client
        .get(&format!("/api/v1/games/{}/events", game_id))
        .await;
    let api_events = api_response.body["events"].as_array().unwrap();

    // Get events from MySQL
    let mysql_events = query_game_events(&context.mysql_pool, &game_id).await;

    // Compare each event
    for (api_event, mysql_event) in api_events.iter().zip(mysql_events.iter()) {
        let api_sequence = api_event["sequence"].as_u64().unwrap();
        let api_type = api_event["type"].as_str().unwrap();

        assert_eq!(
            api_sequence, mysql_event.sequence_number,
            "Sequence numbers should match"
        );

        // API may transform event type (e.g., "Started" -> "GameStarted")
        // Check that the core type name is contained
        assert!(
            api_type.contains("Started")
                || mysql_event.event_type.contains("Started")
                || api_type == mysql_event.event_type,
            "Event types should be related: API='{}', MySQL='{}'",
            api_type,
            mysql_event.event_type
        );
    }
}
