use chrono::{DateTime, Utc};
use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use roguelike_api::dto::response::{
    BaseStatsResponse, CombatStatsResponse, ComponentStatusResponse, ComponentsResponse,
    DroppedItemResponse, EnemySummaryResponse, EquipmentResponse, FloorResponse,
    FloorSummaryResponse, GameEventResponse, GameSessionResponse, GameStatusResponse,
    HealthResponse, HealthStatusResponse, ItemKindResponse, ItemRarityResponse, ItemResponse,
    LeaderboardEntryResponse, LeaderboardResponse, PlayerDetailResponse, PlayerResponse,
    PositionResponse, ResourceResponse, StatusEffectResponse, TileKindResponse, TileResponse,
    TurnResultResponse,
};

fn create_player_response() -> PlayerResponse {
    PlayerResponse {
        player_id: "player-12345678".to_string(),
        name: "Hero".to_string(),
        position: PositionResponse { x: 25, y: 30 },
        health: ResourceResponse {
            current: 85,
            max: 100,
        },
        mana: ResourceResponse {
            current: 40,
            max: 50,
        },
        level: 5,
        experience: 2500,
    }
}

fn create_floor_summary_response() -> FloorSummaryResponse {
    FloorSummaryResponse {
        level: 3,
        width: 50,
        height: 40,
        explored_percentage: 45.5,
    }
}

fn create_game_session_response() -> GameSessionResponse {
    GameSessionResponse {
        game_id: "game-abcd1234-efgh-5678-ijkl-9012mnop".to_string(),
        player: create_player_response(),
        floor: create_floor_summary_response(),
        turn_count: 150,
        status: GameStatusResponse::InProgress,
    }
}

fn create_floor_response(width: usize, height: usize) -> FloorResponse {
    let tiles: Vec<Vec<TileResponse>> = (0..height)
        .map(|y| {
            (0..width)
                .map(|x| TileResponse {
                    kind: if x == 0 || x == width - 1 || y == 0 || y == height - 1 {
                        TileKindResponse::Wall
                    } else {
                        TileKindResponse::Floor
                    },
                    is_explored: x < width / 2,
                    is_visible: (x as i32 - 25).abs() < 5 && (y as i32 - 20).abs() < 5,
                })
                .collect()
        })
        .collect();

    let visible_enemies: Vec<EnemySummaryResponse> = (0i32..5)
        .map(|i| EnemySummaryResponse {
            enemy_id: format!("enemy-{}", i),
            enemy_type: "Goblin".to_string(),
            position: PositionResponse { x: 20 + i, y: 15 },
            health_percentage: 0.8 - (f64::from(i) * 0.1),
        })
        .collect();

    let visible_items: Vec<DroppedItemResponse> = (0i32..3)
        .map(|i| DroppedItemResponse {
            item_id: format!("item-{}", i),
            name: format!("Health Potion {}", i),
            position: PositionResponse { x: 10 + i, y: 10 },
        })
        .collect();

    FloorResponse {
        level: 3,
        width: width as u32,
        height: height as u32,
        tiles,
        visible_enemies,
        visible_items,
        stairs_up: Some(PositionResponse { x: 5, y: 5 }),
        stairs_down: Some(PositionResponse { x: 45, y: 35 }),
    }
}

fn create_game_event_response(sequence: u64) -> GameEventResponse {
    GameEventResponse {
        sequence,
        event_type: "PlayerMoved".to_string(),
        data: serde_json::json!({
            "from": {"x": 24, "y": 30},
            "to": {"x": 25, "y": 30},
            "direction": "east"
        }),
        occurred_at: DateTime::parse_from_rfc3339("2026-01-12T10:00:00Z")
            .unwrap()
            .with_timezone(&Utc),
    }
}

fn create_turn_result_response(event_count: usize) -> TurnResultResponse {
    let events: Vec<GameEventResponse> = (0..event_count)
        .map(|i| create_game_event_response(i as u64 + 1))
        .collect();

    TurnResultResponse {
        game: create_game_session_response(),
        turn_events: events,
        game_over: false,
        game_over_reason: None,
    }
}

fn create_player_detail_response() -> PlayerDetailResponse {
    PlayerDetailResponse {
        player_id: "player-12345678".to_string(),
        name: "Hero".to_string(),
        position: PositionResponse { x: 25, y: 30 },
        health: ResourceResponse {
            current: 85,
            max: 100,
        },
        mana: ResourceResponse {
            current: 40,
            max: 50,
        },
        level: 5,
        experience: 2500,
        experience_to_next_level: 3000,
        base_stats: BaseStatsResponse {
            strength: 15,
            dexterity: 12,
            intelligence: 10,
            vitality: 14,
        },
        combat_stats: CombatStatsResponse {
            attack: 25,
            defense: 18,
            speed: 15,
        },
        equipment: EquipmentResponse {
            weapon: Some(ItemResponse {
                item_id: "sword-001".to_string(),
                name: "Iron Sword".to_string(),
                kind: ItemKindResponse::Weapon,
                description: "A sturdy iron sword".to_string(),
                rarity: Some(ItemRarityResponse::Uncommon),
            }),
            armor: Some(ItemResponse {
                item_id: "armor-001".to_string(),
                name: "Leather Armor".to_string(),
                kind: ItemKindResponse::Armor,
                description: "Basic leather armor".to_string(),
                rarity: Some(ItemRarityResponse::Common),
            }),
            helmet: None,
            accessory: None,
        },
        status_effects: vec![StatusEffectResponse {
            effect_type: "Blessed".to_string(),
            remaining_turns: 10,
            magnitude: 1.2,
        }],
    }
}

fn create_leaderboard_response(entry_count: usize) -> LeaderboardResponse {
    let entries: Vec<LeaderboardEntryResponse> = (0..entry_count)
        .map(|i| LeaderboardEntryResponse {
            rank: (i + 1) as u32,
            player_name: format!("Player_{}", i + 1),
            score: 100000 - (i as u64 * 5000),
            dungeon_depth: 15 - (i as u32 / 2),
            outcome: if i == 0 { "victory" } else { "defeat" }.to_string(),
            completed_at: DateTime::parse_from_rfc3339("2026-01-12T10:00:00Z")
                .unwrap()
                .with_timezone(&Utc),
        })
        .collect();

    LeaderboardResponse {
        leaderboard_type: "global".to_string(),
        entries,
    }
}

fn create_health_response() -> HealthResponse {
    HealthResponse {
        status: HealthStatusResponse::Healthy,
        version: "0.1.0".to_string(),
        components: ComponentsResponse {
            database: ComponentStatusResponse::Up,
            cache: ComponentStatusResponse::Up,
        },
    }
}

fn benchmark_serialization(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("dto_serialization");

    let game_session = create_game_session_response();
    group.bench_function("GameSessionResponse", |bencher| {
        bencher.iter(|| serde_json::to_string(black_box(&game_session)).unwrap())
    });

    let player_detail = create_player_detail_response();
    group.bench_function("PlayerDetailResponse", |bencher| {
        bencher.iter(|| serde_json::to_string(black_box(&player_detail)).unwrap())
    });

    let health = create_health_response();
    group.bench_function("HealthResponse", |bencher| {
        bencher.iter(|| serde_json::to_string(black_box(&health)).unwrap())
    });

    for size in [10, 25, 50] {
        let floor = create_floor_response(size, size);
        group.bench_with_input(
            BenchmarkId::new("FloorResponse", format!("{}x{}", size, size)),
            &floor,
            |bencher, floor| bencher.iter(|| serde_json::to_string(black_box(floor)).unwrap()),
        );
    }

    for count in [10, 50, 100] {
        let turn_result = create_turn_result_response(count);
        group.bench_with_input(
            BenchmarkId::new("TurnResultResponse", format!("{}_events", count)),
            &turn_result,
            |bencher, turn_result| {
                bencher.iter(|| serde_json::to_string(black_box(turn_result)).unwrap())
            },
        );
    }

    for count in [10, 50, 100] {
        let leaderboard = create_leaderboard_response(count);
        group.bench_with_input(
            BenchmarkId::new("LeaderboardResponse", format!("{}_entries", count)),
            &leaderboard,
            |bencher, leaderboard| {
                bencher.iter(|| serde_json::to_string(black_box(leaderboard)).unwrap())
            },
        );
    }

    group.finish();
}

fn benchmark_deserialization(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("dto_deserialization");

    let game_session_json = serde_json::to_string(&create_game_session_response()).unwrap();
    group.bench_function("GameSessionResponse", |bencher| {
        bencher.iter(|| {
            serde_json::from_str::<GameSessionResponse>(black_box(&game_session_json)).unwrap()
        })
    });

    let health_json = serde_json::to_string(&create_health_response()).unwrap();
    group.bench_function("HealthResponse", |bencher| {
        bencher.iter(|| serde_json::from_str::<HealthResponse>(black_box(&health_json)).unwrap())
    });

    for size in [10, 25, 50] {
        let floor_json = serde_json::to_string(&create_floor_response(size, size)).unwrap();
        group.bench_with_input(
            BenchmarkId::new("FloorResponse", format!("{}x{}", size, size)),
            &floor_json,
            |bencher, json| {
                bencher.iter(|| serde_json::from_str::<FloorResponse>(black_box(json)).unwrap())
            },
        );
    }

    group.finish();
}

criterion_group!(benches, benchmark_serialization, benchmark_deserialization);
criterion_main!(benches);
