use std::mem::size_of;

use chrono::{DateTime, Utc};
use roguelike_api::dto::command::{CommandRequest, DirectionRequest, EquipmentSlotRequest};
use roguelike_api::dto::request::{
    CreateGameRequest, EndGameRequest, ExecuteCommandRequest, GameOutcomeRequest, GetEventsParams,
    GetFloorParams, GetLeaderboardParams, LeaderboardTypeRequest,
};
use roguelike_api::dto::response::{
    BaseStatsResponse, CombatStatsResponse, ComponentStatusResponse, ComponentsResponse,
    DroppedItemResponse, EnemySummaryResponse, EquipmentResponse, ErrorDetailResponse,
    ErrorResponse, EventsResponse, FloorResponse, FloorSummaryResponse, GameEndResponse,
    GameEventResponse, GameSessionResponse, GameStatusResponse, HealthResponse,
    HealthStatusResponse, InventoryResponse, ItemKindResponse, ItemRarityResponse, ItemResponse,
    ItemStackResponse, LeaderboardEntryResponse, LeaderboardResponse, PlayerDetailResponse,
    PlayerResponse, PositionResponse, ResourceResponse, StatusEffectResponse, TileKindResponse,
    TileResponse, TurnResultResponse, VisibleAreaResponse, VisibleTileResponse,
};

fn print_separator() {
    println!("{}", "=".repeat(70));
}

fn print_header(title: &str) {
    println!();
    print_separator();
    println!("{}", title);
    print_separator();
}

fn print_size<T>(name: &str) {
    println!("{:<45} {:>8} bytes", name, size_of::<T>());
}

fn main() {
    println!();
    println!("roguelike API DTO Memory Size Report");
    println!("Generated: {}", Utc::now().format("%Y-%m-%d %H:%M:%S UTC"));

    print_header("Response DTOs - Primitive Types");
    print_size::<PositionResponse>("PositionResponse");
    print_size::<ResourceResponse>("ResourceResponse");
    print_size::<BaseStatsResponse>("BaseStatsResponse");
    print_size::<CombatStatsResponse>("CombatStatsResponse");
    print_size::<GameStatusResponse>("GameStatusResponse");
    print_size::<TileKindResponse>("TileKindResponse");
    print_size::<TileResponse>("TileResponse");
    print_size::<ItemKindResponse>("ItemKindResponse");
    print_size::<ItemRarityResponse>("ItemRarityResponse");
    print_size::<HealthStatusResponse>("HealthStatusResponse");
    print_size::<ComponentStatusResponse>("ComponentStatusResponse");

    print_header("Response DTOs - Composite Types (Stack Size)");
    print_size::<FloorSummaryResponse>("FloorSummaryResponse");
    print_size::<PlayerResponse>("PlayerResponse");
    print_size::<PlayerDetailResponse>("PlayerDetailResponse");
    print_size::<GameSessionResponse>("GameSessionResponse");
    print_size::<GameEndResponse>("GameEndResponse");
    print_size::<TurnResultResponse>("TurnResultResponse");
    print_size::<EnemySummaryResponse>("EnemySummaryResponse");
    print_size::<DroppedItemResponse>("DroppedItemResponse");
    print_size::<ItemResponse>("ItemResponse");
    print_size::<ItemStackResponse>("ItemStackResponse");
    print_size::<EquipmentResponse>("EquipmentResponse");
    print_size::<StatusEffectResponse>("StatusEffectResponse");
    print_size::<InventoryResponse>("InventoryResponse");
    print_size::<FloorResponse>("FloorResponse");
    print_size::<VisibleTileResponse>("VisibleTileResponse");
    print_size::<VisibleAreaResponse>("VisibleAreaResponse");
    print_size::<GameEventResponse>("GameEventResponse");
    print_size::<EventsResponse>("EventsResponse");
    print_size::<LeaderboardEntryResponse>("LeaderboardEntryResponse");
    print_size::<LeaderboardResponse>("LeaderboardResponse");
    print_size::<HealthResponse>("HealthResponse");
    print_size::<ComponentsResponse>("ComponentsResponse");
    print_size::<ErrorResponse>("ErrorResponse");
    print_size::<ErrorDetailResponse>("ErrorDetailResponse");

    print_header("Request DTOs (Stack Size)");
    print_size::<CreateGameRequest>("CreateGameRequest");
    print_size::<EndGameRequest>("EndGameRequest");
    print_size::<GameOutcomeRequest>("GameOutcomeRequest");
    print_size::<ExecuteCommandRequest>("ExecuteCommandRequest");
    print_size::<GetEventsParams>("GetEventsParams");
    print_size::<GetLeaderboardParams>("GetLeaderboardParams");
    print_size::<LeaderboardTypeRequest>("LeaderboardTypeRequest");
    print_size::<GetFloorParams>("GetFloorParams");

    print_header("Command DTOs (Stack Size)");
    print_size::<CommandRequest>("CommandRequest");
    print_size::<DirectionRequest>("DirectionRequest");
    print_size::<EquipmentSlotRequest>("EquipmentSlotRequest");

    print_header("Standard Library Types (Reference)");
    print_size::<String>("String");
    print_size::<Vec<u8>>("Vec<u8>");
    print_size::<Option<String>>("Option<String>");
    print_size::<DateTime<Utc>>("DateTime<Utc>");
    print_size::<serde_json::Value>("serde_json::Value");

    print_header("Heap Size Estimation - FloorResponse");
    for (width, height) in [(10, 10), (25, 25), (50, 50), (100, 100)] {
        let tiles_heap = width * height * size_of::<TileResponse>();
        let vec_overhead =
            width * size_of::<Vec<TileResponse>>() + size_of::<Vec<Vec<TileResponse>>>();
        let total = tiles_heap + vec_overhead;
        println!(
            "FloorResponse {}x{}: ~{} bytes ({:.2} KB)",
            width,
            height,
            total,
            total as f64 / 1024.0
        );
    }

    print_header("Heap Size Estimation - TurnResultResponse");
    for event_count in [10, 50, 100, 500] {
        let event_stack = size_of::<GameEventResponse>();
        let event_string_estimate = 50;
        let json_estimate = 200;
        let per_event = event_stack + event_string_estimate + json_estimate;
        let total = per_event * event_count + size_of::<TurnResultResponse>();
        println!(
            "TurnResultResponse ({} events): ~{} bytes ({:.2} KB)",
            event_count,
            total,
            total as f64 / 1024.0
        );
    }

    print_header("Heap Size Estimation - LeaderboardResponse");
    for entry_count in [10, 50, 100] {
        let entry_stack = size_of::<LeaderboardEntryResponse>();
        let string_estimate = 30 + 10 + 20;
        let per_entry = entry_stack + string_estimate;
        let total = per_entry * entry_count + size_of::<LeaderboardResponse>();
        println!(
            "LeaderboardResponse ({} entries): ~{} bytes ({:.2} KB)",
            entry_count,
            total,
            total as f64 / 1024.0
        );
    }

    print_header("Memory SLO Comparison");
    println!();
    println!("Target: Container idle < 100MB");
    println!("Target: Load test peak < 500MB");
    println!();

    let typical_game_session = size_of::<GameSessionResponse>() + 200;
    let typical_floor = 50 * 50 * size_of::<TileResponse>() + 50 * size_of::<Vec<TileResponse>>();
    let typical_player = size_of::<PlayerDetailResponse>() + 500;

    println!("Typical GameSession: ~{} bytes", typical_game_session);
    println!(
        "Typical Floor (50x50): ~{} bytes ({:.2} KB)",
        typical_floor,
        typical_floor as f64 / 1024.0
    );
    println!("Typical PlayerDetail: ~{} bytes", typical_player);

    let concurrent_sessions = 100;
    let total_memory =
        concurrent_sessions * (typical_game_session + typical_floor + typical_player);
    println!();
    println!(
        "{} concurrent sessions estimated memory: ~{} bytes ({:.2} MB)",
        concurrent_sessions,
        total_memory,
        total_memory as f64 / 1024.0 / 1024.0
    );

    print_separator();
    println!();
}
