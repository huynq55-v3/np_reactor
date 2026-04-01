use ::rand::{Rng, seq::SliceRandom as _};
use macroquad::prelude::*;
use std::collections::HashSet;

// Bridge (Cầu nối) để rand 0.8 chạy được trên Macroquad Web
fn macroquad_getrandom(buf: &mut [u8]) -> Result<(), getrandom::Error> {
    for byte in buf.iter_mut() {
        *byte = macroquad::rand::gen_range(0, 255) as u8;
    }
    Ok(())
}
getrandom::register_custom_getrandom!(macroquad_getrandom);

// ==========================================
// CẤU TRÚC DỮ LIỆU
// ==========================================
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
    threshold_pct: f32,
    actual_sols: usize,
    last_flipped: Option<usize>, // <-- THÊM: Ghi nhớ con vừa lật
}

impl GameState {
    fn count_solutions(n: usize, clauses: &Vec<Clause>) -> usize {
        let max_mask = 1_usize << n;
        let mut count = 0;
        for mask in 0..max_mask {
            let mut all_sat = true;
            for clause in clauses {
                let mut clause_sat = false;
                for &(v_idx, required_sign) in &clause.literals {
                    let bit_val = (mask >> v_idx) & 1 == 1;
                    if bit_val == required_sign {
                        clause_sat = true;
                        break;
                    }
                }
                if !clause_sat {
                    all_sat = false;
                    break;
                }
            }
            if all_sat {
                count += 1;
            }
        }
        count
    }

    fn randomerate(n: usize, threshold_pct: f32) -> Self {
        let mut rng = ::rand::thread_rng();
        let m = (n as f32 * 4.26) as usize; // Chốt cứng tỉ lệ 4.26 (Ngưỡng khó nhất)

        let max_sols = ((1_usize << n) as f32 * (threshold_pct / 100.0)) as usize;
        let max_sols = max_sols.max(1);

        loop {
            let mut clauses = Vec::new();
            let mut seen_clauses = HashSet::new();

            // 1. TẠO MỆNH ĐỀ HOÀN TOÀN NGẪU NHIÊN (Không gieo nghiệm!)
            while clauses.len() < m {
                let k = rng.gen_range(3..=3.min(n));
                let mut available_vars: Vec<usize> = (0..n).collect();
                available_vars.shuffle(&mut rng);

                let mut literals = Vec::new();
                for i in 0..k {
                    let v = available_vars[i];
                    let sign = rng.gen_bool(0.5);
                    literals.push((v, sign));
                }
                literals.sort_by_key(|&(v_idx, _)| v_idx);

                if seen_clauses.insert(literals.clone()) {
                    clauses.push(Clause { literals });
                }
            }

            // 2. SAU ĐÓ MỚI ĐẾM NGHIỆM
            let actual_sols = Self::count_solutions(n, &clauses);

            // 3. CHỈ LẤY NHỮNG VÁN CÓ NGHIỆM (Và số nghiệm <= max_sols)
            if actual_sols > 0 && actual_sols <= max_sols {
                return Self {
                    n,
                    vars: vec![false; n],
                    clauses,
                    is_won: false,
                    steps: 0,
                    threshold_pct,
                    actual_sols,
                    last_flipped: None,
                };
            }
        }
    }

    fn check_win_condition(&mut self) {
        let mut any_unsat = false;
        for clause in &self.clauses {
            let mut clause_sat = false;
            for &(v_idx, required_sign) in &clause.literals {
                if self.vars[v_idx] == required_sign {
                    clause_sat = true;
                    break;
                }
            }
            if !clause_sat {
                any_unsat = true;
                break;
            }
        }
        self.is_won = !any_unsat;
    }
}

// ==========================================
// VÒNG LẶP GAME CHÍNH
// ==========================================
#[macroquad::main("NP-Hard")]
async fn main() {
    let mut current_n = 4;
    let mut current_threshold = 1.0;
    let mut game = GameState::randomerate(current_n, current_threshold);

    loop {
        clear_background(Color::new(0.1, 0.1, 0.12, 1.0));
        let sw = screen_width();

        let font_size = if sw < 600.0 { 16.0 } else { 22.0 };
        let info_text = format!(
            "N={} | Steps: {} | Thresh: {:.1}% | Sols: {}",
            current_n, game.steps, current_threshold, game.actual_sols
        );
        draw_text(&info_text, 10.0, 30.0, font_size, YELLOW);

        let mut bx = 10.0;
        let mut by = 50.0;
        let btn_w = if sw < 400.0 { (sw - 30.0) / 2.0 } else { 120.0 };
        let btn_h = 35.0;
        let gap = 10.0;

        let mut draw_btn = |text: &str, color: Color| -> (f32, f32, f32, f32) {
            if bx + btn_w > sw - 10.0 {
                bx = 10.0;
                by += btn_h + gap;
            }
            let rect = (bx, by, btn_w, btn_h);
            draw_rectangle(bx, by, btn_w, btn_h, color);

            let text_dim = measure_text(text, None, 18, 1.0);
            draw_text(
                text,
                bx + (btn_w - text_dim.width) / 2.0,
                by + (btn_h + text_dim.height) / 2.0 - 3.0,
                18.0,
                WHITE,
            );
            bx += btn_w + gap;
            rect
        };

        let btn_n_plus = draw_btn("N + 1", Color::new(0.2, 0.6, 0.2, 1.0));
        let btn_new_game = draw_btn("New Game", Color::new(0.8, 0.6, 0.1, 1.0));
        let btn_thresh_plus = draw_btn("Thresh (+)", Color::new(0.2, 0.4, 0.6, 1.0));
        let btn_thresh_minus = draw_btn("Thresh (-)", Color::new(0.6, 0.2, 0.2, 1.0));

        if is_mouse_button_pressed(MouseButton::Left) {
            let (mx, my) = mouse_position();
            let is_clicked = |rect: (f32, f32, f32, f32)| -> bool {
                mx >= rect.0 && mx <= rect.0 + rect.2 && my >= rect.1 && my <= rect.1 + rect.3
            };

            if is_clicked(btn_n_plus) {
                current_n += 1;
                game = GameState::randomerate(current_n, current_threshold);
            } else if is_clicked(btn_new_game) {
                game = GameState::randomerate(current_n, current_threshold);
            } else if is_clicked(btn_thresh_plus) {
                current_threshold += 0.5;
                game = GameState::randomerate(current_n, current_threshold);
            } else if is_clicked(btn_thresh_minus) && current_threshold > 0.1 {
                current_threshold = (current_threshold - 0.5).max(0.1);
                game = GameState::randomerate(current_n, current_threshold);
            }
        }

        // 3. VẼ DÀN CÔNG TẮC
        let vars_area_y = by + btn_h + 20.0;
        let cols = (current_n as f32).sqrt().ceil() as usize;
        let rows = (current_n as f32 / cols as f32).ceil() as usize;
        let padding = 10.0;
        let var_w = ((sw - 20.0) / cols as f32) - padding;
        let var_h = 45.0;

        for i in 0..game.n {
            let c = i % cols;
            let r = i / cols;
            let vx = 10.0 + c as f32 * (var_w + padding);
            let vy = vars_area_y + r as f32 * (var_h + padding);

            let color = if game.vars[i] {
                Color::new(0.1, 0.6, 0.1, 1.0)
            } else {
                Color::new(0.6, 0.1, 0.1, 1.0)
            };
            draw_rectangle(vx, vy, var_w, var_h, color);

            // // ---> THÊM: Vẽ viền sáng (Cyan) nếu đây là công tắc vừa lật <---
            // if game.last_flipped == Some(i) {
            //     draw_rectangle_lines(vx - 2.0, vy - 2.0, var_w + 4.0, var_h + 4.0, 4.0, SKYBLUE);
            // }

            let text = format!("{}", i);
            let text_dim = measure_text(&text, None, 20, 1.0);
            draw_text(
                &text,
                vx + (var_w - text_dim.width) / 2.0,
                vy + (var_h + text_dim.height) / 2.0 - 5.0,
                20.0,
                WHITE,
            );

            if !game.is_won && is_mouse_button_pressed(MouseButton::Left) {
                let (mx, my) = mouse_position();
                if mx >= vx && mx <= vx + var_w && my >= vy && my <= vy + var_h {
                    game.vars[i] = !game.vars[i];
                    game.steps += 1;
                    game.last_flipped = Some(i); // <-- THÊM: Cập nhật con vừa lật
                    game.check_win_condition();
                }
            }
        }

        // 4. VẼ TOÀN BỘ CÁC MỆNH ĐỀ THEO CỤM
        let clauses_area_y = vars_area_y + (rows as f32 * (var_h + padding)) + 30.0;
        draw_line(
            10.0,
            clauses_area_y - 15.0,
            sw - 10.0,
            clauses_area_y - 15.0,
            2.0,
            GRAY,
        );

        if game.is_won {
            let win_text = "YOU WIN!";
            let text_dim = measure_text(win_text, None, 40, 1.0);
            draw_text(
                win_text,
                (sw - text_dim.width) / 2.0,
                clauses_area_y + 80.0,
                40.0,
                GOLD,
            );
        } else {
            let mut unsat_count = 0;
            let mut cx = 15.0;
            let mut cy = clauses_area_y;

            for clause in &game.clauses {
                let mut clause_sat = false;
                for &(v_idx, required_sign) in &clause.literals {
                    if game.vars[v_idx] == required_sign {
                        clause_sat = true;
                        break;
                    }
                }

                if !clause_sat {
                    unsat_count += 1;
                }

                let literal_w = 35.0;
                let literal_h = 25.0;
                let spacing = 4.0;
                let exact_clause_w = (literal_w * clause.literals.len() as f32)
                    + (spacing * (clause.literals.len() - 1) as f32);
                let pad = 4.0;

                if cx + exact_clause_w + pad > sw - 10.0 {
                    cx = 15.0;
                    cy += literal_h + 25.0;
                }

                if !clause_sat {
                    draw_rectangle_lines(
                        cx - pad,
                        cy - pad,
                        exact_clause_w + pad * 2.0,
                        literal_h + pad * 2.0,
                        3.0,
                        YELLOW,
                    );
                } else {
                    draw_rectangle_lines(
                        cx - pad,
                        cy - pad,
                        exact_clause_w + pad * 2.0,
                        literal_h + pad * 2.0,
                        1.0,
                        Color::new(0.3, 0.3, 0.3, 1.0),
                    );
                }

                let mut lx = cx;
                for &(v_idx, required_sign) in &clause.literals {
                    let mut bg_color = if !required_sign {
                        Color::new(0.6, 0.1, 0.1, 1.0)
                    } else {
                        Color::new(0.1, 0.5, 0.1, 1.0)
                    };

                    let mut text_color = WHITE;
                    if clause_sat {
                        bg_color.a = 0.4;
                        text_color = Color::new(0.7, 0.7, 0.7, 1.0);
                    }

                    draw_rectangle(lx, cy, literal_w, literal_h, bg_color);

                    // // ---> THÊM: Vẽ viền sáng (Cyan) cho con vừa lật ngay bên trong Mệnh đề <---
                    // if game.last_flipped == Some(v_idx) {
                    //     draw_rectangle_lines(lx, cy, literal_w, literal_h, 2.0, SKYBLUE);
                    // } else {
                    //     draw_rectangle_lines(
                    //         lx,
                    //         cy,
                    //         literal_w,
                    //         literal_h,
                    //         1.0,
                    //         Color::new(0.2, 0.2, 0.2, 0.5),
                    //     );
                    // }

                    let text = format!("{}", v_idx);
                    let text_dim = measure_text(&text, None, 16, 1.0);
                    draw_text(
                        &text,
                        lx + (literal_w - text_dim.width) / 2.0,
                        cy + (literal_h + text_dim.height) / 2.0 - 2.0,
                        16.0,
                        text_color,
                    );
                    lx += literal_w + spacing;
                }
                cx += exact_clause_w + 20.0;
            }

            draw_text(
                &format!("{} clauses remain...", unsat_count),
                10.0,
                clauses_area_y - 5.0,
                16.0,
                ORANGE,
            );
        }

        next_frame().await
    }
}
