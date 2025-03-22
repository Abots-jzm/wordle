const GAMES: &str = include_str!("../answers.txt");
use wordle::{play, solver::Solver};

fn main() {
    for answer in GAMES.split_whitespace() {
        let mut guesser = Solver::new();
        play(answer, guesser);
    }

    println!("Hello, world!");
}
