const GAMES: &str = include_str!("../answers.txt");
use wordle::{solver::Solver, Wordle};

fn main() {
    let wordle = Wordle::new();
    let mut total_score = 0;
    let mut games_played = 0;

    for answer in GAMES.split_whitespace() {
        let guesser = Solver::new();
        if let Some(score) = wordle.play(answer, guesser) {
            println!("guessed '{}' in {} tries\n", answer, score);
            total_score += score;
            games_played += 1;
        } else {
            eprintln!("failed to guess");
        }
    }

    if games_played > 0 {
        let average = total_score as f64 / games_played as f64;
        println!("Average score: {:.2} tries per word", average);
    } else {
        println!("No successful games played");
    }
}
