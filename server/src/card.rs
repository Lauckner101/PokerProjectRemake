use rand::seq::SliceRandom;
use rand::thread_rng;
use std::fmt;
use serde::{Serialize, Deserialize};
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Suit {
    Clubs,
    Diamonds,
    Hearts,
    Spades,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)] 
pub enum Rank {
    Two = 2,
    Three = 3,
    Four = 4,
    Five = 5,
    Six = 6,
    Seven = 7,
    Eight = 8,
    Nine = 9,
    Ten = 10,
    Jack = 11,
    Queen = 12,
    King = 13,
    Ace = 14,
    Joker = 15,
}

#[derive(Copy, Clone, Debug, PartialEq)]
#[derive(Serialize, Deserialize)]
pub struct Card {
    pub rank: Rank,
    pub suit: Option<Suit>, //joker optional
}
//diaplay trait for card struct
impl fmt::Display for Card {
    fn fmt(&self, f: &mut fmt::Formatter) -> std::fmt::Result {
        let rank_str = match self.rank {
            Rank::Jack => "Jack".to_string(),
            Rank::Queen => "Queen".to_string(),
            Rank::King => "King".to_string(),
            Rank::Ace => "Ace".to_string(),
            Rank::Joker => "Joker".to_string(),
            _ => (self.rank as u8).to_string(), // Ranks 2–10
        };

        match self.suit {
            Some(suit) => write!(f, "{}_of_{:?}", rank_str, suit),
            None => write!(f, "{}", rank_str),
        }
    }
}
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Deck {
    pub cards: Vec<Card>,
}

impl Deck {
    //create a standard deck of 52 cards
    pub fn new_standard() -> Self {
        let mut cards = Vec::new();
        for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
            for rank in [
                Rank::Two,
                Rank::Three,
                Rank::Four,
                Rank::Five,
                Rank::Six,
                Rank::Seven,
                Rank::Eight,
                Rank::Nine,
                Rank::Ten,
                Rank::Jack,
                Rank::Queen,
                Rank::King,
                Rank::Ace,
            ] {
                cards.push(Card {
                    rank,
                    suit: Some(suit),
                });
            }
        }
        Self { cards }
    }

    pub fn new_with_jokers() -> Self {
        let mut deck = Self::new_standard();
        deck.cards.push(Card {
            rank: Rank::Joker,
            suit: None,
        });
        deck.cards.push(Card {
            rank: Rank::Joker,
            suit: None,
        });
        //return
        deck
    }
    pub fn shuffle(&mut self) {
        let mut rng = thread_rng();
        self.cards.shuffle(&mut rng);
    }
    //pop one card
    pub fn pop_one_card(&mut self) -> Option<Card> {
        self.cards.pop()
    }
    pub fn remaining_cards(&self) -> usize {
        self.cards.len()
    }
}
