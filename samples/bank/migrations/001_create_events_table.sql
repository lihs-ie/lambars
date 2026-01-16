-- Bank Event Store Schema
-- Migration: 001_create_events_table

-- Enable UUID extension
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- Events table (append-only event store)
CREATE TABLE IF NOT EXISTS events (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    aggregate_id UUID NOT NULL,
    aggregate_type VARCHAR(100) NOT NULL,
    event_type VARCHAR(100) NOT NULL,
    event_data JSONB NOT NULL,
    metadata JSONB DEFAULT '{}',
    version BIGINT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT unique_aggregate_version UNIQUE (aggregate_id, version)
);

-- Index for efficient aggregate reconstruction
CREATE INDEX IF NOT EXISTS idx_events_aggregate_id ON events (aggregate_id);
CREATE INDEX IF NOT EXISTS idx_events_aggregate_id_version ON events (aggregate_id, version);
CREATE INDEX IF NOT EXISTS idx_events_created_at ON events (created_at);
CREATE INDEX IF NOT EXISTS idx_events_event_type ON events (event_type);

-- Snapshots table (for performance optimization)
CREATE TABLE IF NOT EXISTS snapshots (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    aggregate_id UUID NOT NULL UNIQUE,
    aggregate_type VARCHAR(100) NOT NULL,
    state_data JSONB NOT NULL,
    version BIGINT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Index for snapshot lookup
CREATE INDEX IF NOT EXISTS idx_snapshots_aggregate_id ON snapshots (aggregate_id);

-- Outbox table (for reliable event publishing)
CREATE TABLE IF NOT EXISTS outbox (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    aggregate_id UUID NOT NULL,
    event_type VARCHAR(100) NOT NULL,
    payload JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    published_at TIMESTAMPTZ,
    retry_count INT NOT NULL DEFAULT 0,
    last_error TEXT
);

-- Index for unpublished messages
CREATE INDEX IF NOT EXISTS idx_outbox_unpublished ON outbox (created_at) WHERE published_at IS NULL;
CREATE INDEX IF NOT EXISTS idx_outbox_retry ON outbox (retry_count, created_at) WHERE published_at IS NULL;

-- Function to update updated_at timestamp
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ language 'plpgsql';

-- Trigger for snapshots updated_at
DROP TRIGGER IF EXISTS update_snapshots_updated_at ON snapshots;
CREATE TRIGGER update_snapshots_updated_at
    BEFORE UPDATE ON snapshots
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();
