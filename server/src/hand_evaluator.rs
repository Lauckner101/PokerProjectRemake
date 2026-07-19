use crate::card::{Card, Rank, Suit};
use itertools::Itertools;

pub struct HandEvaluator;

impl HandEvaluator {
    /// Evaluates a poker hand and returns a numerical ranking (lower is better)
    pub fn evaluate_hand_strength(hand: &[Card]) -> u32 {
        if hand.len() < 5 {
            return 10; // high card as default for invalid hands
        }

        // generate all 5-card combinations from the 7 cards
        let mut best_score = u32::MAX;
        for combo in hand.iter().combinations(5) {
            let five_card_hand: Vec<Card> = combo.into_iter().cloned().collect();
            let score = Self::evaluate_five_card_hand(&five_card_hand);
            best_score = best_score.min(score);
        }
        best_score
    }

    fn evaluate_five_card_hand(hand: &[Card]) -> u32 {
        let mut sorted_hand = hand.to_vec();
        sorted_hand.sort_by(|a, b| a.rank.cmp(&b.rank));

        let suits: Vec<Option<Suit>> = sorted_hand.iter().map(|card| card.suit).collect();
        let ranks: Vec<Rank> = sorted_hand.iter().map(|card| card.rank).collect();

        // Royal Flush
        if suits.iter().all(|&s| s == suits[0])
            && ranks == vec![Rank::Ten, Rank::Jack, Rank::Queen, Rank::King, Rank::Ace]
        {
            return 1;
        }

        // Straight Flush
        if suits.iter().all(|&s| s == suits[0]) && Self::is_consecutive(&ranks) {
            return 2;
        }

        // Four of a Kind
        if Self::has_n_of_a_kind(&ranks, 4) {
            return 3;
        }

        // Full House
        if Self::has_full_house(&ranks) {
            return 4;
        }

        // Flush
        if suits.iter().all(|&s| s == suits[0]) {
            return 5;
        }

        // Straight
        if Self::is_consecutive(&ranks) {
            return 6;
        }

        // Three of a Kind
        if Self::has_n_of_a_kind(&ranks, 3) {
            return 7;
        }

        // Two Pair
        if Self::two_pair(&ranks) {
            return 8;
        }

        // One Pair
        if Self::has_n_of_a_kind(&ranks, 2) {
            return 9;
        }

        // High Card
        10
    }

    fn is_consecutive(ranks: &[Rank]) -> bool {
        let mut sorted = ranks.to_vec();
        sorted.sort();
        sorted
            .windows(2)
            .all(|pair| (pair[1] as u8) - (pair[0] as u8) == 1)
    }

    fn has_n_of_a_kind(ranks: &[Rank], n: usize) -> bool {
        // Sort the ranks to bring duplicates together
        let mut sorted_ranks = ranks.to_vec();
        sorted_ranks.sort();

        // Iterate through the sorted ranks and count consecutive duplicates
        let mut count = 1;
        for i in 1..sorted_ranks.len() {
            if sorted_ranks[i] == sorted_ranks[i - 1] {
                count += 1;
            } else {
                // If the count matches n then we found n of a kind
                if count == n {
                    return true;
                }
                count = 1; // Reset count
            }
        }

        // Final check for the last group of consecutive duplicates
        count == n
    }

    fn has_full_house(ranks: &[Rank]) -> bool {
        let has_three_of_a_kind = Self::has_n_of_a_kind(ranks, 3); // Check for three of a kind
        let has_pair = Self::has_n_of_a_kind(ranks, 2); // Check for a pair

        // A full house requires both a three of a kind and a pair
        has_three_of_a_kind && has_pair
    }

    fn two_pair(ranks: &[Rank]) -> bool {
        let mut sorted_ranks = ranks.to_vec();
        sorted_ranks.sort();

        let mut pair_count = 0;
        let mut count = 1;

        for i in 1..sorted_ranks.len() {
            if sorted_ranks[i] == sorted_ranks[i - 1] {
                count += 1;
            } else {
                // If we encounter a pair, increment the pair count
                if count == 2 {
                    pair_count += 1;
                }
                count = 1;
            }
        }

        // Check the last group in case the hand ends with a pair
        if count == 2 {
            pair_count += 1;
        }

        // We need exactly two pairs
        pair_count == 2
    }
}
