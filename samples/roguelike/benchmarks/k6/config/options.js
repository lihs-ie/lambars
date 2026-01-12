export const smokeOptions = {
    scenarios: {
        smoke: {
            executor: 'constant-vus',
            vus: 1,
            duration: '30s',
        },
    },
    thresholds: {
        http_req_failed: ['rate<0.01'],
        http_req_duration: ['p(95)<500'],
        roguelike_health_latency: ['p(95)<100'],
        roguelike_create_game_latency: ['p(95)<500'],
        roguelike_get_game_latency: ['p(95)<300'],
        roguelike_command_latency: ['p(95)<400'],
    },
};

export const loadOptions = {
    scenarios: {
        load: {
            executor: 'ramping-vus',
            startVUs: 0,
            stages: [
                { duration: '1m', target: 10 },
                { duration: '3m', target: 10 },
                { duration: '1m', target: 20 },
                { duration: '2m', target: 20 },
                { duration: '1m', target: 0 },
            ],
        },
    },
    thresholds: {
        http_req_failed: ['rate<0.01'],
        http_req_duration: ['p(95)<500', 'p(99)<1000'],
        roguelike_health_latency: ['p(95)<100'],
        roguelike_create_game_latency: ['p(95)<500'],
        roguelike_get_game_latency: ['p(95)<200'],
        roguelike_command_latency: ['p(95)<300'],
        roguelike_errors: ['rate<0.05'],
    },
};

export const stressOptions = {
    scenarios: {
        stress: {
            executor: 'ramping-vus',
            startVUs: 0,
            stages: [
                { duration: '2m', target: 50 },
                { duration: '5m', target: 50 },
                { duration: '2m', target: 100 },
                { duration: '3m', target: 100 },
                { duration: '2m', target: 0 },
            ],
        },
    },
    thresholds: {
        http_req_failed: ['rate<0.05'],
        http_req_duration: ['p(95)<1000', 'p(99)<2000'],
        roguelike_errors: ['rate<0.10'],
    },
};

export function getOptions(scenario) {
    switch (scenario) {
        case 'load':
            return loadOptions;
        case 'stress':
            return stressOptions;
        case 'smoke':
        default:
            return smokeOptions;
    }
}
