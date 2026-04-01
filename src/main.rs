use ::rand::RngExt;
use ::rand::seq::SliceRandom;
use macroquad::prelude::*;
use std::collections::HashSet;

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
    threshold_pct: f32, // Ngưỡng % nghiệm cho phép
    actual_sols: usize, // Số lượng nghiệm thực tế của ván này!
}

impl GameState {
    // Thuật toán Vét cạn (Brute-force) siêu tốc bằng Bitwise
    fn count_solutions(n: usize, clauses: &Vec<Clause>) -> usize {
        let max_mask = 1_usize << n;
        let mut count = 0;

        // Duyệt toàn bộ 2^N trường hợp
        for mask in 0..max_mask {
            let mut all_sat = true;
            for clause in clauses {
                let mut clause_sat = false;
                for &(v_idx, required_sign) in &clause.literals {
                    // Trích xuất bit thứ v_idx ra xem là 0 (False) hay 1 (True)
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
        let mut rng = ::rand::rng();
        let m_min = (n as f32 * 4.26) as usize;
        let m_max = (n as f32 * 4.26) as usize;

        // Tính ra con số nghiệm tối đa cho phép
        let max_sols = ((1_usize << n) as f32 * (threshold_pct / 100.0)) as usize;
        let max_sols = max_sols.max(1); // Ít nhất phải cho phép 1 nghiệm

        loop {
            let m = rng.random_range(m_min..=m_max);
            let solution: Vec<bool> = (0..n).map(|_| rng.random_bool(0.5)).collect();
            let mut clauses = Vec::new();
            let mut seen_clauses = HashSet::new();

            // 1. SINH ĐỀ TỰ NHIÊN (Không gò ép siết độ khó nữa)
            while clauses.len() < m {
                let k = rng.random_range(3..=3.min(n));

                let mut available_vars: Vec<usize> = (0..n).collect();
                available_vars.shuffle(&mut rng);
                let chosen_vars = &available_vars[0..k];

                let mut literals = Vec::new();
                let mut is_sat = false;

                for &v in chosen_vars {
                    let sign = rng.random_bool(0.5);
                    literals.push((v, sign));
                    if solution[v] == sign {
                        is_sat = true;
                    }
                }

                // Cứu nếu sai bét
                if !is_sat {
                    let lucky = rng.random_range(0..k);
                    literals[lucky].1 = !literals[lucky].1;
                }

                literals.sort_by_key(|&(v_idx, _)| v_idx);

                if seen_clauses.insert(literals.clone()) {
                    clauses.push(Clause { literals });
                }
            }

            // 2. CHO BOT VÉT CẠN KIỂM TRA SỐ NGHIỆM
            let actual_sols = Self::count_solutions(n, &clauses);

            // 3. ĐẠT NGƯỠNG THÌ XUẤT XƯỞNG!
            if actual_sols <= max_sols {
                return Self {
                    n,
                    vars: vec![false; n],
                    clauses,
                    is_won: false,
                    steps: 0,
                    threshold_pct,
                    actual_sols,
                };
            }
            // Không đạt thì vòng lặp Loop tự động quay lại vứt rác sinh đề mới.
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
    let mut current_n = 8;
    let mut current_threshold = 1.0; // Khởi đầu: Lọc ra game có số nghiệm <= 1.0% không gian
    let mut game = GameState::randomerate(current_n, current_threshold);

    loop {
        clear_background(Color::new(0.1, 0.1, 0.12, 1.0));

        let sw = screen_width();

        // ------------------------------------------------
        // 1. UI ĐIỀU KHIỂN & CHỈ SỐ VÉT CẠN
        // ------------------------------------------------
        // In ra thông số gây cấn: Số nghiệm thực tế!
        let info_text = format!(
            "N={} | Steps: {} | Threshold: {:.1}% | Number of solutions: {}",
            current_n, game.steps, current_threshold, game.actual_sols
        );
        draw_text(&info_text, 20.0, 40.0, 25.0, YELLOW);

        let btn_w = 120.0;
        let btn_h = 35.0;
        let gap = 10.0;
        let mut bx = sw - btn_w - 20.0;
        let by = 15.0;

        // Nút: Tăng N
        draw_rectangle(bx, by, btn_w, btn_h, Color::new(0.2, 0.6, 0.2, 1.0));
        draw_text("N + 1", bx + 30.0, by + 25.0, 25.0, WHITE);
        let btn_n_plus = (bx, by, btn_w, btn_h);

        bx -= btn_w + gap;
        // Nút: Đổi Đề Mới
        draw_rectangle(bx, by, btn_w, btn_h, Color::new(0.8, 0.6, 0.1, 1.0));
        draw_text("New Problem", bx + 25.0, by + 25.0, 25.0, WHITE);
        let btn_new_game = (bx, by, btn_w, btn_h);

        bx -= btn_w + gap;
        // Nút: Tăng ngưỡng (Dễ hơn)
        draw_rectangle(bx, by, btn_w, btn_h, Color::new(0.2, 0.4, 0.6, 1.0));
        draw_text("Threshold (+)", bx + 5.0, by + 25.0, 20.0, WHITE);
        let btn_thresh_plus = (bx, by, btn_w, btn_h);

        bx -= btn_w + gap;
        // Nút: Giảm ngưỡng (Khó hơn)
        draw_rectangle(bx, by, btn_w, btn_h, Color::new(0.6, 0.2, 0.2, 1.0));
        draw_text("Threshold (-)", bx + 5.0, by + 25.0, 20.0, WHITE);
        let btn_thresh_minus = (bx, by, btn_w, btn_h);

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
                current_threshold += 0.5; // Tăng ngưỡng thêm 0.5% (Dễ đi)
                game = GameState::randomerate(current_n, current_threshold);
            } else if is_clicked(btn_thresh_minus) && current_threshold > 0.1 {
                current_threshold = (current_threshold - 0.5).max(0.1); // Giảm ngưỡng (Khó lên), tối thiểu 0.1%
                game = GameState::randomerate(current_n, current_threshold);
            }
        }

        // ------------------------------------------------
        // 2. VẼ DÀN CÔNG TẮC
        // ------------------------------------------------
        let vars_area_y = 80.0;
        let cols = (current_n as f32).sqrt().ceil() as usize;
        let rows = (current_n as f32 / cols as f32).ceil() as usize;

        let padding = 10.0;
        let var_w = ((sw - 40.0) / cols as f32) - padding;
        let var_h = 50.0;

        for i in 0..game.n {
            let c = i % cols;
            let r = i / cols;
            let vx = 20.0 + c as f32 * (var_w + padding);
            let vy = vars_area_y + r as f32 * (var_h + padding);

            let color = if game.vars[i] {
                Color::new(0.1, 0.6, 0.1, 1.0)
            } else {
                Color::new(0.6, 0.1, 0.1, 1.0)
            };
            draw_rectangle(vx, vy, var_w, var_h, color);

            let text = format!("V{}", i);
            let text_dim = measure_text(&text, None, 25, 1.0);
            draw_text(
                &text,
                vx + (var_w - text_dim.width) / 2.0,
                vy + (var_h + text_dim.height) / 2.0 - 5.0,
                25.0,
                WHITE,
            );

            if !game.is_won && is_mouse_button_pressed(MouseButton::Left) {
                let (mx, my) = mouse_position();
                if mx >= vx && mx <= vx + var_w && my >= vy && my <= vy + var_h {
                    game.vars[i] = !game.vars[i];
                    game.steps += 1;
                    game.check_win_condition();
                }
            }
        }

        // ------------------------------------------------
        // 3. VẼ CÁC MỆNH ĐỀ CHƯA ĐƯỢC THỎA MÃN
        // ------------------------------------------------
        let clauses_area_y = vars_area_y + (rows as f32 * (var_h + padding)) + 40.0;
        draw_line(
            20.0,
            clauses_area_y - 20.0,
            sw - 20.0,
            clauses_area_y - 20.0,
            2.0,
            GRAY,
        );

        if game.is_won {
            let win_text = "YOU WIN!";
            let text_dim = measure_text(win_text, None, 50, 1.0);
            draw_text(
                win_text,
                (sw - text_dim.width) / 2.0,
                clauses_area_y + 100.0,
                50.0,
                GOLD,
            );
        } else {
            let mut unsat_count = 0;
            let mut cx = 20.0;
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
                    let literal_w = 40.0;
                    let literal_h = 30.0;
                    let spacing = 5.0;
                    let clause_w = (literal_w + spacing) * clause.literals.len() as f32;

                    if cx + clause_w > sw - 20.0 {
                        cx = 20.0;
                        cy += literal_h + 20.0;
                    }

                    let mut lx = cx;
                    for &(v_idx, required_sign) in &clause.literals {
                        let bg_color = if !required_sign {
                            Color::new(0.6, 0.1, 0.1, 1.0)
                        } else {
                            Color::new(0.1, 0.5, 0.1, 1.0)
                        };
                        draw_rectangle(lx, cy, literal_w, literal_h, bg_color);
                        draw_rectangle_lines(lx, cy, literal_w, literal_h, 1.0, WHITE);

                        let text = format!("V{}", v_idx);
                        let text_dim = measure_text(&text, None, 20, 1.0);
                        draw_text(
                            &text,
                            lx + (literal_w - text_dim.width) / 2.0,
                            cy + (literal_h + text_dim.height) / 2.0 - 3.0,
                            20.0,
                            WHITE,
                        );
                        lx += literal_w + spacing;
                    }
                    cx += clause_w + 20.0;
                }
            }
            draw_text(
                &format!("{} clauses remain...", unsat_count),
                20.0,
                clauses_area_y - 5.0,
                20.0,
                ORANGE,
            );
        }

        next_frame().await
    }
}
