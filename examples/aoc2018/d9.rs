use std::collections::VecDeque;

// Input, not worth bothering to parse file.
const PLAYERS: usize = 452;
const MARBLES: u64 = 70784;

fn rotate<T>(deque: &mut VecDeque<T>, rotation: isize) {
    if rotation > 0 {
        for _ in 0..rotation {
            let tmp = deque.pop_front().unwrap();
            deque.push_back(tmp);
        }
    } else {
        for _ in 0..-rotation {
            let tmp = deque.pop_back().unwrap();
            deque.push_front(tmp);
        }
    }
}

fn run_game(marbles: u64) -> u64 {
    let mut circle = VecDeque::new();
    circle.push_back(0);
    let mut p = 0;
    let mut player_scores = vec![0u64; PLAYERS];
    let mut m = 1;
    while m <= marbles {
        if m % 23 == 0 {
            player_scores[p] += m;
            rotate(&mut circle, -7);
            player_scores[p] += circle.pop_back().unwrap();
            rotate(&mut circle, 1);
        } else {
            rotate(&mut circle, 1);
            circle.push_back(m);
        }
        p = (p + 1) % PLAYERS;
        m += 1;
    }
    let mut max_score = 0;
    for s in player_scores { if s > max_score {max_score = s;} }
    return max_score;
}

fn main() {
    println!("Part1: {}", run_game(MARBLES));
    println!("Part2: {}", run_game(MARBLES*100));
}
