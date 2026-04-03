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

#[derive(Clone)]
struct Clause {
    literals: Vec<(usize, bool)>,
}

struct GameState {
    n: usize,
    vars: Vec<bool>,
    clauses: Vec<Clause>,
    initial_map: Vec<bool>,
    is_won: bool,
    steps: u32,
    visited_counts: HashMap<Vec<bool>, u32>,
    target_solution: Vec<bool>,
}

impl GameState {
    fn generate() -> Self {
        let n = 25;
        let mut rng = ::rand::thread_rng();
        let secret_solution: Vec<bool> = (0..n).map(|_| rng.gen_bool(0.5)).collect();
        let mut initial_map = vec![false; n];
        for i in 0..n {
            let r = i / 5;
            let c = i % 5;
            let mut neighbors = vec![i];
            if r > 0 {
                neighbors.push((r - 1) * 5 + c);
            }
            if r < 4 {
                neighbors.push((r + 1) * 5 + c);
            }
            if c > 0 {
                neighbors.push(r * 5 + (c - 1));
            }
            if c < 4 {
                neighbors.push(r * 5 + (c + 1));
            }
            let mut parity = false;
            for &idx in &neighbors {
                parity ^= secret_solution[idx];
            }
            initial_map[i] = parity;
        }

        let mut clauses = Vec::new();
        for i in 0..n {
            let r = i / 5;
            let c = i % 5;
            let mut neighbors = vec![i];
            if r > 0 {
                neighbors.push((r - 1) * 5 + c);
            }
            if r < 4 {
                neighbors.push((r + 1) * 5 + c);
            }
            if c > 0 {
                neighbors.push(r * 5 + (c - 1));
            }
            if c < 4 {
                neighbors.push(r * 5 + (c + 1));
            }
            neighbors.sort();
            let k = neighbors.len();
            let target_is_bright = initial_map[i];

            for mask in 0..(1 << k) {
                let mut p = false;
                for j in 0..k {
                    if (mask >> j) & 1 == 1 {
                        p = !p;
                    }
                }
                if p != target_is_bright {
                    let mut lits = Vec::new();
                    for j in 0..k {
                        let sign = (mask >> j) & 1 == 1;
                        lits.push((neighbors[j], !sign));
                    }
                    clauses.push(Clause { literals: lits });
                }
            }
        }

        let initial_vars = vec![false; n];
        let mut visited_counts = HashMap::new();
        visited_counts.insert(initial_vars.clone(), 1);

        Self {
            n,
            vars: initial_vars,
            clauses,
            initial_map,
            is_won: false,
            steps: 0,
            visited_counts,
            target_solution: secret_solution,
        }
    }

    fn check(&mut self) {
        let mut won = true;
        for c in &self.clauses {
            let mut sat = false;
            for &(v, sign) in &c.literals {
                if self.vars[v] == sign {
                    sat = true;
                    break;
                }
            }
            if !sat {
                won = false;
                break;
            }
        }
        self.is_won = won;
    }
}

#[macroquad::main("Lights Out SAT Triple View")]
async fn main() {
    let mut game = GameState::generate();
    let mut scroll_y = 0.0;
    let bg_color = Color::new(0.1, 0.1, 0.12, 1.0);

    // ===================================================================
    // KHAI BÁO THÔNG SỐ UI (Ô VUÔNG & CHỮ NHẬT)
    // ===================================================================
    let cell_size = 45.0; // Kích thước ô vuông (Variables & Bulbs)
    let cell_gap = 10.0; // Khoảng cách giữa các ô vuông
    let cell_font = 18.0; // Cỡ chữ trong ô vuông

    let lit_w = 38.0; // Chiều rộng ô chữ nhật (Literal trong Clause)
    let lit_h = 28.0; // Chiều cao ô chữ nhật
    let lit_gap = 2.0; // Khoảng cách giữa các literal trong 1 clause
    let clause_gap = 15.0; // Khoảng cách giữa các clause với nhau
    let lit_font = 18.0; // Cỡ chữ trong ô Clause
    // ===================================================================

    let var_labels: Vec<String> = (0..game.n).map(|i| (i + 1).to_string()).collect();

    loop {
        let sw = screen_width();
        let sh = screen_height();

        let (_, mouse_wheel_y) = mouse_wheel();
        scroll_y -= mouse_wheel_y * 25.0;
        scroll_y = scroll_y.max(0.0);

        clear_background(bg_color);

        let start_y = 100.0;
        let clause_area_top = start_y + 5.0 * (cell_size + cell_gap) + 60.0;

        // --- LỚP 1: VẼ SAT CLAUSES (CHỈ VẼ UNSAT) ---
        let mut cx = 20.0;
        let mut cy = clause_area_top - scroll_y + 20.0;

        for clause in &game.clauses {
            let mut is_sat = false;
            for &(v, sign) in &clause.literals {
                if game.vars[v] == sign {
                    is_sat = true;
                    break;
                }
            }
            if is_sat {
                continue;
            }

            let total_clause_w = (lit_w * clause.literals.len() as f32)
                + (lit_gap * (clause.literals.len() - 1) as f32);

            if cx + total_clause_w > sw - 20.0 {
                cx = 20.0;
                cy += lit_h + 10.0;
            }

            if cy + lit_h > clause_area_top && cy < sh {
                draw_rectangle_lines(
                    cx - 2.0,
                    cy - 2.0,
                    total_clause_w + 4.0,
                    lit_h + 4.0,
                    2.0,
                    RED,
                );

                let mut lx = cx;
                for &(v, sign) in &clause.literals {
                    let bg = if sign { WHITE } else { BLACK };
                    let tx = if sign { BLACK } else { WHITE };
                    draw_rectangle(lx, cy, lit_w, lit_h, bg);

                    // CĂN GIỮA TEXT TRONG Ô CHỮ NHẬT
                    let text = &var_labels[v];
                    let text_dim = measure_text(text, None, lit_font as u16, 1.0);
                    draw_text(
                        text,
                        lx + (lit_w - text_dim.width) / 2.0,
                        cy + (lit_h + text_dim.offset_y) / 2.0,
                        lit_font,
                        tx,
                    );
                    lx += lit_w + lit_gap;
                }
            }
            cx += total_clause_w + clause_gap;
        }

        // --- LỚP 2: TẤM NỀN CỐ ĐỊNH (MASK) ---
        draw_rectangle(0.0, 0.0, sw, clause_area_top, bg_color);
        draw_line(
            20.0,
            clause_area_top,
            sw - 20.0,
            clause_area_top,
            2.0,
            DARKGRAY,
        );

        // --- LỚP 3: GIAO DIỆN CHÍNH ---
        draw_text("LIGHTS OUT SAT: MASTER VIEW", 20.0, 30.0, 25.0, WHITE);
        let cur_state_visits = *game.visited_counts.get(&game.vars).unwrap_or(&1);
        draw_text(
            &format!(
                "Steps: {} | State Visits: {} | Status: {}",
                game.steps,
                cur_state_visits,
                if game.is_won { "SOLVED" } else { "UNSAT" }
            ),
            20.0,
            55.0,
            18.0,
            YELLOW,
        );

        // 1. MOVES (Variables)
        let var_x = 20.0;
        draw_text("1. MOVES (Variables)", var_x, 85.0, 18.0, SKYBLUE);
        for i in 0..game.n {
            let x = var_x + (i % 5) as f32 * (cell_size + cell_gap);
            let y = start_y + (i / 5) as f32 * (cell_size + cell_gap);

            // Tabu count dự kiến
            game.vars[i] = !game.vars[i];
            let proj_count = *game.visited_counts.get(&game.vars).unwrap_or(&0);
            game.vars[i] = !game.vars[i];

            let count_col = if proj_count == 0 {
                GRAY
            } else if proj_count < 3 {
                YELLOW
            } else {
                RED
            };
            let count_str = proj_count.to_string();
            let count_dim = measure_text(&count_str, None, 15, 1.0);
            draw_text(
                &count_str,
                x + (cell_size - count_dim.width) / 2.0,
                y - 5.0,
                15.0,
                count_col,
            );

            let (bg, txt) = if game.vars[i] {
                (WHITE, BLACK)
            } else {
                (BLACK, WHITE)
            };
            draw_rectangle(x, y, cell_size, cell_size, bg);
            draw_rectangle_lines(x, y, cell_size, cell_size, 2.0, GRAY);

            if is_key_down(KeyCode::Space) && game.target_solution[i] {
                draw_rectangle_lines(
                    x - 3.0,
                    y - 3.0,
                    cell_size + 6.0,
                    cell_size + 6.0,
                    3.0,
                    GREEN,
                );
            }

            // CĂN GIỮA SỐ TRONG Ô VUÔNG
            let text = &var_labels[i];
            let text_dim = measure_text(text, None, cell_font as u16, 1.0);
            draw_text(
                text,
                x + (cell_size - text_dim.width) / 2.0,
                y + (cell_size + text_dim.offset_y) / 2.0,
                cell_font,
                txt,
            );

            if is_mouse_button_pressed(MouseButton::Left) {
                let m = mouse_position();
                if m.0 >= x && m.0 <= x + cell_size && m.1 >= y && m.1 <= y + cell_size {
                    game.vars[i] = !game.vars[i];
                    game.steps += 1;
                    game.check();
                    *game.visited_counts.entry(game.vars.clone()).or_insert(0) += 1;
                }
            }
        }

        // 2. BULBS (Physical State)
        let bulb_x = var_x + 5.0 * (cell_size + cell_gap) + 40.0;
        draw_text("2. BULBS (Constraint View)", bulb_x, 85.0, 18.0, ORANGE);
        for i in 0..game.n {
            let r = i / 5;
            let c = i % 5;
            let x = bulb_x + c as f32 * (cell_size + cell_gap);
            let y = start_y + r as f32 * (cell_size + cell_gap);

            let mut parity = game.initial_map[i];
            parity ^= game.vars[i];
            if r > 0 {
                parity ^= game.vars[(r - 1) * 5 + c];
            }
            if r < 4 {
                parity ^= game.vars[(r + 1) * 5 + c];
            }
            if c > 0 {
                parity ^= game.vars[r * 5 + (c - 1)];
            }
            if c < 4 {
                parity ^= game.vars[r * 5 + (c + 1)];
            }

            let color = if parity {
                YELLOW
            } else {
                Color::new(0.2, 0.2, 0.3, 1.0)
            };
            draw_rectangle(x, y, cell_size, cell_size, color);
            draw_rectangle_lines(x, y, cell_size, cell_size, 2.0, GRAY);

            if parity {
                let txt = "ON";
                let t_dim = measure_text(txt, None, 14, 1.0);
                draw_text(
                    txt,
                    x + (cell_size - t_dim.width) / 2.0,
                    y + (cell_size + t_dim.offset_y) / 2.0,
                    14.0,
                    BLACK,
                );
            }
        }

        let unsat_count = game
            .clauses
            .iter()
            .filter(|c| !c.literals.iter().any(|&(v, s)| game.vars[v] == s))
            .count();
        draw_text(
            &format!("3. SAT CLAUSES: {} UNSAT (Scroll Down)", unsat_count),
            20.0,
            clause_area_top - 10.0,
            18.0,
            GREEN,
        );

        if game.is_won {
            draw_text("SYSTEM SATISFIED!", sw / 2.0 - 100.0, sh - 30.0, 30.0, GOLD);
        }

        next_frame().await
    }
}
