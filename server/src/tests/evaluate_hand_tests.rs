
use crate::card::{Card, Rank, Suit};
use crate::hand_evaluator::HandEvaluator;


    
// Helper function to create a card
fn create_card(rank: Rank, suit: Suit) -> Card {
    Card { rank, suit: Some(suit) }
}


#[test]
fn test_royal_flush() {
    let hand = vec![
        create_card(Rank::Ten, Suit::Hearts),
        create_card(Rank::Jack, Suit::Hearts),
        create_card(Rank::Queen, Suit::Hearts),
        create_card(Rank::King, Suit::Hearts),
        create_card(Rank::Ace, Suit::Hearts),
    ];

    let hand_strength = HandEvaluator::evaluate_hand_strength(&hand);
    assert_eq!(hand_strength, 1, "Expected Royal Flush to have strength 1");
}

#[test]
fn test_straight_flush() {
    let hand = vec![
        create_card(Rank::Two, Suit::Spades),
        create_card(Rank::Three, Suit::Spades),
        create_card(Rank::Four, Suit::Spades),
        create_card(Rank::Five, Suit::Spades),
        create_card(Rank::Six, Suit::Spades),
    ];

    let hand_strength = HandEvaluator::evaluate_hand_strength(&hand);
    assert_eq!(hand_strength, 2, "Expected Straight Flush to have strength 2");
}

#[test]
fn test_four_of_a_kind() {
    let hand = vec![
        create_card(Rank::Ace, Suit::Clubs),
        create_card(Rank::Ace, Suit::Diamonds),
        create_card(Rank::Ace, Suit::Hearts),
        create_card(Rank::Ace, Suit::Spades),
        create_card(Rank::King, Suit::Clubs),
    ];

    let hand_strength = HandEvaluator::evaluate_hand_strength(&hand);
    assert_eq!(hand_strength, 3, "Expected Four of a Kind to have strength 3");
}

#[test]
fn test_full_house() {
    let hand = vec![
        create_card(Rank::Three, Suit::Clubs),
        create_card(Rank::Three, Suit::Diamonds),
        create_card(Rank::Three, Suit::Hearts),
        create_card(Rank::King, Suit::Spades),
        create_card(Rank::King, Suit::Clubs),
    ];

    let hand_strength = HandEvaluator::evaluate_hand_strength(&hand);
    assert_eq!(hand_strength, 4, "Expected Full House to have strength 4");
}

#[test]
fn test_flush() {
    let hand = vec![
        create_card(Rank::Two, Suit::Hearts),
        create_card(Rank::Five, Suit::Hearts),
        create_card(Rank::Eight, Suit::Hearts),
        create_card(Rank::Jack, Suit::Hearts),
        create_card(Rank::King, Suit::Hearts),
    ];

    let hand_strength = HandEvaluator::evaluate_hand_strength(&hand);
    assert_eq!(hand_strength, 5, "Expected Flush to have strength 5");
}

#[test]
fn test_straight() {
    let hand = vec![
        create_card(Rank::Four, Suit::Clubs),
        create_card(Rank::Five, Suit::Diamonds),
        create_card(Rank::Six, Suit::Spades),
        create_card(Rank::Seven, Suit::Hearts),
        create_card(Rank::Eight, Suit::Clubs),
    ];

    let hand_strength = HandEvaluator::evaluate_hand_strength(&hand);
    assert_eq!(hand_strength, 6, "Expected Straight to have strength 6");
}

#[test]
fn test_three_of_a_kind() {
    let hand = vec![
        create_card(Rank::Nine, Suit::Clubs),
        create_card(Rank::Nine, Suit::Diamonds),
        create_card(Rank::Nine, Suit::Hearts),
        create_card(Rank::Four, Suit::Spades),
        create_card(Rank::Two, Suit::Clubs),
    ];

    let hand_strength = HandEvaluator::evaluate_hand_strength(&hand);
    assert_eq!(hand_strength, 7, "Expected Three of a Kind to have strength 7");
}

#[test]
fn test_two_pair() {
    let hand = vec![
        create_card(Rank::Seven, Suit::Hearts),
        create_card(Rank::Seven, Suit::Diamonds),
        create_card(Rank::Ace, Suit::Clubs),
        create_card(Rank::Ace, Suit::Spades),
        create_card(Rank::Five, Suit::Clubs),
    ];

    let hand_strength = HandEvaluator::evaluate_hand_strength(&hand);
    assert_eq!(hand_strength, 8, "Expected Two Pair to have strength 8");
}

#[test]
fn test_one_pair() {
    let hand = vec![
        create_card(Rank::Four, Suit::Clubs),
        create_card(Rank::Four, Suit::Diamonds),
        create_card(Rank::Ten, Suit::Hearts),
        create_card(Rank::Jack, Suit::Spades),
        create_card(Rank::King, Suit::Clubs),
    ];

    let hand_strength = HandEvaluator::evaluate_hand_strength(&hand);
    assert_eq!(hand_strength, 9, "Expected One Pair to have strength 9");
}

#[test]
fn test_high_card() {
    let hand = vec![
        create_card(Rank::Two, Suit::Clubs),
        create_card(Rank::Five, Suit::Diamonds),
        create_card(Rank::Seven, Suit::Hearts),
        create_card(Rank::Jack, Suit::Spades),
        create_card(Rank::King, Suit::Clubs),
    ];

    let hand_strength = HandEvaluator::evaluate_hand_strength(&hand);
    assert_eq!(hand_strength, 10, "Expected High Card to have strength 10");
}
