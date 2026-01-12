//! Data Transfer Objects for the API layer.
//!
//! This module provides all DTOs for request/response handling:
//!
//! - [`request`]: Request DTOs for API endpoints
//! - [`response`]: Response DTOs for API endpoints
//! - [`command`]: Command DTOs for game actions

pub mod command;
pub mod request;
pub mod response;

pub use command::CommandRequest;
pub use request::{
    CreateGameRequest, EndGameRequest, ExecuteCommandRequest, GetEventsParams, GetFloorParams,
    GetLeaderboardParams,
};
pub use response::{
    BaseStatsResponse, CombatStatsResponse, DroppedItemResponse, EnemySummaryResponse,
    EquipmentResponse, ErrorResponse, EventsResponse, FloorResponse, FloorSummaryResponse,
    GameEndResponse, GameEventResponse, GameSessionResponse, HealthResponse, InventoryResponse,
    ItemResponse, ItemStackResponse, LeaderboardEntryResponse, LeaderboardResponse,
    PlayerDetailResponse, PlayerResponse, PositionResponse, ResourceResponse, StatusEffectResponse,
    TileResponse, TurnResultResponse, VisibleAreaResponse, VisibleTileResponse,
};
