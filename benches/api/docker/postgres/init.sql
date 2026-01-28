-- PostgreSQL initialization script for Task Management Benchmark API
--
-- This script creates the database schema required for:
--   - Task storage (JSONB)
--   - Project storage (JSONB)
--   - Event sourcing (task_events)

-- =============================================================================
-- Extensions
-- =============================================================================

CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- pg_trgm extension for trigram-based text search
-- Enables efficient ILIKE/LIKE queries using GIN indexes
CREATE EXTENSION IF NOT EXISTS pg_trgm;

-- =============================================================================
-- Tasks Table
-- =============================================================================

CREATE TABLE IF NOT EXISTS tasks (
    id UUID PRIMARY KEY,
    data JSONB NOT NULL,
    version BIGINT NOT NULL DEFAULT 1,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Index for version-based queries (optimistic locking)
CREATE INDEX IF NOT EXISTS idx_tasks_version ON tasks(version);

-- Index for created_at (pagination ordering)
CREATE INDEX IF NOT EXISTS idx_tasks_created_at ON tasks(created_at);

-- GIN index for JSONB queries (if needed for searching)
CREATE INDEX IF NOT EXISTS idx_tasks_data ON tasks USING GIN(data);

-- GIN index for title search using trigrams (REQ-SEARCH-DB-001)
-- Enables efficient ILIKE/LIKE queries on JSONB title field
-- NOTE: Queries using this index MUST include LIMIT/OFFSET to prevent full table scans
CREATE INDEX IF NOT EXISTS idx_tasks_title_trgm
    ON tasks USING GIN (lower((data->>'title')) gin_trgm_ops);

-- GIN index for tags search using trigrams (REQ-SEARCH-DB-001)
-- Enables efficient ILIKE/LIKE queries on JSONB tags field
-- NOTE: Queries using this index MUST include LIMIT/OFFSET to prevent full table scans
CREATE INDEX IF NOT EXISTS idx_tasks_tags_trgm
    ON tasks USING GIN ((lower((data->>'tags')::text)) gin_trgm_ops);

-- =============================================================================
-- Projects Table
-- =============================================================================

CREATE TABLE IF NOT EXISTS projects (
    id UUID PRIMARY KEY,
    data JSONB NOT NULL,
    version BIGINT NOT NULL DEFAULT 1,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Index for version-based queries (optimistic locking)
CREATE INDEX IF NOT EXISTS idx_projects_version ON projects(version);

-- Index for created_at (pagination ordering)
CREATE INDEX IF NOT EXISTS idx_projects_created_at ON projects(created_at);

-- GIN index for JSONB queries
CREATE INDEX IF NOT EXISTS idx_projects_data ON projects USING GIN(data);

-- =============================================================================
-- Task Events Table (Event Sourcing)
-- =============================================================================

CREATE TABLE IF NOT EXISTS task_events (
    id UUID PRIMARY KEY,
    task_id UUID NOT NULL,
    event_type VARCHAR(100) NOT NULL,
    event_data JSONB NOT NULL,
    version BIGINT NOT NULL,
    occurred_at TIMESTAMPTZ NOT NULL,
    -- Ensure unique version per task (optimistic locking for events)
    CONSTRAINT unique_task_version UNIQUE (task_id, version)
);

-- Index for loading events by task_id in version order
CREATE INDEX IF NOT EXISTS idx_task_events_task_id ON task_events(task_id, version);

-- Index for event type queries
CREATE INDEX IF NOT EXISTS idx_task_events_event_type ON task_events(event_type);

-- Index for time-based queries
CREATE INDEX IF NOT EXISTS idx_task_events_occurred_at ON task_events(occurred_at);

-- =============================================================================
-- Trigger: Auto-update updated_at on tasks
-- =============================================================================

CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE TRIGGER tasks_updated_at
    BEFORE UPDATE ON tasks
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

CREATE OR REPLACE TRIGGER projects_updated_at
    BEFORE UPDATE ON projects
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- =============================================================================
-- Sample Data (Optional - for testing)
-- =============================================================================

-- Uncomment to insert sample data for testing
-- INSERT INTO tasks (id, data, version) VALUES
--     (uuid_generate_v4(), '{"title": "Sample Task", "status": "pending"}', 1);
