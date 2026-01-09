-- Dungeon of Pure Functions - Database Schema
-- Event Sourcing + Snapshot Pattern

-- =============================================================================
-- game_sessions - Game Session (Aggregate Root)
-- =============================================================================
CREATE TABLE IF NOT EXISTS game_sessions (
    game_id BINARY(16) PRIMARY KEY,
    player_id BINARY(16) NOT NULL,
    current_floor_level INT UNSIGNED NOT NULL DEFAULT 1,
    turn_count BIGINT UNSIGNED NOT NULL DEFAULT 0,
    status ENUM('in_progress', 'victory', 'defeat', 'paused') NOT NULL DEFAULT 'in_progress',
    random_seed BIGINT UNSIGNED NOT NULL,
    event_sequence BIGINT UNSIGNED NOT NULL DEFAULT 0,
    created_at TIMESTAMP(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    updated_at TIMESTAMP(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6),

    INDEX idx_player_id (player_id),
    INDEX idx_status (status),
    INDEX idx_created_at (created_at)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- =============================================================================
-- game_events - Domain Events (Event Sourcing)
-- =============================================================================
CREATE TABLE IF NOT EXISTS game_events (
    event_id BINARY(16) PRIMARY KEY,
    game_id BINARY(16) NOT NULL,
    sequence_number BIGINT UNSIGNED NOT NULL,
    event_type VARCHAR(100) NOT NULL,
    event_data JSON NOT NULL,
    occurred_at TIMESTAMP(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),

    UNIQUE KEY uk_game_sequence (game_id, sequence_number),
    INDEX idx_game_id (game_id),
    INDEX idx_event_type (event_type),
    INDEX idx_occurred_at (occurred_at),

    CONSTRAINT fk_events_game FOREIGN KEY (game_id) REFERENCES game_sessions(game_id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- =============================================================================
-- game_snapshots - State Snapshots (Performance Optimization)
-- =============================================================================
CREATE TABLE IF NOT EXISTS game_snapshots (
    snapshot_id BINARY(16) PRIMARY KEY,
    game_id BINARY(16) NOT NULL,
    event_sequence BIGINT UNSIGNED NOT NULL,
    snapshot_data JSON NOT NULL,
    created_at TIMESTAMP(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),

    INDEX idx_game_sequence (game_id, event_sequence),
    INDEX idx_created_at (created_at),

    CONSTRAINT fk_snapshots_game FOREIGN KEY (game_id) REFERENCES game_sessions(game_id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- =============================================================================
-- players - Player Data
-- =============================================================================
CREATE TABLE IF NOT EXISTS players (
    player_id BINARY(16) PRIMARY KEY,
    game_id BINARY(16) NOT NULL,
    name VARCHAR(50) NOT NULL,
    level INT UNSIGNED NOT NULL DEFAULT 1,
    current_health INT UNSIGNED NOT NULL,
    max_health INT UNSIGNED NOT NULL,
    current_mana INT UNSIGNED NOT NULL,
    max_mana INT UNSIGNED NOT NULL,
    experience BIGINT UNSIGNED NOT NULL DEFAULT 0,
    position_x INT NOT NULL,
    position_y INT NOT NULL,
    inventory JSON NOT NULL,
    equipment JSON NOT NULL,
    status_effects JSON NOT NULL,
    created_at TIMESTAMP(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    updated_at TIMESTAMP(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6),

    INDEX idx_game_id (game_id),
    INDEX idx_level (level),

    CONSTRAINT fk_players_game FOREIGN KEY (game_id) REFERENCES game_sessions(game_id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- =============================================================================
-- floors - Floor/Dungeon Level Data
-- =============================================================================
CREATE TABLE IF NOT EXISTS floors (
    floor_id BINARY(16) PRIMARY KEY,
    game_id BINARY(16) NOT NULL,
    level INT UNSIGNED NOT NULL,
    width INT UNSIGNED NOT NULL,
    height INT UNSIGNED NOT NULL,
    tiles JSON NOT NULL,
    rooms JSON NOT NULL,
    stairs_up_x INT,
    stairs_up_y INT,
    stairs_down_x INT,
    stairs_down_y INT,
    is_revealed BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMP(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),

    UNIQUE KEY uk_game_level (game_id, level),
    INDEX idx_game_id (game_id),

    CONSTRAINT fk_floors_game FOREIGN KEY (game_id) REFERENCES game_sessions(game_id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- =============================================================================
-- enemies - Enemy Data
-- =============================================================================
CREATE TABLE IF NOT EXISTS enemies (
    enemy_id BINARY(16) PRIMARY KEY,
    game_id BINARY(16) NOT NULL,
    floor_level INT UNSIGNED NOT NULL,
    enemy_type VARCHAR(50) NOT NULL,
    name VARCHAR(100) NOT NULL,
    current_health INT UNSIGNED NOT NULL,
    max_health INT UNSIGNED NOT NULL,
    attack_power INT UNSIGNED NOT NULL,
    defense INT UNSIGNED NOT NULL,
    position_x INT NOT NULL,
    position_y INT NOT NULL,
    behavior_pattern VARCHAR(50) NOT NULL,
    status_effects JSON NOT NULL,
    is_alive BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMP(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    updated_at TIMESTAMP(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6),

    INDEX idx_game_floor (game_id, floor_level),
    INDEX idx_alive (is_alive),

    CONSTRAINT fk_enemies_game FOREIGN KEY (game_id) REFERENCES game_sessions(game_id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- =============================================================================
-- items - Item Data
-- =============================================================================
CREATE TABLE IF NOT EXISTS items (
    item_id BINARY(16) PRIMARY KEY,
    game_id BINARY(16) NOT NULL,
    floor_level INT UNSIGNED,
    item_type VARCHAR(50) NOT NULL,
    name VARCHAR(100) NOT NULL,
    description TEXT,
    rarity ENUM('common', 'uncommon', 'rare', 'epic', 'legendary') NOT NULL DEFAULT 'common',
    properties JSON NOT NULL,
    position_x INT,
    position_y INT,
    owner_type ENUM('floor', 'player', 'enemy') NOT NULL,
    owner_id BINARY(16),
    created_at TIMESTAMP(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),

    INDEX idx_game_floor (game_id, floor_level),
    INDEX idx_owner (owner_type, owner_id),
    INDEX idx_item_type (item_type),

    CONSTRAINT fk_items_game FOREIGN KEY (game_id) REFERENCES game_sessions(game_id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
