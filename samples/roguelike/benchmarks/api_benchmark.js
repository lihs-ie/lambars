import http from 'k6/http';
import { check, sleep, group } from 'k6';
import { Counter, Rate, Trend } from 'k6/metrics';

// Custom metrics
const healthCheckLatency = new Trend('health_check_latency');
const createGameLatency = new Trend('create_game_latency');
const getGameLatency = new Trend('get_game_latency');
const getPlayerLatency = new Trend('get_player_latency');
const executeCommandLatency = new Trend('execute_command_latency');
const getFloorLatency = new Trend('get_floor_latency');
const getEventsLatency = new Trend('get_events_latency');
const leaderboardLatency = new Trend('leaderboard_latency');
const gameLifecycleLatency = new Trend('game_lifecycle_latency');

const errorRate = new Rate('errors');
const gameCreationErrors = new Counter('game_creation_errors');

const BASE_URL = __ENV.BASE_URL || 'http://localhost:8080';

export const options = {
    scenarios: {
        // Smoke test
        smoke: {
            executor: 'constant-vus',
            vus: 1,
            duration: '10s',
            startTime: '0s',
            tags: { scenario: 'smoke' },
        },
        // Load test
        load: {
            executor: 'ramping-vus',
            startVUs: 0,
            stages: [
                { duration: '30s', target: 10 },
                { duration: '1m', target: 10 },
                { duration: '30s', target: 0 },
            ],
            startTime: '15s',
            tags: { scenario: 'load' },
        },
        // Stress test
        stress: {
            executor: 'ramping-vus',
            startVUs: 0,
            stages: [
                { duration: '30s', target: 20 },
                { duration: '30s', target: 50 },
                { duration: '1m', target: 50 },
                { duration: '30s', target: 0 },
            ],
            startTime: '2m30s',
            tags: { scenario: 'stress' },
        },
    },
    thresholds: {
        http_req_duration: ['p(95)<500', 'p(99)<1000'],
        http_req_failed: ['rate<0.01'],
        errors: ['rate<0.05'],
        health_check_latency: ['p(95)<100'],
        create_game_latency: ['p(95)<500'],
        get_game_latency: ['p(95)<200'],
        execute_command_latency: ['p(95)<300'],
    },
};

// Health check benchmark
function benchmarkHealthCheck() {
    const startTime = Date.now();
    const response = http.get(`${BASE_URL}/api/v1/health`);
    healthCheckLatency.add(Date.now() - startTime);

    const success = check(response, {
        'health check status is 200': (r) => r.status === 200,
        'health check has status field': (r) => {
            const body = JSON.parse(r.body);
            return body.status === 'healthy';
        },
        'health check has components': (r) => {
            const body = JSON.parse(r.body);
            return body.components && body.components.database === 'up';
        },
    });

    errorRate.add(!success);
    return success;
}

// Create game benchmark
function benchmarkCreateGame(playerName) {
    const startTime = Date.now();
    const payload = JSON.stringify({ player_name: playerName });
    const params = {
        headers: { 'Content-Type': 'application/json' },
    };

    const response = http.post(`${BASE_URL}/api/v1/games`, payload, params);
    createGameLatency.add(Date.now() - startTime);

    const success = check(response, {
        'create game status is 201': (r) => r.status === 201,
        'create game returns game_id': (r) => {
            const body = JSON.parse(r.body);
            return body.game_id !== undefined;
        },
    });

    if (!success) {
        gameCreationErrors.add(1);
    }
    errorRate.add(!success);

    if (success) {
        const body = JSON.parse(response.body);
        return body.game_id;
    }
    return null;
}

// Get game benchmark
function benchmarkGetGame(gameId) {
    if (!gameId) return false;

    const startTime = Date.now();
    const response = http.get(`${BASE_URL}/api/v1/games/${gameId}`);
    getGameLatency.add(Date.now() - startTime);

    const success = check(response, {
        'get game status is 200': (r) => r.status === 200,
        'get game returns status': (r) => {
            const body = JSON.parse(r.body);
            return body.status !== undefined;
        },
    });

    errorRate.add(!success);
    return success;
}

// Get player benchmark
function benchmarkGetPlayer(gameId) {
    if (!gameId) return false;

    const startTime = Date.now();
    const response = http.get(`${BASE_URL}/api/v1/games/${gameId}/player`);
    getPlayerLatency.add(Date.now() - startTime);

    const success = check(response, {
        'get player status is 200': (r) => r.status === 200,
        'get player returns health': (r) => {
            const body = JSON.parse(r.body);
            return body.health !== undefined;
        },
    });

    errorRate.add(!success);
    return success;
}

// Execute command benchmark
function benchmarkExecuteCommand(gameId, command) {
    if (!gameId) return false;

    const startTime = Date.now();
    const payload = JSON.stringify(command);
    const params = {
        headers: { 'Content-Type': 'application/json' },
    };

    const response = http.post(`${BASE_URL}/api/v1/games/${gameId}/commands`, payload, params);
    executeCommandLatency.add(Date.now() - startTime);

    const success = check(response, {
        'execute command status is 200': (r) => r.status === 200,
    });

    errorRate.add(!success);
    return success;
}

// Get floor benchmark
function benchmarkGetFloor(gameId) {
    if (!gameId) return false;

    const startTime = Date.now();
    const response = http.get(`${BASE_URL}/api/v1/games/${gameId}/floor`);
    getFloorLatency.add(Date.now() - startTime);

    const success = check(response, {
        'get floor status is 200': (r) => r.status === 200,
    });

    errorRate.add(!success);
    return success;
}

// Get events benchmark
function benchmarkGetEvents(gameId) {
    if (!gameId) return false;

    const startTime = Date.now();
    const response = http.get(`${BASE_URL}/api/v1/games/${gameId}/events`);
    getEventsLatency.add(Date.now() - startTime);

    const success = check(response, {
        'get events status is 200': (r) => r.status === 200,
        'get events returns array': (r) => {
            const body = JSON.parse(r.body);
            return Array.isArray(body.events);
        },
    });

    errorRate.add(!success);
    return success;
}

// Leaderboard benchmark
function benchmarkLeaderboard() {
    const startTime = Date.now();
    const response = http.get(`${BASE_URL}/api/v1/leaderboard`);
    leaderboardLatency.add(Date.now() - startTime);

    const success = check(response, {
        'leaderboard status is 200': (r) => r.status === 200,
        'leaderboard returns entries': (r) => {
            const body = JSON.parse(r.body);
            return body.entries !== undefined;
        },
    });

    errorRate.add(!success);
    return success;
}

// Full game lifecycle benchmark
function benchmarkGameLifecycle() {
    const startTime = Date.now();

    // Create game
    const gameId = benchmarkCreateGame(`Player_${__VU}_${__ITER}`);
    if (!gameId) {
        gameLifecycleLatency.add(Date.now() - startTime);
        return;
    }

    // Get game state
    benchmarkGetGame(gameId);

    // Get player info
    benchmarkGetPlayer(gameId);

    // Get floor
    benchmarkGetFloor(gameId);

    // Execute some commands
    const directions = ['north', 'south', 'east', 'west'];
    for (let i = 0; i < 3; i++) {
        const direction = directions[Math.floor(Math.random() * directions.length)];
        benchmarkExecuteCommand(gameId, { type: 'move', direction: direction });
        sleep(0.1);
    }

    // Get events
    benchmarkGetEvents(gameId);

    // End game
    const endResponse = http.post(`${BASE_URL}/api/v1/games/${gameId}/end`, null, {
        headers: { 'Content-Type': 'application/json' },
    });

    check(endResponse, {
        'end game status is 200': (r) => r.status === 200,
    });

    gameLifecycleLatency.add(Date.now() - startTime);
}

export default function () {
    group('Health Check', () => {
        benchmarkHealthCheck();
    });

    sleep(0.5);

    group('Game Lifecycle', () => {
        benchmarkGameLifecycle();
    });

    sleep(0.5);

    group('Leaderboard', () => {
        benchmarkLeaderboard();
    });

    sleep(1);
}

export function handleSummary(data) {
    return {
        'stdout': textSummary(data, { indent: '  ', enableColors: true }),
        'benchmark_results.json': JSON.stringify(data, null, 2),
    };
}

function textSummary(data, opts) {
    const lines = [];
    lines.push('\n╔════════════════════════════════════════════════════════════╗');
    lines.push('║           ROGUELIKE API BENCHMARK RESULTS                  ║');
    lines.push('╚════════════════════════════════════════════════════════════╝\n');

    // HTTP metrics
    if (data.metrics.http_req_duration) {
        const m = data.metrics.http_req_duration.values;
        lines.push('📊 HTTP Request Duration:');
        lines.push(`   avg: ${m.avg?.toFixed(2) || 'N/A'}ms`);
        lines.push(`   min: ${m.min?.toFixed(2) || 'N/A'}ms`);
        lines.push(`   max: ${m.max?.toFixed(2) || 'N/A'}ms`);
        lines.push(`   p(90): ${m['p(90)']?.toFixed(2) || 'N/A'}ms`);
        lines.push(`   p(95): ${m['p(95)']?.toFixed(2) || 'N/A'}ms`);
        lines.push(`   p(99): ${m['p(99)']?.toFixed(2) || 'N/A'}ms\n`);
    }

    // Custom metrics
    const customMetrics = [
        ['health_check_latency', '🏥 Health Check'],
        ['create_game_latency', '🎮 Create Game'],
        ['get_game_latency', '📖 Get Game'],
        ['get_player_latency', '👤 Get Player'],
        ['execute_command_latency', '⚡ Execute Command'],
        ['get_floor_latency', '🗺️  Get Floor'],
        ['get_events_latency', '📋 Get Events'],
        ['leaderboard_latency', '🏆 Leaderboard'],
        ['game_lifecycle_latency', '🔄 Full Lifecycle'],
    ];

    lines.push('📈 Endpoint Latencies:');
    for (const [key, label] of customMetrics) {
        if (data.metrics[key]) {
            const m = data.metrics[key].values;
            lines.push(`   ${label}: avg=${m.avg?.toFixed(2) || 'N/A'}ms, p95=${m['p(95)']?.toFixed(2) || 'N/A'}ms`);
        }
    }
    lines.push('');

    // Request counts
    if (data.metrics.http_reqs) {
        lines.push('📦 Request Stats:');
        lines.push(`   Total requests: ${data.metrics.http_reqs.values.count || 0}`);
        lines.push(`   Requests/sec: ${data.metrics.http_reqs.values.rate?.toFixed(2) || 'N/A'}`);
    }

    // Error rate
    if (data.metrics.errors) {
        lines.push(`   Error rate: ${((data.metrics.errors.values.rate || 0) * 100).toFixed(2)}%`);
    }

    // Thresholds
    lines.push('\n🎯 Threshold Results:');
    for (const [name, threshold] of Object.entries(data.thresholds || {})) {
        const status = threshold.ok ? '✅ PASS' : '❌ FAIL';
        lines.push(`   ${name}: ${status}`);
    }

    lines.push('\n════════════════════════════════════════════════════════════════\n');

    return lines.join('\n');
}
