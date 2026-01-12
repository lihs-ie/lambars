use roguelike_domain::common::Position;
use roguelike_domain::game_session::GameStatus;
use roguelike_workflow::SessionStateAccessor;

use super::response::{
    FloorSummaryResponse, GameSessionResponse, GameStatusResponse, PlayerResponse,
    PositionResponse, ResourceResponse,
};

// =============================================================================
// Position Conversion
// =============================================================================

impl From<&Position> for PositionResponse {
    fn from(pos: &Position) -> Self {
        Self {
            x: pos.x(),
            y: pos.y(),
        }
    }
}

impl From<Position> for PositionResponse {
    fn from(pos: Position) -> Self {
        Self::from(&pos)
    }
}

// =============================================================================
// GameStatus Conversion
// =============================================================================

impl From<GameStatus> for GameStatusResponse {
    fn from(status: GameStatus) -> Self {
        match status {
            GameStatus::InProgress => GameStatusResponse::InProgress,
            GameStatus::Victory => GameStatusResponse::Victory,
            GameStatus::Defeat => GameStatusResponse::Defeat,
            GameStatus::Paused => GameStatusResponse::Paused,
        }
    }
}

// =============================================================================
// Session to Response Conversion
// =============================================================================

pub fn session_to_game_response<S: SessionStateAccessor>(
    session: &S,
    player_name: &str,
) -> GameSessionResponse {
    let status = GameStatusResponse::from(session.status());

    GameSessionResponse {
        game_id: session.identifier().to_string(),
        player: PlayerResponse {
            player_id: uuid::Uuid::new_v4().to_string(),
            name: player_name.to_string(),
            position: PositionResponse { x: 5, y: 5 },
            health: ResourceResponse {
                current: 100,
                max: 100,
            },
            mana: ResourceResponse {
                current: 50,
                max: 50,
            },
            level: 1,
            experience: 0,
        },
        floor: FloorSummaryResponse {
            level: 1,
            width: 50,
            height: 40,
            explored_percentage: 0.0,
        },
        turn_count: session.turn_count().value() as u32,
        status,
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    mod position_conversion {
        use super::*;

        #[rstest]
        fn from_position_ref() {
            let pos = Position::new(10, 20);
            let response = PositionResponse::from(&pos);
            assert_eq!(response.x, 10);
            assert_eq!(response.y, 20);
        }

        #[rstest]
        fn from_position_owned() {
            let pos = Position::new(-5, 15);
            let response = PositionResponse::from(pos);
            assert_eq!(response.x, -5);
            assert_eq!(response.y, 15);
        }
    }

    mod game_status_conversion {
        use super::*;

        #[rstest]
        fn in_progress() {
            let response = GameStatusResponse::from(GameStatus::InProgress);
            assert_eq!(response, GameStatusResponse::InProgress);
        }

        #[rstest]
        fn victory() {
            let response = GameStatusResponse::from(GameStatus::Victory);
            assert_eq!(response, GameStatusResponse::Victory);
        }

        #[rstest]
        fn defeat() {
            let response = GameStatusResponse::from(GameStatus::Defeat);
            assert_eq!(response, GameStatusResponse::Defeat);
        }

        #[rstest]
        fn paused() {
            let response = GameStatusResponse::from(GameStatus::Paused);
            assert_eq!(response, GameStatusResponse::Paused);
        }
    }
}
