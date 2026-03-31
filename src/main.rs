use ::rand::RngExt;
use ::rand::seq::SliceRandom; // Thêm :: để chỉ định rõ thư viện bên ngoài
use macroquad::prelude::*; // Thêm ::

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
}

impl GameState {
    fn randomerate(n: usize) -> Self {
        // Thêm :: vào trước rand
        let mut rng = ::rand::rng();

        let m_min = (n as f32 * 2.5) as usize;
        let m_max = (n as f32 * 4.5) as usize;
        let m = rng.random_range(m_min..=m_max);

        let solution: Vec<bool> = (0..n).map(|_| rng.random_bool(0.5)).collect();
        let mut clauses = Vec::new();

        for _ in 0..m {
            let k = rng.random_range(2..=5.min(n));

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

            if !is_sat {
                let lucky = rng.random_range(0..k);
                literals[lucky].1 = !literals[lucky].1;
            }

            clauses.push(Clause { literals });
        }

        Self {
            n,
            vars: vec![false; n],
            clauses,
            is_won: false,
            steps: 0,
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
#[macroquad::main("NP-Hard Game")]
async fn main() {
    let mut current_n = 5;
    let mut game = GameState::randomerate(current_n);

    loop {
        clear_background(Color::new(0.1, 0.1, 0.12, 1.0));

        let sw = screen_width();
        // Đã xóa biến sh (screen_height) vì không dùng đến, tránh warning

        // ------------------------------------------------
        // 1. UI ĐIỀU KHIỂN BÊN TRÊN
        // ------------------------------------------------
        draw_text(
            &format!("Level N = {} | Steps: {}", current_n, game.steps),
            20.0,
            40.0,
            30.0,
            WHITE,
        );

        let btn_w = 150.0;
        let btn_h = 40.0;
        let btn_x = sw - btn_w - 20.0;
        let btn_y = 15.0;

        draw_rectangle(btn_x, btn_y, btn_w, btn_h, DARKGRAY);
        draw_text("Increase N (+1)", btn_x + 15.0, btn_y + 25.0, 25.0, GREEN);

        draw_rectangle(btn_x - btn_w - 10.0, btn_y, btn_w, btn_h, DARKGRAY);
        draw_text(
            "New Problem",
            btn_x - btn_w + 5.0,
            btn_y + 25.0,
            25.0,
            YELLOW,
        );

        if is_mouse_button_pressed(MouseButton::Left) {
            let (mx, my) = mouse_position();
            if mx >= btn_x && mx <= btn_x + btn_w && my >= btn_y && my <= btn_y + btn_h {
                current_n += 1;
                game = GameState::randomerate(current_n);
            }
            if mx >= btn_x - btn_w - 10.0
                && mx <= btn_x - 10.0
                && my >= btn_y
                && my <= btn_y + btn_h
            {
                game = GameState::randomerate(current_n);
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
                Color::new(0.2, 0.8, 0.2, 1.0)
            } else {
                Color::new(0.8, 0.2, 0.2, 1.0)
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

                    let mut clause_str = String::new();
                    for (idx, &(v_idx, required_sign)) in clause.literals.iter().enumerate() {
                        if !required_sign {
                            clause_str.push_str("NOT ");
                        }
                        clause_str.push_str(&format!("V{}", v_idx));
                        if idx < clause.literals.len() - 1 {
                            clause_str.push_str(" OR ");
                        }
                    }

                    let text_dim = measure_text(&clause_str, None, 20, 1.0);
                    let box_w = text_dim.width + 20.0;
                    let box_h = 30.0;

                    if cx + box_w > sw - 20.0 {
                        cx = 20.0;
                        cy += box_h + 10.0;
                    }

                    draw_rectangle(cx, cy, box_w, box_h, Color::new(0.5, 0.1, 0.1, 1.0));
                    draw_rectangle_lines(cx, cy, box_w, box_h, 2.0, RED);
                    draw_text(&clause_str, cx + 10.0, cy + 20.0, 20.0, WHITE);

                    cx += box_w + 10.0;
                }
            }

            draw_text(
                &format!("{} clauses are still stubborn...", unsat_count),
                20.0,
                clauses_area_y - 5.0,
                20.0,
                ORANGE,
            );
        }

        next_frame().await
    }
}
