import http from 'k6/http';

const BASE_URL = __ENV.BASE_URL || 'http://localhost:8080';

const jsonHeaders = {
    headers: { 'Content-Type': 'application/json' },
};

export const client = {
    health() {
        return http.get(`${BASE_URL}/api/v1/health`);
    },

    createGame(playerName, seed = null) {
        const payload = { player_name: playerName };
        if (seed !== null) {
            payload.seed = seed;
        }
        return http.post(`${BASE_URL}/api/v1/games`, JSON.stringify(payload), jsonHeaders);
    },

    getGame(gameId) {
        return http.get(`${BASE_URL}/api/v1/games/${gameId}`);
    },

    endGame(gameId, outcome = 'abandon') {
        return http.post(
            `${BASE_URL}/api/v1/games/${gameId}/end`,
            JSON.stringify({ outcome }),
            jsonHeaders
        );
    },

    executeCommand(gameId, command) {
        return http.post(
            `${BASE_URL}/api/v1/games/${gameId}/commands`,
            JSON.stringify({ command }),
            jsonHeaders
        );
    },

    getPlayer(gameId) {
        return http.get(`${BASE_URL}/api/v1/games/${gameId}/player`);
    },

    getInventory(gameId) {
        return http.get(`${BASE_URL}/api/v1/games/${gameId}/inventory`);
    },

    getFloor(gameId, includeFog = true) {
        const query = includeFog ? '' : '?include_fog=false';
        return http.get(`${BASE_URL}/api/v1/games/${gameId}/floor${query}`);
    },

    getVisibleArea(gameId) {
        return http.get(`${BASE_URL}/api/v1/games/${gameId}/floor/visible`);
    },

    getEvents(gameId, since = null, limit = null) {
        const params = [];
        if (since !== null) params.push(`since=${since}`);
        if (limit !== null) params.push(`limit=${limit}`);
        const query = params.length > 0 ? '?' + params.join('&') : '';
        return http.get(`${BASE_URL}/api/v1/games/${gameId}/events${query}`);
    },

    getLeaderboard(type = 'global', limit = 10) {
        return http.get(`${BASE_URL}/api/v1/leaderboard?type=${type}&limit=${limit}`);
    },
};

export function parseResponse(response) {
    try {
        return JSON.parse(response.body);
    } catch {
        return null;
    }
}
