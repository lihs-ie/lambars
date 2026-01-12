mod helpers;

mod coverage {
    mod commands_tests;
    mod events_tests;
    mod floor_tests;
    mod game_session_tests;
    mod health_tests;
    mod leaderboard_tests;
    mod player_tests;
}

mod scenarios {
    mod cache_tests;
    mod concurrent_tests;
    mod event_sourcing_tests;
    mod lifecycle_tests;
}
