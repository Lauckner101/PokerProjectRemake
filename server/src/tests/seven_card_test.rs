use crate::card::{Card, Rank, Suit};
use crate::dealer::Dealer;
use crate::game_variants::seven_card::SevenCardStud;
use crate::hand_evaluator::HandEvaluator;
use crate::player::Player;

// Helper function to create a mock player
fn create_mock_player(name: &str) -> Player {
    Player {
        name: name.to_string(),
        hand: Vec::new(),
        face_up_cards: Vec::new(),
        winnings: 0,
        is_active: true,
        current_bet: 0,
        game_statistics: Vec::new(),
    }
}

#[test]
fn test_new_game_initialization() {
    let players = vec![create_mock_player("Alice"), create_mock_player("Bob")];
    let game = SevenCardStud::new(players.clone());

    assert_eq!(game.players.len(), 2);
    assert_eq!(game.bet_pool, 0);
    assert_eq!(game.players[0].name, "Alice");
    assert_eq!(game.players[1].name, "Bob");
}

#[test]
fn test_deal_initial_cards() {
    let players = vec![create_mock_player("Alice"), create_mock_player("Bob")];
    let mut game = SevenCardStud::new(players);

    game.deal_initial_cards();

    for player in &game.players {
        assert_eq!(player.hand.len(), 2); // 2 face-down cards
        assert_eq!(player.face_up_cards.len(), 1); // 1 face-up card
        assert!(player.is_active);
    }
}

#[test]
fn test_deal_face_up_card() {
    let players = vec![create_mock_player("Alice"), create_mock_player("Bob")];
    let mut game = SevenCardStud::new(players);

    game.deal_initial_cards(); // Start with 1 face-up
    game.deal_face_up_card(); // Add another face-up

    for player in &game.players {
        assert_eq!(player.hand.len(), 2); // Still 2 face-down
        assert_eq!(player.face_up_cards.len(), 2); // Now 2 face-up
    }
}

#[test]
fn test_deal_river() {
    let players = vec![create_mock_player("Alice"), create_mock_player("Bob")];
    let mut game = SevenCardStud::new(players);

    game.deal_initial_cards(); // 2 down, 1 up
    game.deal_river(); // Add 1 more face-down

    for player in &game.players {
        assert_eq!(player.hand.len(), 3); // 3 face-down after river
        assert_eq!(player.face_up_cards.len(), 1); // Still 1 face-up
    }
}

#[test]
fn test_determine_winner_single() {
    let players = vec![create_mock_player("Alice"), create_mock_player("Bob")];
    let mut game = SevenCardStud::new(players);

    // Mock hands for testing
    game.players[0].hand = vec![
        Card {
            rank: Rank::Ace,
            suit: Some(Suit::Spades),
        },
        Card {
            rank: Rank::King,
            suit: Some(Suit::Spades),
        },
        Card {
            rank: Rank::Queen,
            suit: Some(Suit::Spades),
        },
    ];
    game.players[0].face_up_cards = vec![
        Card {
            rank: Rank::Jack,
            suit: Some(Suit::Spades),
        },
        Card {
            rank: Rank::Ten,
            suit: Some(Suit::Spades),
        },
        Card {
            rank: Rank::Nine,
            suit: Some(Suit::Spades),
        },
        Card {
            rank: Rank::Eight,
            suit: Some(Suit::Spades),
        },
    ]; // Straight Flush (10 to A, Spades)

    game.players[1].hand = vec![
        Card {
            rank: Rank::Two,
            suit: Some(Suit::Hearts),
        },
        Card {
            rank: Rank::Three,
            suit: Some(Suit::Hearts),
        },
        Card {
            rank: Rank::Four,
            suit: Some(Suit::Hearts),
        },
    ];
    game.players[1].face_up_cards = vec![
        Card {
            rank: Rank::Five,
            suit: Some(Suit::Hearts),
        },
        Card {
            rank: Rank::Six,
            suit: Some(Suit::Hearts),
        },
        Card {
            rank: Rank::Seven,
            suit: Some(Suit::Hearts),
        },
        Card {
            rank: Rank::Eight,
            suit: Some(Suit::Hearts),
        },
    ]; // Straight (2 to 6, mixed suits)

    let winners = game.determine_winner();
    assert_eq!(winners.len(), 1);
    assert_eq!(winners[0].name, "Alice"); // Alice wins with Straight Flush
}

#[test]
fn test_determine_winner_tie() {
    let players = vec![create_mock_player("Alice"), create_mock_player("Bob")];
    let mut game = SevenCardStud::new(players);

    // Both players have the same hand strength (e.g., two pair, Aces and Kings)
    game.players[0].hand = vec![
        Card {
            rank: Rank::Ace,
            suit: Some(Suit::Spades),
        },
        Card {
            rank: Rank::King,
            suit: Some(Suit::Hearts),
        },
        Card {
            rank: Rank::Ten,
            suit: Some(Suit::Spades),
        },
    ];
    game.players[0].face_up_cards = vec![
        Card {
            rank: Rank::Ace,
            suit: Some(Suit::Hearts),
        },
        Card {
            rank: Rank::King,
            suit: Some(Suit::Spades),
        },
        Card {
            rank: Rank::Nine,
            suit: Some(Suit::Diamonds),
        },
        Card {
            rank: Rank::Eight,
            suit: Some(Suit::Clubs),
        },
    ];

    game.players[1].hand = vec![
        Card {
            rank: Rank::Ace,
            suit: Some(Suit::Clubs),
        },
        Card {
            rank: Rank::King,
            suit: Some(Suit::Diamonds),
        },
        Card {
            rank: Rank::Ten,
            suit: Some(Suit::Clubs),
        },
    ];
    game.players[1].face_up_cards = vec![
        Card {
            rank: Rank::Ace,
            suit: Some(Suit::Diamonds),
        },
        Card {
            rank: Rank::King,
            suit: Some(Suit::Clubs),
        },
        Card {
            rank: Rank::Nine,
            suit: Some(Suit::Hearts),
        },
        Card {
            rank: Rank::Eight,
            suit: Some(Suit::Spades),
        },
    ];

    let winners = game.determine_winner();
    assert_eq!(winners.len(), 2); // Tie between Alice and Bob
    assert!(winners.iter().any(|p| p.name == "Alice"));
    assert!(winners.iter().any(|p| p.name == "Bob"));
}

#[test]
fn test_winnings_distribution() {
    let players = vec![create_mock_player("Alice"), create_mock_player("Bob")];
    let mut game = SevenCardStud::new(players);

    game.bet_pool = 100;
    game.players[0].hand = vec![
        Card {
            rank: Rank::Ace,
            suit: Some(Suit::Spades),
        },
        Card {
            rank: Rank::King,
            suit: Some(Suit::Spades),
        },
        Card {
            rank: Rank::Queen,
            suit: Some(Suit::Spades),
        },
    ];
    game.players[0].face_up_cards = vec![
        Card {
            rank: Rank::Jack,
            suit: Some(Suit::Spades),
        },
        Card {
            rank: Rank::Ten,
            suit: Some(Suit::Spades),
        },
        Card {
            rank: Rank::Nine,
            suit: Some(Suit::Spades),
        },
        Card {
            rank: Rank::Eight,
            suit: Some(Suit::Spades),
        },
    ]; // Straight Flush

    game.players[1].hand = vec![
        Card {
            rank: Rank::Two,
            suit: Some(Suit::Hearts),
        },
        Card {
            rank: Rank::Three,
            suit: Some(Suit::Hearts),
        },
        Card {
            rank: Rank::Four,
            suit: Some(Suit::Hearts),
        },
    ];
    game.players[1].face_up_cards = vec![
        Card {
            rank: Rank::Five,
            suit: Some(Suit::Hearts),
        },
        Card {
            rank: Rank::Six,
            suit: Some(Suit::Hearts),
        },
        Card {
            rank: Rank::Seven,
            suit: Some(Suit::Hearts),
        },
        Card {
            rank: Rank::Eight,
            suit: Some(Suit::Hearts),
        },
    ]; // Straight

    let initial_winnings_alice = game.players[0].winnings;
    let initial_winnings_bob = game.players[1].winnings;

    // Simulate end of game by calling determine_winner and distributing winnings
    let winners = game.determine_winner();
    let winner_names: Vec<String> = winners.iter().map(|winner| winner.name.clone()).collect();
    let winnings_per_player = game.bet_pool / winner_names.len() as u32;
    for winner_name in winner_names {
        for player in &mut game.players {
            if player.name == winner_name {
                player.add_winnings(winnings_per_player);
            }
        }
    }
    game.bet_pool = 0;

    assert_eq!(game.bet_pool, 0); // Pot should be reset
    assert_eq!(game.players[0].winnings, initial_winnings_alice + 100); // Alice wins all
    assert_eq!(game.players[1].winnings, initial_winnings_bob); // Bob wins nothing
}
