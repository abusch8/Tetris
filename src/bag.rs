use rand::{rngs::StdRng, seq::SliceRandom};
use strum::IntoEnumIterator;

use crate::tetromino::*;

pub struct Bag {
    pub next: Vec<Tetromino>,
    pub rest: Vec<Tetromino>,
}

impl Bag {
    pub fn new(seed: &mut StdRng) -> Self {
        let mut bag = Bag::rand_bag_gen(seed);
        Bag {
            next: bag.split_off(bag.len() - 3),
            rest: bag,
        }
    }

    fn rand_bag_gen(seed: &mut StdRng) -> Vec<Tetromino> {
        let mut bag = TetrominoVariant::iter()
            .map(|variant| Tetromino::new(variant))
            .collect::<Vec<Tetromino>>();

        bag.shuffle(seed);
        bag
    }

    pub fn get_next(&mut self, seed: &mut StdRng) -> Tetromino {
        self.next.push(self.rest.pop().unwrap());
        if self.rest.is_empty() {
            self.rest = Bag::rand_bag_gen(seed);
        }
        self.next.remove(0)
    }
}

