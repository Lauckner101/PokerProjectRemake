use crate::player::Player;
use crate::card::{Card, Rank, Suit};

fn create_test_player(name: &str) -> Player {
    Player::new(name)
}

#[test]
fn test_player_initialization() {
    let player = create_test_player("Player 1");

    // The player's hand should be empty initially
    assert!(player.hand.is_empty());

    // The player's winnings should be 0
    assert_eq!(player.winnings, 0);

    // The player should be active
    assert!(player.is_active);

    // The player's current bet should be 0
    assert_eq!(player.current_bet, 0);

    // The player should not have any game statistics initially
    assert!(player.game_statistics.is_empty());
}

#[test]
fn test_add_cards() {
    let mut player = create_test_player("Player 1");

    let cards = vec![
        Card { rank: Rank::Ace, suit: Some(Suit::Spades) },
        Card { rank: Rank::King, suit: Some(Suit::Hearts) },
    ];

    player.add_cards(cards.clone());

    // The player's hand should contain the added cards
    assert_eq!(player.show_hand(), cards);

    // The player's hand size should match the number of cards added
    assert_eq!(player.hand.len(), 2);
}

#[test]
fn test_add_winnings() {
    let mut player = create_test_player("Player 1");
    player.start_new_game();

    // Add some winnings to the player
    player.add_winnings(100);

    // The player's total winnings should be updated
    assert_eq!(player.winnings, 100);

    // The player's game statistics should reflect the new winnings
    assert_eq!(player.game_statistics.len(), 1);
    assert_eq!(player.game_statistics.last().unwrap().total_winnings, 100);

    // Add more winnings
    player.add_winnings(50);

    // The player's total winnings should now be 150
    assert_eq!(player.winnings, 150);
    assert_eq!(player.game_statistics[0].total_winnings, 150);
}

#[test]
fn test_fold() {
    let mut player = create_test_player("Player 1");

    // Fold the player
    player.fold();

    // The player should no longer be active
    assert!(!player.is_active);

    // Check that calling is_active() also reflects the fold
    assert_eq!(player.is_active(), false);
}

#[test]
fn test_bet() {
    let mut player = create_test_player("Player 1");
    player.start_new_game();

    // Place a bet of 50
    player.bet(50);

    // The player's current bet should be updated
    assert_eq!(player.current_bet, 50);

    // The player's total bets in game statistics should be updated
    assert_eq!(player.game_statistics.len(), 1);
    assert_eq!(player.game_statistics[0].total_bets, 50);
}

#[test]
fn test_check() {
    let mut player = create_test_player("Player 1");

    // A player who has not placed a bet should check (i.e., current_bet == 0)
    assert!(player.check());

    // Place a bet of 50
    player.bet(50);

    // After placing a bet, check should return false
    assert!(!player.check());
}

#[test]
fn test_call() {
    let mut player = create_test_player("Player 1");
    player.start_new_game();
    
    // Player calls to match the highest bet of 100
    player.bet(50);
    player.call(100);

    // The player's bet should now match the highest bet
    assert_eq!(player.current_bet, 100);

    // The player's total bets should reflect the change
    assert_eq!(player.game_statistics[0].total_bets, 100);
}

#[test]
fn test_clear_hand() {
    let mut player = create_test_player("Player 1");

    let cards = vec![
        Card { rank: Rank::Ace, suit: Some(Suit::Spades) },
        Card { rank: Rank::King, suit: Some(Suit::Hearts) },
    ];

    player.add_cards(cards.clone());

    // Ensure the player has cards before clearing the hand
    assert_eq!(player.hand.len(), 2);

    // Clear the player's hand
    player.clear_hand();

    // The player's hand should now be empty
    assert!(player.hand.is_empty());
}

#[test]
fn test_reset_for_new_round() {
    let mut player = create_test_player("Player 1");

    let cards = vec![
        Card { rank: Rank::Ace, suit: Some(Suit::Spades) },
        Card { rank: Rank::King, suit: Some(Suit::Hearts) },
    ];

    player.add_cards(cards.clone());
    player.bet(50);

    // Reset the player for a new round
    player.reset_for_new_round();

    // The player's hand should be empty
    assert!(player.hand.is_empty());

    // The player should be active again
    assert!(player.is_active);

    // The player's current bet should be reset to 0
    assert_eq!(player.current_bet, 0);
}

#[test]
fn test_start_new_game() {
    let mut player = create_test_player("Player 1");

    // Start a new game
    player.start_new_game();

    // The player's game statistics should contain one entry for the new game
    assert_eq!(player.game_statistics.len(), 1);
    assert_eq!(player.game_statistics[0].total_bets, 0);
    assert_eq!(player.game_statistics[0].total_winnings, 0);
}