use crate::dealer::Dealer;
use crate::card::{Card, Rank, Suit};
use crate::player::Player;


#[test]
fn test_dealer_initialization() {
    let dealer = Dealer::new();

    // The dealer should have 52 cards in the deck initially
    assert_eq!(dealer.deck.remaining_cards(), 52);

    // The dealer should not have any community cards initially
    assert_eq!(dealer.get_community_cards().len(), 0);
}

#[test]
fn test_deal_cards() {
    let mut dealer = Dealer::new();

    // Deal 5 cards to the first player
    let dealt_cards = dealer.deal_cards(5);

    // There should be 5 cards dealt
    assert_eq!(dealt_cards.len(), 5);

    // The deck should now have 47 cards remaining
    assert_eq!(dealer.deck.remaining_cards(), 47);

    // Ensure that the dealt cards are actual cards (not None)
    for card in dealt_cards {
        assert!(card.rank != Rank::Joker || card.suit.is_some());
    }
}

#[test]
fn test_deal_community_card() {
    let mut dealer = Dealer::new();

    // Deal a community card
    dealer.deal_community_card();

    // The community cards should now have 1 card
    assert_eq!(dealer.get_community_cards().len(), 1);

    // The deck should have 51 cards remaining after dealing a community card
    assert_eq!(dealer.deck.remaining_cards(), 51);

    // The community card should be a valid card (not a Joker without a suit)
    let community_card = &dealer.get_community_cards()[0];
    assert!(community_card.rank != Rank::Joker || community_card.suit.is_some());
}

#[test]
fn test_shuffle_deck() {
    let mut dealer = Dealer::new();

    let original_deck = dealer.deck.cards.clone();

    // Shuffle the deck
    dealer.shuffle_deck();

    // After shuffling, the deck should be in a different order
    assert_ne!(dealer.deck.cards, original_deck);
}

#[test]
fn test_deal_cards_until_deck_empty() {
    let mut dealer = Dealer::new();

    // Deal all 52 cards from the deck
    let mut dealt_cards = Vec::new();
    while dealer.deck.remaining_cards() > 0 {
        dealt_cards.extend(dealer.deal_cards(1));
    }

    // After dealing all cards, the deck should be empty
    assert_eq!(dealer.deck.remaining_cards(), 0);
    // All dealt cards should be valid
    for card in dealt_cards {
        assert!(card.rank != Rank::Joker || card.suit.is_some());
    }
}

#[test]
fn test_deal_community_cards_until_deck_empty() {
    let mut dealer = Dealer::new();

    // Deal all 5 community cards
    for _ in 0..5 {
        dealer.deal_community_card();
    }

    // The dealer should have 5 community cards
    assert_eq!(dealer.get_community_cards().len(), 5);

    // The deck should have 47 cards remaining (52 - 5 for community cards)
    assert_eq!(dealer.deck.remaining_cards(), 47);
}
