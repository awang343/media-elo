use media_elo_core::Row;
use rand::seq::SliceRandom;
use rand::Rng;
use std::collections::VecDeque;
use uuid::Uuid;

pub const HISTORY_SIZE: usize = 30;
pub const RANDOM_PAIR_CHANCE: f64 = 0.2;
pub const PAIR_ATTEMPTS: usize = 20;

pub struct Pairer {
    recent: VecDeque<(Uuid, Uuid)>,
    history_size: usize,
}

impl Default for Pairer {
    fn default() -> Self {
        Self::new()
    }
}

impl Pairer {
    pub fn new() -> Self {
        Self {
            recent: VecDeque::with_capacity(HISTORY_SIZE),
            history_size: HISTORY_SIZE,
        }
    }

    fn pair_id(a: &Row, b: &Row) -> (Uuid, Uuid) {
        if a.id <= b.id {
            (a.id, b.id)
        } else {
            (b.id, a.id)
        }
    }

    fn pick_candidate_idx<R: Rng>(rng: &mut R, rows: &[Row], pool: &[usize]) -> usize {
        let weights: Vec<f64> = pool
            .iter()
            .map(|&i| 1.0 / (1.0 + rows[i].matches as f64))
            .collect();
        weighted_pick_idx(rng, pool, &weights)
    }

    fn weighted_opponent_idx<R: Rng>(
        rng: &mut R,
        rows: &[Row],
        a_idx: usize,
        candidates: &[usize],
    ) -> usize {
        let a_elo = rows[a_idx].elo;
        let weights: Vec<f64> = candidates
            .iter()
            .map(|&i| {
                let b = &rows[i];
                (1.0 / (1.0 + b.matches as f64))
                    * (1.0 / (1.0 + (a_elo - b.elo).abs()))
            })
            .collect();
        weighted_pick_idx(rng, candidates, &weights)
    }

    /// Picks a pair from `eligible` (indices into `rows`). Returned values are row IDs.
    pub fn pick(&self, rows: &[Row], eligible: &[usize]) -> Option<(Uuid, Uuid)> {
        if eligible.len() < 2 {
            return None;
        }
        let mut rng = rand::thread_rng();

        let a_idx = Self::pick_candidate_idx(&mut rng, rows, eligible);
        let a_type = rows[a_idx].type_.clone();

        let same_type: Vec<usize> = eligible
            .iter()
            .copied()
            .filter(|&i| i != a_idx && rows[i].type_ == a_type)
            .collect();
        if same_type.is_empty() {
            return None;
        }

        let unmatched: Vec<usize> = same_type
            .iter()
            .copied()
            .filter(|&i| rows[i].matches == 0)
            .collect();

        if !unmatched.is_empty() {
            for _ in 0..PAIR_ATTEMPTS {
                let &b_idx = unmatched.choose(&mut rng).unwrap();
                let pid = Self::pair_id(&rows[a_idx], &rows[b_idx]);
                if !self.recent.contains(&pid) {
                    return Some((rows[a_idx].id, rows[b_idx].id));
                }
            }
        }

        for _ in 0..PAIR_ATTEMPTS {
            let b_idx = if rng.gen::<f64>() < RANDOM_PAIR_CHANCE {
                *same_type.choose(&mut rng).unwrap()
            } else {
                Self::weighted_opponent_idx(&mut rng, rows, a_idx, &same_type)
            };
            let pid = Self::pair_id(&rows[a_idx], &rows[b_idx]);
            if !self.recent.contains(&pid) {
                return Some((rows[a_idx].id, rows[b_idx].id));
            }
        }
        None
    }

    pub fn forget_last(&mut self) -> Option<(Uuid, Uuid)> {
        self.recent.pop_back()
    }

    pub fn remember(&mut self, a: &Row, b: &Row) {
        if self.recent.len() == self.history_size {
            self.recent.pop_front();
        }
        self.recent.push_back(Self::pair_id(a, b));
    }
}

fn weighted_pick_idx<R: Rng>(rng: &mut R, pool: &[usize], weights: &[f64]) -> usize {
    let total: f64 = weights.iter().sum();
    if total <= 0.0 {
        return pool[0];
    }
    let mut t = rng.gen::<f64>() * total;
    for (i, w) in weights.iter().enumerate() {
        t -= w;
        if t <= 0.0 {
            return pool[i];
        }
    }
    pool[pool.len() - 1]
}
