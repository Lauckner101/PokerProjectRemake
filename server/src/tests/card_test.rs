use crate::card::{Card, Rank, Suit, Deck};


#[test]
fn test_new_standard_deck() {
    let deck = Deck::new_standard();
    // A standard deck should have 52 cards
    assert_eq!(deck.remaining_cards(), 52);

    // The deck should contain all four suits, each with 13 cards
    let mut clubs = 0;
    let mut diamonds = 0;
    let mut hearts = 0;
    let mut spades = 0;

    for card in &deck.cards {
        match card.suit {
            Some(Suit::Clubs) => clubs += 1,
            Some(Suit::Diamonds) => diamonds += 1,
            Some(Suit::Hearts) => hearts += 1,
            Some(Suit::Spades) => spades += 1,
            None => panic!("Joker should not be in a standard deck"),
        }
    }

    assert_eq!(clubs, 13);
    assert_eq!(diamonds, 13);
    assert_eq!(hearts, 13);
    assert_eq!(spades, 13);
}

#[test]
fn test_new_with_jokers_deck() {
    let deck = Deck::new_with_jokers();
    // A deck with jokers should have 54 cards
    assert_eq!(deck.remaining_cards(), 54);

    // There should be exactly 2 jokers in the deck
    let joker_count = deck.cards.iter().filter(|card| card.rank == Rank::Joker).count();
    assert_eq!(joker_count, 2);

    // The rest of the deck should be a standard deck of 52 cards
    let standard_deck_count = deck.remaining_cards() - joker_count;
    assert_eq!(standard_deck_count, 52);
}

#[test]
fn test_shuffle_deck() {
    let mut deck = Deck::new_standard();
    let original_deck = deck.cards.clone();

    deck.shuffle();
    let shuffled_deck = deck.cards.clone();

    // After shuffling, the deck should not be in the same order
    assert_ne!(original_deck, shuffled_deck);
}

#[test]
fn test_pop_one_card() {
    let mut deck = Deck::new_standard();
    
    // Pop a card from the deck
    let card = deck.pop_one_card();
    assert!(card.is_some());
    assert_eq!(deck.remaining_cards(), 51);

    // The deck should be empty when all cards are popped
    while let Some(_) = deck.pop_one_card() {}
    assert_eq!(deck.remaining_cards(), 0);

    // Popping from an empty deck should return None
    let card = deck.pop_one_card();
    assert!(card.is_none());
}
