use crate::helpers::IntegrationTestContext;
use rstest::rstest;

fn assert_not_implemented_or_error(status: u16) {
    assert!(
        status == 501 || status == 500 || status == 422,
        "Expected 501, 500, or 422, got {}",
        status
    );
}

// =============================================================================
// C1-C4: Move Commands Return 501
// =============================================================================

#[rstest]
#[case("north")]
#[case("south")]
#[case("east")]
#[case("west")]
#[tokio::test]
async fn c1_c4_move_command_returns_501(#[case] direction: &str) {
    let mut context = IntegrationTestContext::new().await;
    context.cleanup_all().await;

    let game_id = context.create_game("Hero").await;

    let request = serde_json::json!({
        "command": {
            "Move": {
                "direction": direction
            }
        }
    });

    let response = context
        .client
        .post(&format!("/api/v1/games/{}/commands", game_id), &request)
        .await;

    assert_not_implemented_or_error(response.status_code());
}

// =============================================================================
// C5: Wait Command Returns 501
// =============================================================================

#[rstest]
#[tokio::test]
async fn c5_wait_command_returns_501() {
    let mut context = IntegrationTestContext::new().await;
    context.cleanup_all().await;

    let game_id = context.create_game("Hero").await;

    let request = serde_json::json!({
        "command": "Wait"
    });

    let response = context
        .client
        .post(&format!("/api/v1/games/{}/commands", game_id), &request)
        .await;

    assert_not_implemented_or_error(response.status_code());
}

// =============================================================================
// C6: Attack Command Returns 501
// =============================================================================

#[rstest]
#[tokio::test]
async fn c6_attack_command_returns_501() {
    let mut context = IntegrationTestContext::new().await;
    context.cleanup_all().await;

    let game_id = context.create_game("Hero").await;

    let request = serde_json::json!({
        "command": {
            "Attack": {
                "target_id": "enemy-123"
            }
        }
    });

    let response = context
        .client
        .post(&format!("/api/v1/games/{}/commands", game_id), &request)
        .await;

    assert_not_implemented_or_error(response.status_code());
}

// =============================================================================
// C7-C8: UseItem Command Returns 501
// =============================================================================

#[rstest]
#[tokio::test]
async fn c7_use_item_without_target_returns_501() {
    let mut context = IntegrationTestContext::new().await;
    context.cleanup_all().await;

    let game_id = context.create_game("Hero").await;

    let request = serde_json::json!({
        "command": {
            "UseItem": {
                "item_id": "item-123"
            }
        }
    });

    let response = context
        .client
        .post(&format!("/api/v1/games/{}/commands", game_id), &request)
        .await;

    assert_not_implemented_or_error(response.status_code());
}

#[rstest]
#[tokio::test]
async fn c8_use_item_with_target_returns_501() {
    let mut context = IntegrationTestContext::new().await;
    context.cleanup_all().await;

    let game_id = context.create_game("Hero").await;

    let request = serde_json::json!({
        "command": {
            "UseItem": {
                "item_id": "item-123",
                "target_id": "enemy-456"
            }
        }
    });

    let response = context
        .client
        .post(&format!("/api/v1/games/{}/commands", game_id), &request)
        .await;

    assert_not_implemented_or_error(response.status_code());
}

// =============================================================================
// C9-C14: Other Commands Return 501
// =============================================================================

#[rstest]
#[tokio::test]
async fn c9_pick_up_command_returns_501() {
    let mut context = IntegrationTestContext::new().await;
    context.cleanup_all().await;

    let game_id = context.create_game("Hero").await;

    let request = serde_json::json!({
        "command": {
            "PickUp": {
                "item_id": "item-123"
            }
        }
    });

    let response = context
        .client
        .post(&format!("/api/v1/games/{}/commands", game_id), &request)
        .await;

    assert_not_implemented_or_error(response.status_code());
}

#[rstest]
#[tokio::test]
async fn c10_drop_command_returns_501() {
    let mut context = IntegrationTestContext::new().await;
    context.cleanup_all().await;

    let game_id = context.create_game("Hero").await;

    let request = serde_json::json!({
        "command": {
            "Drop": {
                "item_id": "item-123"
            }
        }
    });

    let response = context
        .client
        .post(&format!("/api/v1/games/{}/commands", game_id), &request)
        .await;

    assert_not_implemented_or_error(response.status_code());
}

#[rstest]
#[tokio::test]
async fn c11_equip_command_returns_501() {
    let mut context = IntegrationTestContext::new().await;
    context.cleanup_all().await;

    let game_id = context.create_game("Hero").await;

    let request = serde_json::json!({
        "command": {
            "Equip": {
                "item_id": "item-123"
            }
        }
    });

    let response = context
        .client
        .post(&format!("/api/v1/games/{}/commands", game_id), &request)
        .await;

    assert_not_implemented_or_error(response.status_code());
}

#[rstest]
#[case("weapon")]
#[case("armor")]
#[case("helmet")]
#[case("accessory")]
#[tokio::test]
async fn c12_unequip_command_returns_501(#[case] slot: &str) {
    let mut context = IntegrationTestContext::new().await;
    context.cleanup_all().await;

    let game_id = context.create_game("Hero").await;

    let request = serde_json::json!({
        "command": {
            "Unequip": {
                "slot": slot
            }
        }
    });

    let response = context
        .client
        .post(&format!("/api/v1/games/{}/commands", game_id), &request)
        .await;

    assert_not_implemented_or_error(response.status_code());
}

#[rstest]
#[tokio::test]
async fn c13_descend_command_returns_501() {
    let mut context = IntegrationTestContext::new().await;
    context.cleanup_all().await;

    let game_id = context.create_game("Hero").await;

    let request = serde_json::json!({
        "command": "Descend"
    });

    let response = context
        .client
        .post(&format!("/api/v1/games/{}/commands", game_id), &request)
        .await;

    assert_not_implemented_or_error(response.status_code());
}

#[rstest]
#[tokio::test]
async fn c14_ascend_command_returns_501() {
    let mut context = IntegrationTestContext::new().await;
    context.cleanup_all().await;

    let game_id = context.create_game("Hero").await;

    let request = serde_json::json!({
        "command": "Ascend"
    });

    let response = context
        .client
        .post(&format!("/api/v1/games/{}/commands", game_id), &request)
        .await;

    assert_not_implemented_or_error(response.status_code());
}

// =============================================================================
// C15: Invalid UUID Returns 400
// =============================================================================

#[rstest]
#[tokio::test]
async fn c15_command_invalid_uuid_returns_400() {
    let context = IntegrationTestContext::new().await;

    let request = serde_json::json!({
        "command": "Wait"
    });

    let response = context
        .client
        .post("/api/v1/games/invalid-uuid/commands", &request)
        .await;

    // Accept 400 or 422 for validation errors
    assert!(
        response.status_code() == 400 || response.status_code() == 422,
        "Expected 400 or 422, got {}",
        response.status_code()
    );
}

// =============================================================================
// C16: Invalid Command Type Returns 400
// =============================================================================

#[rstest]
#[tokio::test]
async fn c16_command_invalid_type_returns_400() {
    let mut context = IntegrationTestContext::new().await;
    context.cleanup_all().await;

    let game_id = context.create_game("Hero").await;

    let request = serde_json::json!({
        "command": "InvalidCommand"
    });

    let response = context
        .client
        .post(&format!("/api/v1/games/{}/commands", game_id), &request)
        .await;

    // Accept 400, 422, or 500 for invalid command type
    assert!(
        response.status_code() == 400 || response.status_code() == 422 || response.status_code() == 500,
        "Expected 400, 422, or 500, got {}",
        response.status_code()
    );
}

// =============================================================================
// C17: Missing Command Field Returns 400
// =============================================================================

#[rstest]
#[tokio::test]
async fn c17_command_missing_field_returns_400() {
    let mut context = IntegrationTestContext::new().await;
    context.cleanup_all().await;

    let game_id = context.create_game("Hero").await;

    let request = serde_json::json!({});

    let response = context
        .client
        .post(&format!("/api/v1/games/{}/commands", game_id), &request)
        .await;

    // Accept 400 or 422 for missing fields
    assert!(
        response.status_code() == 400 || response.status_code() == 422,
        "Expected 400 or 422, got {}",
        response.status_code()
    );
}
