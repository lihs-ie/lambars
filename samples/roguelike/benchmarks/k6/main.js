import { sleep, group, check } from 'k6';
import { getOptions } from './config/options.js';
import { client, parseResponse } from './helpers/http-client.js';
import { commands, randomDirection } from './helpers/commands.js';
import * as metrics from './helpers/metrics.js';

const SCENARIO = __ENV.SCENARIO || 'smoke';
export const options = getOptions(SCENARIO);

function testHealth() {
    const start = Date.now();
    const response = client.health();
    metrics.healthLatency.add(Date.now() - start);

    const success = check(response, {
        'health: status 200': (r) => r.status === 200,
        'health: status is healthy': (r) => {
            const body = parseResponse(r);
            return body && body.status === 'healthy';
        },
    });

    if (!success) metrics.errors.add(1);
    metrics.errorRate.add(!success);
}

function testGameLifecycle() {
    const start = Date.now();

    const createStart = Date.now();
    const createResponse = client.createGame(`Player_${__VU}_${__ITER}`);
    metrics.createGameLatency.add(Date.now() - createStart);

    if (createResponse.status !== 201) {
        metrics.errors.add(1);
        metrics.errorRate.add(true);
        return;
    }

    metrics.gamesCreated.add(1);
    const body = parseResponse(createResponse);
    const gameId = body ? body.game_id : null;

    if (!gameId) {
        metrics.errors.add(1);
        metrics.errorRate.add(true);
        return;
    }

    const getStart = Date.now();
    const getResponse = client.getGame(gameId);
    metrics.getGameLatency.add(Date.now() - getStart);
    check(getResponse, { 'get game: status 200': (r) => r.status === 200 });

    const playerStart = Date.now();
    client.getPlayer(gameId);
    metrics.getPlayerLatency.add(Date.now() - playerStart);

    const floorStart = Date.now();
    client.getFloor(gameId);
    metrics.getFloorLatency.add(Date.now() - floorStart);

    for (let i = 0; i < 5; i++) {
        const direction = randomDirection();
        const commandStart = Date.now();
        const commandResponse = client.executeCommand(gameId, commands.move(direction));
        metrics.commandMoveLatency.add(Date.now() - commandStart);
        metrics.commandLatency.add(Date.now() - commandStart);
        metrics.commandsExecuted.add(1);

        sleep(0.1);
    }

    const waitStart = Date.now();
    client.executeCommand(gameId, commands.wait());
    metrics.commandWaitLatency.add(Date.now() - waitStart);
    metrics.commandLatency.add(Date.now() - waitStart);

    const eventsStart = Date.now();
    client.getEvents(gameId);
    metrics.getEventsLatency.add(Date.now() - eventsStart);

    const endStart = Date.now();
    client.endGame(gameId, 'abandon');
    metrics.endGameLatency.add(Date.now() - endStart);

    metrics.gameLifecycleLatency.add(Date.now() - start);
}

function testLeaderboard() {
    const start = Date.now();
    const response = client.getLeaderboard('global', 10);
    metrics.getLeaderboardLatency.add(Date.now() - start);

    check(response, {
        'leaderboard: status 200': (r) => r.status === 200,
    });
}

export default function () {
    group('Health Check', testHealth);
    sleep(0.2);

    group('Game Lifecycle', testGameLifecycle);
    sleep(0.5);

    group('Leaderboard', testLeaderboard);
    sleep(0.3);
}

export function handleSummary(data) {
    const scenario = SCENARIO;

    return {
        stdout: generateTextReport(data, scenario),
    };
}

function generateTextReport(data, scenario) {
    const lines = [];
    lines.push('');
    lines.push('='.repeat(70));
    lines.push('        ROGUELIKE API BENCHMARK RESULTS');
    lines.push('='.repeat(70));
    lines.push('');
    lines.push(`Scenario: ${scenario.toUpperCase()}`);
    lines.push(`Timestamp: ${new Date().toISOString()}`);
    lines.push('');

    if (data.metrics.http_req_duration) {
        const m = data.metrics.http_req_duration.values;
        lines.push('HTTP Request Duration:');
        lines.push(`  avg:    ${formatMs(m.avg)}`);
        lines.push(`  min:    ${formatMs(m.min)}`);
        lines.push(`  max:    ${formatMs(m.max)}`);
        lines.push(`  p(90):  ${formatMs(m['p(90)'])}`);
        lines.push(`  p(95):  ${formatMs(m['p(95)'])}`);
        lines.push(`  p(99):  ${formatMs(m['p(99)'])}`);
        lines.push('');
    }

    lines.push('Endpoint Latencies (p95):');
    const endpointMetrics = [
        ['roguelike_health_latency', 'Health Check'],
        ['roguelike_create_game_latency', 'Create Game'],
        ['roguelike_get_game_latency', 'Get Game'],
        ['roguelike_get_player_latency', 'Get Player'],
        ['roguelike_get_floor_latency', 'Get Floor'],
        ['roguelike_command_latency', 'Execute Command'],
        ['roguelike_get_events_latency', 'Get Events'],
        ['roguelike_get_leaderboard_latency', 'Leaderboard'],
        ['roguelike_game_lifecycle_latency', 'Full Lifecycle'],
    ];

    for (const [key, label] of endpointMetrics) {
        if (data.metrics[key]) {
            const m = data.metrics[key].values;
            const p95 = formatMs(m['p(95)']);
            const avg = formatMs(m.avg);
            lines.push(`  ${label.padEnd(20)} avg=${avg.padStart(10)}, p95=${p95.padStart(10)}`);
        }
    }
    lines.push('');

    if (data.metrics.http_reqs) {
        lines.push('Request Stats:');
        lines.push(`  Total requests: ${data.metrics.http_reqs.values.count || 0}`);
        lines.push(`  Requests/sec:   ${(data.metrics.http_reqs.values.rate || 0).toFixed(2)}`);
    }

    if (data.metrics.roguelike_errors) {
        lines.push(`  Error rate:     ${((data.metrics.roguelike_errors.values.rate || 0) * 100).toFixed(2)}%`);
    }
    lines.push('');

    lines.push('Threshold Results:');
    for (const [name, threshold] of Object.entries(data.thresholds || {})) {
        const status = threshold.ok ? 'PASS' : 'FAIL';
        const icon = threshold.ok ? '[OK]' : '[NG]';
        lines.push(`  ${icon} ${name}: ${status}`);
    }

    lines.push('');
    lines.push('='.repeat(70));
    lines.push('');

    return lines.join('\n');
}

function formatMs(value) {
    if (value === undefined || value === null) return 'N/A';
    return `${value.toFixed(2)}ms`;
}
