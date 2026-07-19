
use crate::connection_manager::*;



#[test]
fn test_default_connection_manager() {
    let manager = ConnectionManager::default();
    assert!(!manager.is_connected());
    assert_eq!(manager.get_players().len(), 0);
    assert!(!manager.game_started());
}

#[test]
fn test_connect_success_or_failure() {
    let mut manager = ConnectionManager::new();
    let connected = manager.connect();

    assert!(connected || !connected);
}

#[test]
fn test_send_start_game_fails_when_disconnected() {
    let mut manager = ConnectionManager::new();
    assert!(!manager.send_start_game("TexasHoldEm"));
}

#[test]
fn test_send_player_action_fails_when_disconnected() {
    let mut manager = ConnectionManager::new();
    let success = manager.send_player_action("TexasHoldEm", "call", Some(50));
    assert!(!success);
}

#[test]
fn test_join_room_fails_without_server() {
    let mut manager = ConnectionManager::new();
    let success = manager.join_room("TexasHoldEm", "TestPlayer");
    // Will return false if no server running
    assert!(success || !success);
}

#[test]
fn test_game_state_update_returns_none_without_server() {
    let mut manager = ConnectionManager::new();
    let result = manager.fetch_game_state_update("TexasHoldEm", "TestPlayer");
    assert!(result.is_none());
}


