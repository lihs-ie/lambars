use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use roguelike_api::dto::command::{CommandRequest, DirectionRequest, EquipmentSlotRequest};
use roguelike_api::dto::request::{
    CreateGameRequest, EndGameRequest, ExecuteCommandRequest, GetEventsParams, GetLeaderboardParams,
};

fn benchmark_request_parsing(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("request_parsing");

    let create_game_json = r#"{"player_name": "Hero", "seed": 12345}"#;
    group.bench_function("CreateGameRequest_with_seed", |bencher| {
        bencher.iter(|| {
            serde_json::from_str::<CreateGameRequest>(black_box(create_game_json)).unwrap()
        })
    });

    let create_game_minimal = r#"{"player_name": "Hero"}"#;
    group.bench_function("CreateGameRequest_minimal", |bencher| {
        bencher.iter(|| {
            serde_json::from_str::<CreateGameRequest>(black_box(create_game_minimal)).unwrap()
        })
    });

    let end_game_json = r#"{"outcome": "victory"}"#;
    group.bench_function("EndGameRequest", |bencher| {
        bencher.iter(|| serde_json::from_str::<EndGameRequest>(black_box(end_game_json)).unwrap())
    });

    let move_command = r#"{"command": {"type": "move", "direction": "north"}}"#;
    group.bench_function("ExecuteCommandRequest_move", |bencher| {
        bencher.iter(|| {
            serde_json::from_str::<ExecuteCommandRequest>(black_box(move_command)).unwrap()
        })
    });

    let attack_command = r#"{"command": {"type": "attack", "target_id": "enemy-12345678-abcd-1234-efgh-567890abcdef"}}"#;
    group.bench_function("ExecuteCommandRequest_attack", |bencher| {
        bencher.iter(|| {
            serde_json::from_str::<ExecuteCommandRequest>(black_box(attack_command)).unwrap()
        })
    });

    let use_item_command =
        r#"{"command": {"type": "use_item", "item_id": "potion-001", "target_id": "player-001"}}"#;
    group.bench_function("ExecuteCommandRequest_use_item", |bencher| {
        bencher.iter(|| {
            serde_json::from_str::<ExecuteCommandRequest>(black_box(use_item_command)).unwrap()
        })
    });

    let wait_command = r#"{"command": {"type": "wait"}}"#;
    group.bench_function("ExecuteCommandRequest_wait", |bencher| {
        bencher.iter(|| {
            serde_json::from_str::<ExecuteCommandRequest>(black_box(wait_command)).unwrap()
        })
    });

    let equip_command = r#"{"command": {"type": "equip", "item_id": "sword-001"}}"#;
    group.bench_function("ExecuteCommandRequest_equip", |bencher| {
        bencher.iter(|| {
            serde_json::from_str::<ExecuteCommandRequest>(black_box(equip_command)).unwrap()
        })
    });

    let unequip_command = r#"{"command": {"type": "unequip", "slot": "weapon"}}"#;
    group.bench_function("ExecuteCommandRequest_unequip", |bencher| {
        bencher.iter(|| {
            serde_json::from_str::<ExecuteCommandRequest>(black_box(unequip_command)).unwrap()
        })
    });

    group.finish();
}

fn benchmark_command_serialization(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("command_serialization");

    let commands = [
        (
            "move",
            CommandRequest::Move {
                direction: DirectionRequest::North,
            },
        ),
        (
            "attack",
            CommandRequest::Attack {
                target_id: "enemy-12345678".to_string(),
            },
        ),
        (
            "use_item",
            CommandRequest::UseItem {
                item_id: "potion-001".to_string(),
                target_id: Some("player-001".to_string()),
            },
        ),
        ("wait", CommandRequest::Wait),
        ("descend", CommandRequest::Descend),
        ("ascend", CommandRequest::Ascend),
        (
            "pick_up",
            CommandRequest::PickUp {
                item_id: "item-001".to_string(),
            },
        ),
        (
            "drop",
            CommandRequest::Drop {
                item_id: "item-001".to_string(),
            },
        ),
        (
            "equip",
            CommandRequest::Equip {
                item_id: "sword-001".to_string(),
            },
        ),
        (
            "unequip",
            CommandRequest::Unequip {
                slot: EquipmentSlotRequest::Weapon,
            },
        ),
    ];

    for (name, command) in commands {
        let request = ExecuteCommandRequest {
            command: command.clone(),
        };
        group.bench_with_input(
            BenchmarkId::new("ExecuteCommandRequest", name),
            &request,
            |bencher, request| bencher.iter(|| serde_json::to_string(black_box(request)).unwrap()),
        );
    }

    group.finish();
}

fn benchmark_query_params_parsing(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("query_params");

    let events_with_params = vec![("since", "100"), ("limit", "50")];
    group.bench_function("GetEventsParams_from_pairs", |bencher| {
        bencher.iter(|| {
            let mut params = GetEventsParams::default();
            for (key, value) in black_box(&events_with_params) {
                match *key {
                    "since" => params.since = value.parse().ok(),
                    "limit" => params.limit = value.parse().ok(),
                    _ => {}
                }
            }
            params
        })
    });

    let leaderboard_json = r#"{"limit": 20, "leaderboard_type": "weekly"}"#;
    group.bench_function("GetLeaderboardParams_parse", |bencher| {
        bencher.iter(|| {
            serde_json::from_str::<GetLeaderboardParams>(black_box(leaderboard_json)).unwrap()
        })
    });

    group.finish();
}

fn benchmark_uuid_parsing(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("uuid_parsing");

    let uuid_str = "550e8400-e29b-41d4-a716-446655440000";
    group.bench_function("uuid_parse", |bencher| {
        bencher.iter(|| uuid::Uuid::parse_str(black_box(uuid_str)).unwrap())
    });

    group.bench_function("uuid_new_v4", |bencher| bencher.iter(uuid::Uuid::new_v4));

    let uuid = uuid::Uuid::parse_str(uuid_str).unwrap();
    group.bench_function("uuid_to_string", |bencher| {
        bencher.iter(|| black_box(&uuid).to_string())
    });

    group.finish();
}

criterion_group!(
    benches,
    benchmark_request_parsing,
    benchmark_command_serialization,
    benchmark_query_params_parsing,
    benchmark_uuid_parsing
);
criterion_main!(benches);
