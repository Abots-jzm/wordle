use super::{Guess, Guesser};

pub struct Solver;

impl Solver {
    pub fn new() -> Self {
        Solver
    }
}

impl Guesser for Solver {
    fn guess(&mut self, history: &[Guess]) -> String {
        todo!();
    }
}
