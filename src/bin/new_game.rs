use ::rand::{Rng, seq::SliceRandom as _};
use macroquad::prelude::*;
use std::collections::{HashMap, HashSet};

fn macroquad_getrandom(buf: &mut [u8]) -> Result<(), getrandom::Error> {
    for byte in buf.iter_mut() {
        *byte = macroquad::rand::gen_range(0, 255) as u8;
    }
    Ok(())
}
getrandom::register_custom_getrandom!(macroquad_getrandom);

fn get_hex_label(idx: usize) -> String {
    format!("{:02X}", idx)
}

#[derive(Clone, Copy, PartialEq)]
enum GameMode {
    Hard3SAT,
    XorRing,
}

impl GameMode {
    fn next(&self) -> Self {
        match self {
            GameMode::Hard3SAT => GameMode::XorRing,
            GameMode::XorRing => GameMode::Hard3SAT,
        }
    }
    fn to_string(&self) -> &str {
        match self {
            GameMode::Hard3SAT => "Mode: Hard 3-SAT",
            GameMode::XorRing => "Mode: XOR Ring",
        }
    }
}

#[derive(Clone)]
struct Clause {
    literals: Vec<(usize, bool)>,
}

struct GameState {
    n: usize,
    vars: Vec<bool>,
    clauses: Vec<Clause>,
    is_won: bool,
    steps: u32,
    actual_sols: usize,
    last_flipped: Option<usize>,
    visited_counts: HashMap<Vec<bool>, u32>,
    mode: GameMode,
    page: usize,
    per_page: usize,
}

impl GameState {
    fn count_solutions(n: usize, clauses: &Vec<Clause>) -> usize {
        let mut assignment = vec![None; n];
        Self::backtrack_count(0, n, &mut assignment, clauses)
    }

    fn backtrack_count(
        var_idx: usize,
        n: usize,
        assignment: &mut Vec<Option<bool>>,
        clauses: &[Clause],
    ) -> usize {
        for clause in clauses {
            let (mut is_sat, mut is_unresolved) = (false, false);
            for &(v, req) in &clause.literals {
                match assignment[v] {
                    Some(val) => {
                        if val == req {
                            is_sat = true;
                            break;
                        }
                    }
                    None => is_unresolved = true,
                }
            }
            if !is_sat && !is_unresolved {
                return 0;
            }
        }
        if var_idx == n {
            return 1;
        }
        let mut total = 0;
        assignment[var_idx] = Some(true);
        total += Self::backtrack_count(var_idx + 1, n, assignment, clauses);
        assignment[var_idx] = Some(false);
        total += Self::backtrack_count(var_idx + 1, n, assignment, clauses);
        assignment[var_idx] = None;
        total
    }

    fn generate(n: usize, mode: GameMode) -> Self {
        let mut rng = ::rand::thread_rng();
        let mut clauses = Vec::new();
        let mut actual_sols = 0;

        match mode {
            GameMode::Hard3SAT => {
                let m = (n as f32 * 4.26).ceil() as usize;
                loop {
                    clauses.clear();
                    let mut seen = HashSet::new();
                    while clauses.len() < m {
                        let mut v_indices: Vec<usize> = (0..n).collect();
                        v_indices.shuffle(&mut rng);
                        let mut lits = vec![
                            (v_indices[0], rng.gen_bool(0.5)),
                            (v_indices[1], rng.gen_bool(0.5)),
                            (v_indices[2], rng.gen_bool(0.5)),
                        ];
                        lits.sort_by_key(|k| k.0);
                        let key = format!("{:?}", lits);
                        if seen.insert(key) {
                            clauses.push(Clause { literals: lits });
                        }
                    }
                    actual_sols = Self::count_solutions(n, &clauses);
                    if actual_sols > 0 {
                        break;
                    }
                }
            }
            GameMode::XorRing => {
                let mut secret = vec![false; n];
                for i in 0..n {
                    secret[i] = rng.gen_bool(0.5);
                }
                for i in 0..n {
                    let (a, b, c) = (i, (i + 1) % n, (i + 2) % n);
                    let res = secret[a] ^ secret[b] ^ secret[c];
                    let configs = if res {
                        vec![
                            (true, true, true),
                            (true, false, false),
                            (false, true, false),
                            (false, false, true),
                        ]
                    } else {
                        vec![
                            (false, false, false),
                            (false, true, true),
                            (true, false, true),
                            (true, true, false),
                        ]
                    };
                    for (sa, sb, sc) in configs {
                        let mut lits = vec![(a, sa), (b, sb), (c, sc)];
                        lits.sort_by_key(|k| k.0);
                        clauses.push(Clause { literals: lits });
                    }
                }
                actual_sols = Self::count_solutions(n, &clauses);
            }
        }

        let mut initial_vars = vec![false; n];
        for i in 0..n {
            initial_vars[i] = rng.gen_bool(0.5);
        }
        let mut initial_counts = HashMap::new();
        initial_counts.insert(initial_vars.clone(), 1);

        GameState {
            n,
            vars: initial_vars,
            clauses,
            is_won: false,
            steps: 0,
            actual_sols,
            last_flipped: None,
            visited_counts: initial_counts,
            mode,
            page: 0,
            per_page: 144, // 12x12
        }
    }
}

#[macroquad::main("NP-Reactor: 12-Column Grid")]
async fn main() {
    let mut current_n = 32;
    let mut current_mode = GameMode::Hard3SAT;
    let mut game = GameState::generate(current_n, current_mode);
    let bg_color = Color::new(0.4, 0.45, 0.5, 1.0);

    loop {
        let sw = screen_width();
        let sh = screen_height();
        clear_background(bg_color);

        // --- 1. Header & Buttons ---
        let current_state_count = game.visited_counts.get(&game.vars).unwrap_or(&1);
        draw_text(
            &format!(
                "N={} | Steps: {} | Sols: {} | State: {} | M: {}",
                current_n,
                game.steps,
                game.actual_sols,
                current_state_count,
                game.clauses.len()
            ),
            10.0,
            25.0,
            20.0,
            YELLOW,
        );

        let mut bx = 10.0;
        let mut by = 40.0;
        if draw_btn(
            "N+1",
            &mut bx,
            &mut by,
            80.0,
            30.0,
            10.0,
            sw,
            Color::new(0.2, 0.4, 0.2, 1.0),
        ) {
            current_n += 1;
            game = GameState::generate(current_n, current_mode);
        }
        if draw_btn(
            "New",
            &mut bx,
            &mut by,
            80.0,
            30.0,
            10.0,
            sw,
            Color::new(0.6, 0.4, 0.1, 1.0),
        ) {
            game = GameState::generate(current_n, current_mode);
        }
        if draw_btn(
            game.mode.to_string(),
            &mut bx,
            &mut by,
            160.0,
            30.0,
            10.0,
            sw,
            Color::new(0.2, 0.3, 0.5, 1.0),
        ) {
            current_mode = game.mode.next();
            game = GameState::generate(current_n, current_mode);
        }
        let total_pages = (game.clauses.len() + game.per_page - 1) / game.per_page;
        if draw_btn(
            &format!("Pg < ({}/{})", game.page + 1, total_pages),
            &mut bx,
            &mut by,
            120.0,
            30.0,
            10.0,
            sw,
            DARKGRAY,
        ) {
            if game.page > 0 {
                game.page -= 1;
            }
        }
        if draw_btn("Pg >", &mut bx, &mut by, 80.0, 30.0, 10.0, sw, DARKGRAY) {
            if (game.page + 1) * game.per_page < game.clauses.len() {
                game.page += 1;
            }
        }

        // --- 2. Boolean Switches (32 columns max) ---
        let var_size = 32.0;
        let var_gap = 4.0;
        let start_vx = 10.0;
        let mut vy = by + 50.0;

        for i in 0..game.n {
            let col = i % 32;
            let row = i / 32;
            let vx = start_vx + col as f32 * (var_size + var_gap);
            let row_y = vy + row as f32 * (var_size + 25.0);

            // Radar (Centered)
            let mut proj = game.vars.clone();
            proj[i] = !proj[i];
            let p_count = game.visited_counts.get(&proj).unwrap_or(&0);
            let radar_txt = format!("{}", p_count);
            let t_dim = measure_text(&radar_txt, None, 14, 1.0);
            draw_text(
                &radar_txt,
                vx + (var_size - t_dim.width) / 2.0,
                row_y - 4.0,
                14.0,
                if *p_count == 0 { LIGHTGRAY } else { RED },
            );

            // Switch (Centered Hex)
            let (bg, txt_c) = if game.vars[i] {
                (WHITE, BLACK)
            } else {
                (BLACK, WHITE)
            };
            draw_rectangle(vx, row_y, var_size, var_size, bg);
            if game.last_flipped == Some(i) {
                draw_rectangle_lines(
                    vx - 1.0,
                    row_y - 1.0,
                    var_size + 2.0,
                    var_size + 2.0,
                    2.0,
                    SKYBLUE,
                );
            }

            let hex = get_hex_label(i);
            let h_dim = measure_text(&hex, None, 16, 1.0);
            draw_text(
                &hex,
                vx + (var_size - h_dim.width) / 2.0,
                row_y + (var_size + h_dim.height) / 2.0 - 2.0,
                16.0,
                txt_c,
            );

            if !game.is_won && is_mouse_button_pressed(MouseButton::Left) {
                let (mx, my) = mouse_position();
                if mx >= vx && mx <= vx + var_size && my >= row_y && my <= row_y + var_size {
                    game.vars[i] = !game.vars[i];
                    game.steps += 1;
                    game.last_flipped = Some(i);
                    let mut any_unsat = false;
                    for c in &game.clauses {
                        if !c.literals.iter().any(|&(v, r)| game.vars[v] == r) {
                            any_unsat = true;
                            break;
                        }
                    }
                    game.is_won = !any_unsat;
                    *game.visited_counts.entry(game.vars.clone()).or_insert(0) += 1;
                }
            }
            if i == game.n - 1 {
                vy = row_y + var_size + 20.0;
            }
        }

        // --- 3. Clauses Grid (12 Columns x 12 Rows) ---
        draw_line(10.0, vy, sw - 10.0, vy, 1.0, DARKGRAY);
        vy += 15.0;
        let start_idx = game.page * game.per_page;
        let end_idx = (start_idx + game.per_page).min(game.clauses.len());

        let cell_w = (sw - 30.0) / 12.0;
        let cell_h = 32.0;
        let lit_w = (cell_w - 10.0) / 3.0;

        for (idx, clause) in game.clauses[start_idx..end_idx].iter().enumerate() {
            let c_col = idx % 12;
            let c_row = idx / 12;
            let cx = 10.0 + c_col as f32 * cell_w;
            let cy = vy + c_row as f32 * (cell_h + 8.0);

            let is_sat = clause.literals.iter().any(|&(v, req)| game.vars[v] == req);

            // Vẽ viền Clause - Đã căn giữa chính xác trong cell
            let total_clause_w = clause.literals.len() as f32 * lit_w;
            let padding_x = (cell_w - total_clause_w) / 2.0;

            if !is_sat {
                draw_rectangle_lines(
                    cx + padding_x - 2.0,
                    cy - 2.0,
                    total_clause_w + 4.0,
                    cell_h + 4.0,
                    2.0,
                    YELLOW,
                );
            }

            for (l_idx, &(v_idx, req)) in clause.literals.iter().enumerate() {
                let lx = cx + padding_x + l_idx as f32 * lit_w;
                let (l_bg, l_txt) = if req { (WHITE, BLACK) } else { (BLACK, WHITE) };
                let alpha = if is_sat { 0.2 } else { 1.0 };

                draw_rectangle(
                    lx,
                    cy,
                    lit_w - 1.0,
                    cell_h,
                    Color::new(l_bg.r, l_bg.g, l_bg.b, alpha),
                );

                let h_label = get_hex_label(v_idx);
                let h_dim = measure_text(&h_label, None, 14, 1.0);
                draw_text(
                    &h_label,
                    lx + (lit_w - h_dim.width) / 2.0,
                    cy + (cell_h + h_dim.height) / 2.0,
                    20.0,
                    Color::new(l_txt.r, l_txt.g, l_txt.b, alpha),
                );
            }
        }

        if game.is_won {
            draw_text("SATISFIED", sw / 2.0 - 50.0, sh - 50.0, 30.0, GOLD);
        }
        next_frame().await
    }
}

fn draw_btn(
    text: &str,
    bx: &mut f32,
    by: &mut f32,
    w: f32,
    h: f32,
    gap: f32,
    sw: f32,
    color: Color,
) -> bool {
    if *bx + w > sw - 10.0 {
        *bx = 10.0;
        *by += h + gap;
    }
    let rect = (*bx, *by, w, h);
    draw_rectangle(rect.0, rect.1, rect.2, rect.3, color);
    let t_dim = measure_text(text, None, 18, 1.0);
    draw_text(
        text,
        rect.0 + (w - t_dim.width) / 2.0,
        rect.1 + (h + t_dim.height) / 2.0 - 2.0,
        18.0,
        WHITE,
    );
    let clicked = is_mouse_button_pressed(MouseButton::Left) && {
        let (mx, my) = mouse_position();
        mx >= rect.0 && mx <= rect.0 + rect.2 && my >= rect.1 && my <= rect.1 + rect.3
    };
    *bx += w + gap;
    clicked
}
