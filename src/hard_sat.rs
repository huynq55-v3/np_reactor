//! Lõi sinh bài 3-SAT "khó": đúng 1 nghiệm, ở ngưỡng chuyển pha, chọn bài khó nhất trong K bài,
//! và WalkSAT để làm thước đo khi chơi.
//!
//! Biểu diễn: n <= 64 biến, mỗi phép gán là một u64, mỗi mệnh đề là cặp bitmask (pos, neg).

use ::rand::rngs::StdRng;
use ::rand::{Rng, SeedableRng};
use std::collections::HashSet;

pub const MAX_VARS: usize = 64;
pub const RATIO: f64 = 4.26;
/// Mức nhiễu WalkSAT tốt cho 3-SAT ngẫu nhiên gần ngưỡng.
pub const NOISE: f64 = 0.567;
/// Số lượt WalkSAT để chấm độ khó mỗi bài ứng viên.
pub const SELECT_RUNS: usize = 20;
/// Số lượt WalkSAT tham chiếu, chạy từ đúng trạng thái xuất phát của người chơi.
pub const REF_RUNS: usize = 100;
/// Trần số bước cho một lượt WalkSAT.
pub const FLIP_CAP: u32 = 200_000;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct BClause {
    pub pos: u64,
    pub neg: u64,
}

impl BClause {
    #[inline]
    pub fn sat(&self, x: u64) -> bool {
        (self.pos & x) | (self.neg & !x) != 0
    }
    #[inline]
    pub fn vars(&self) -> u64 {
        self.pos | self.neg
    }
    /// Literal của biến v trong mệnh đề này có đang đúng dưới phép gán x không.
    #[inline]
    fn lit_true(&self, v: usize, x: u64) -> bool {
        let b = 1u64 << v;
        (self.pos & b != 0 && x & b != 0) || (self.neg & b != 0 && x & b == 0)
    }
}

pub fn full_mask(n: usize) -> u64 {
    if n >= 64 { u64::MAX } else { (1u64 << n) - 1 }
}

pub fn clause_count(n: usize) -> usize {
    let m = (n as f64 * RATIO).ceil() as usize;
    // Không thể có nhiều mệnh đề khác nhau hơn C(n,3)*8
    let max = if n >= 3 { n * (n - 1) * (n - 2) / 6 * 8 } else { 0 };
    m.min(max)
}

/// 3-SAT ngẫu nhiên thuần (không cài nghiệm), không trùng mệnh đề.
pub fn random_3sat(n: usize, m: usize, rng: &mut impl Rng) -> Vec<BClause> {
    let mut seen = HashSet::with_capacity(m);
    let mut out = Vec::with_capacity(m);
    while out.len() < m {
        let idx = ::rand::seq::index::sample(rng, n, 3);
        let (mut pos, mut neg) = (0u64, 0u64);
        for v in idx.iter() {
            if rng.gen_bool(0.5) { pos |= 1 << v } else { neg |= 1 << v }
        }
        let c = BClause { pos, neg };
        if seen.insert(c) {
            out.push(c);
        }
    }
    out
}

/// DPLL đếm nghiệm có lan truyền đơn vị, dừng khi đạt `limit`.
/// Nghiệm cuối cùng tìm được ghi vào `found`.
pub fn count_upto(cls: &[BClause], full: u64, asg: u64, val: u64, limit: usize, found: &mut u64) -> usize {
    let (mut asg, mut val) = (asg, val);
    let mut branch_clause: Option<u64>;
    loop {
        let mut changed = false;
        let mut best_free = u32::MAX;
        branch_clause = None;
        for c in cls {
            if (c.pos & asg & val) | (c.neg & asg & !val) != 0 {
                continue; // đã đúng
            }
            let fp = c.pos & !asg;
            let fneg = c.neg & !asg;
            let free = fp | fneg;
            match free.count_ones() {
                0 => return 0, // mâu thuẫn
                1 => {
                    asg |= free;
                    if fp != 0 { val |= free } else { val &= !free }
                    changed = true;
                }
                k => {
                    if k < best_free {
                        best_free = k;
                        branch_clause = Some(free);
                    }
                }
            }
        }
        if !changed {
            break;
        }
    }
    if asg == full {
        *found = val;
        return 1;
    }
    // Rẽ nhánh trên một biến của mệnh đề ngắn nhất còn mở; nếu mọi mệnh đề đã đúng thì lấy biến tự do bất kỳ.
    let pool = branch_clause.unwrap_or(!asg & full);
    let b = 1u64 << pool.trailing_zeros();
    let a = count_upto(cls, full, asg | b, val | b, limit, found);
    if a >= limit {
        return a;
    }
    a + count_upto(cls, full, asg | b, val & !b, limit - a, found)
}

/// Bài đã dựng sẵn danh sách xuất hiện để WalkSAT chạy nhanh.
pub struct Instance<'a> {
    pub n: usize,
    pub cls: &'a [BClause],
    occ: Vec<Vec<usize>>,
}

impl<'a> Instance<'a> {
    pub fn new(n: usize, cls: &'a [BClause]) -> Self {
        let mut occ = vec![Vec::new(); n];
        for (i, c) in cls.iter().enumerate() {
            let mut vs = c.vars();
            while vs != 0 {
                let v = vs.trailing_zeros() as usize;
                occ[v].push(i);
                vs &= vs - 1;
            }
        }
        Self { n, cls, occ }
    }

    /// WalkSAT (SKC). Trả về số lần đảo bit tới khi giải xong (hoặc `cap`).
    pub fn walksat(&self, start: u64, noise: f64, cap: u32, rng: &mut impl Rng) -> u32 {
        let cls = self.cls;
        let mut x = start;
        let mut numtrue: Vec<u8> = cls
            .iter()
            .map(|c| ((c.pos & x) | (c.neg & !x)).count_ones() as u8)
            .collect();
        let mut unsat: Vec<usize> = Vec::new();
        let mut where_: Vec<usize> = vec![usize::MAX; cls.len()];
        for (i, &t) in numtrue.iter().enumerate() {
            if t == 0 {
                where_[i] = unsat.len();
                unsat.push(i);
            }
        }

        let mut vars = [0usize; 3];
        let mut brk = [0u32; 3];
        for step in 0..cap {
            if unsat.is_empty() {
                return step;
            }
            let c = cls[unsat[rng.gen_range(0..unsat.len())]];
            let mut k = 0;
            let mut vs = c.vars();
            while vs != 0 {
                vars[k] = vs.trailing_zeros() as usize;
                vs &= vs - 1;
                k += 1;
            }
            // break(v) = số mệnh đề mà literal của v là literal đúng duy nhất
            let mut min_b = u32::MAX;
            for j in 0..k {
                let v = vars[j];
                brk[j] = self.occ[v]
                    .iter()
                    .filter(|&&d| numtrue[d] == 1 && cls[d].lit_true(v, x))
                    .count() as u32;
                min_b = min_b.min(brk[j]);
            }
            let v = if min_b > 0 && rng.gen_bool(noise) {
                vars[rng.gen_range(0..k)]
            } else {
                let cands: Vec<usize> = (0..k).filter(|&j| brk[j] == min_b).map(|j| vars[j]).collect();
                cands[rng.gen_range(0..cands.len())]
            };

            // Đảo v và cập nhật numtrue / danh sách mệnh đề sai
            for &d in &self.occ[v] {
                if cls[d].lit_true(v, x) {
                    numtrue[d] -= 1;
                    if numtrue[d] == 0 {
                        where_[d] = unsat.len();
                        unsat.push(d);
                    }
                } else {
                    numtrue[d] += 1;
                    if numtrue[d] == 1 {
                        let p = where_[d];
                        let last = *unsat.last().unwrap();
                        unsat.swap_remove(p);
                        if last != d {
                            where_[last] = p;
                        }
                        where_[d] = usize::MAX;
                    }
                }
            }
            x ^= 1u64 << v;
        }
        cap
    }
}

pub struct Puzzle {
    pub n: usize,
    pub clauses: Vec<BClause>,
    pub solution: u64,
    pub start: u64,
    /// Trung vị WalkSAT (từ điểm xuất phát ngẫu nhiên) dùng để chọn bài.
    pub hardness: u32,
    /// REF_RUNS lượt WalkSAT từ đúng `start`, đã sắp xếp tăng dần.
    pub ref_runs: Vec<u32>,
    pub seed: u64,
    pub tried: usize,
}

fn median(v: &mut [u32]) -> u32 {
    v.sort_unstable();
    v[v.len() / 2]
}

/// Sinh K bài có đúng 1 nghiệm ở tỉ lệ RATIO, giữ bài WalkSAT thấy khó nhất.
/// Cùng (n, k, seed) luôn ra cùng một bài.
pub fn generate_hard(n: usize, k: usize, seed: u64) -> Puzzle {
    assert!((3..=MAX_VARS).contains(&n));
    let mut rng = StdRng::seed_from_u64(seed);
    let full = full_mask(n);
    let m = clause_count(n);
    let mut best: Option<(Vec<BClause>, u64, u32)> = None;
    let (mut got, mut tried) = (0, 0);
    while got < k.max(1) {
        tried += 1;
        let cls = random_3sat(n, m, &mut rng);
        let mut sol = 0;
        if count_upto(&cls, full, 0, 0, 2, &mut sol) != 1 {
            continue;
        }
        got += 1;
        let inst = Instance::new(n, &cls);
        let mut runs: Vec<u32> = (0..SELECT_RUNS)
            .map(|_| {
                let s = rng.r#gen::<u64>() & full;
                inst.walksat(s, NOISE, FLIP_CAP, &mut rng)
            })
            .collect();
        let med = median(&mut runs);
        if best.as_ref().map_or(true, |b| med > b.2) {
            best = Some((cls, sol, med));
        }
    }
    let (clauses, solution, hardness) = best.unwrap();
    // Điểm xuất phát: ngẫu nhiên, nhưng không trùng nghiệm
    let mut start = rng.r#gen::<u64>() & full;
    while start == solution {
        start = rng.r#gen::<u64>() & full;
    }
    let inst = Instance::new(n, &clauses);
    let mut ref_runs: Vec<u32> = (0..REF_RUNS)
        .map(|_| inst.walksat(start, NOISE, FLIP_CAP, &mut rng))
        .collect();
    ref_runs.sort_unstable();
    Puzzle { n, clauses, solution, start, hardness, ref_runs, seed, tried }
}

impl Puzzle {
    /// Số bước tối thiểu: khoảng cách Hamming từ điểm xuất phát tới nghiệm duy nhất.
    pub fn min_steps(&self) -> u32 {
        (self.start ^ self.solution).count_ones()
    }
    pub fn quantile(&self, q: f64) -> u32 {
        let i = ((self.ref_runs.len() - 1) as f64 * q).round() as usize;
        self.ref_runs[i]
    }
    /// Tỉ lệ lượt WalkSAT chậm hơn hẳn người chơi.
    pub fn faster_than(&self, steps: u32) -> f64 {
        self.ref_runs.iter().filter(|&&r| r > steps).count() as f64 / self.ref_runs.len() as f64
    }
    /// p-value một phía cho giả thuyết "người chơi chỉ ngang một lượt WalkSAT".
    /// Nhỏ = người chơi nhanh bất thường. Bảo thủ: (#lượt <= steps + 1) / (R + 1).
    pub fn p_value(&self, steps: u32) -> f64 {
        let le = self.ref_runs.iter().filter(|&&r| r <= steps).count();
        (le + 1) as f64 / (self.ref_runs.len() + 1) as f64
    }
}

/// Gộp p-value của nhiều ván độc lập (phương pháp Fisher, chi-bình-phương 2k bậc tự do).
pub fn fisher_combined(ps: &[f64]) -> f64 {
    if ps.is_empty() {
        return 1.0;
    }
    let x: f64 = ps.iter().map(|p| -2.0 * p.ln()).sum();
    let half = x / 2.0;
    let mut term = 1.0;
    let mut sum = 1.0;
    for j in 1..ps.len() {
        term *= half / j as f64;
        sum += term;
    }
    ((-half).exp() * sum).min(1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn brute_count(n: usize, cls: &[BClause]) -> usize {
        (0..(1u64 << n)).filter(|&x| cls.iter().all(|c| c.sat(x))).count()
    }

    #[test]
    fn dpll_matches_brute_force() {
        let mut rng = StdRng::seed_from_u64(7);
        for n in 4..=14 {
            for _ in 0..40 {
                let cls = random_3sat(n, clause_count(n), &mut rng);
                let mut f = 0;
                let real = brute_count(n, &cls);
                assert_eq!(count_upto(&cls, full_mask(n), 0, 0, usize::MAX, &mut f), real);
                assert_eq!(count_upto(&cls, full_mask(n), 0, 0, 2, &mut f), real.min(2));
            }
        }
    }

    #[test]
    fn puzzle_is_unique_and_walksat_solves() {
        for n in [5, 12, 20] {
            let p = generate_hard(n, 3, 42);
            assert_eq!(brute_count(n, &p.clauses), 1);
            assert!(p.clauses.iter().all(|c| c.sat(p.solution)));
            assert!(p.ref_runs.iter().all(|&r| r < FLIP_CAP));
            assert!(p.min_steps() >= 1);
        }
    }

    #[test]
    fn deterministic_by_seed() {
        let a = generate_hard(16, 2, 99);
        let b = generate_hard(16, 2, 99);
        assert_eq!(a.clauses, b.clauses);
        assert_eq!(a.ref_runs, b.ref_runs);
    }

    #[test]
    fn fisher_sanity() {
        assert!((fisher_combined(&[0.5]) - 0.5).abs() < 1e-9);
        assert!(fisher_combined(&[0.05, 0.05, 0.05]) < 0.01);
    }
}

#[cfg(test)]
mod timing {
    #[test]
    #[ignore]
    fn timing() {
        for n in [20, 30, 40, 50, 64] {
            for k in [1, 8] {
                let t = std::time::Instant::now();
                let mut meds = vec![];
                for s in 0..5 { let p = super::generate_hard(n, k, s); meds.push((p.quantile(0.5), p.min_steps(), p.tried)); }
                println!("n={n} k={k} {:?}/puzzle  (walksat_med, min, tried) = {:?}", t.elapsed()/5, meds);
            }
        }
    }
}
