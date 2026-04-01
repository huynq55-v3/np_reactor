use ::rand::RngExt;
use ::rand::seq::SliceRandom; // Thêm :: để chỉ định rõ thư viện bên ngoài
use macroquad::prelude::*; // Thêm ::
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
}

impl GameState {
    fn randomerate(n: usize) -> Self {
        // Thêm :: vào trước rand
        let mut rng = ::rand::rng();

        let m_min = (n as f32 * 4.26) as usize;
        let m_max = (n as f32 * 4.26) as usize;
        let m = rng.random_range(m_min..=m_max);

        let solution: Vec<bool> = (0..n).map(|_| rng.random_bool(0.5)).collect();
        let mut clauses = Vec::new();

        // BẢO VỆ ĐÂY: Sổ ghi chép các ngoặc đã xuất hiện
        let mut seen_clauses = HashSet::new();

        // Thay vì dùng for, ta dùng while để đảm bảo sinh ĐỦ m cái ngoặc KHÁC NHAU
        while clauses.len() < m {
            let k = rng.random_range(3..=3.min(n)); // Set cứng K=3

            let mut available_vars: Vec<usize> = (0..n).collect();
            available_vars.shuffle(&mut rng);
            let chosen_vars = &available_vars[0..k];

            let mut literals = Vec::new();
            let mut num_sat = 0; // Đếm số lượng biến ĐÚNG so với nghiệm gốc

            for &v in chosen_vars {
                let sign = rng.random_bool(0.5);
                literals.push((v, sign));
                if solution[v] == sign {
                    num_sat += 1;
                }
            }

            // ====================================================
            // 🔥 THUẬT TOÁN SIẾT ĐỘ KHÓ (STRICT 1-SATISFIABLE) 🔥
            // ====================================================
            if num_sat == 0 {
                // 1. Đang sai hết -> Bắt buộc lật 1 cái để bài toán có đường sống
                let lucky = rng.random_range(0..k);
                literals[lucky].1 = !literals[lucky].1;
            } else if num_sat > 1 && rng.random_bool(0.85) {
                // 2. NẾU QUÁ DỄ (Có 2 hoặc 3 lối thoát)
                // -> 85% xác suất sẽ bít cửa, chỉ chừa lại đúng 1 lối thoát!
                let mut sat_indices = Vec::new();
                for i in 0..k {
                    if solution[literals[i].0] == literals[i].1 {
                        sat_indices.push(i);
                    }
                }

                // Trộn ngẫu nhiên các lối thoát
                sat_indices.shuffle(&mut rng);

                // Giữ lại đúng 1 lối thoát (index 0). Lật ngược tất cả các lối còn lại thành SAI
                for &idx in &sat_indices[1..] {
                    literals[idx].1 = !literals[idx].1;
                }
            }
            // ====================================================

            // Sắp xếp thứ tự biến tăng dần (UI gọn gàng)
            literals.sort_by_key(|&(v_idx, _)| v_idx);

            // KIỂM TRA TRÙNG LẶP: Nếu insert thành công (tức là chưa từng xuất hiện)
            if seen_clauses.insert(literals.clone()) {
                clauses.push(Clause { literals });
            }
        }

        Self {
            n,
            vars: vec![false; n], // Bắt đầu tất cả đều là Đỏ
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
        draw_text("Increase N", btn_x + 15.0, btn_y + 25.0, 25.0, GREEN);

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
                    // Thiết lập kích thước cho từng ô biến nhỏ
                    let literal_w = 40.0;
                    let literal_h = 30.0;
                    let spacing = 5.0; // Khoảng cách giữa các ô trong cùng 1 ngoặc

                    // Tính tổng chiều dài của cả cụm ngoặc này
                    let clause_w = (literal_w + spacing) * clause.literals.len() as f32;

                    // Xuống dòng nếu cụm này vượt quá mép phải màn hình
                    if cx + clause_w > sw - 20.0 {
                        cx = 20.0;
                        cy += literal_h + 20.0;
                    }

                    // Vẽ từng khung chữ nhật (biến) trong mệnh đề
                    let mut lx = cx;
                    for &(v_idx, required_sign) in &clause.literals {
                        // Khung ĐỎ nếu là NOT (cần false), Khung XANH nếu bình thường (cần true)
                        let bg_color = if !required_sign {
                            Color::new(0.6, 0.1, 0.1, 1.0) // Đỏ tối
                        } else {
                            Color::new(0.1, 0.5, 0.1, 1.0) // Xanh lá tối
                        };

                        // Vẽ ô vuông và viền trắng cho sắc nét
                        draw_rectangle(lx, cy, literal_w, literal_h, bg_color);
                        draw_rectangle_lines(lx, cy, literal_w, literal_h, 1.0, WHITE);

                        // In tên biến "V0", "V1" (bỏ hẳn chữ NOT) ra giữa ô
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

                    // Dịch con trỏ cx sang phải, chừa 1 khoảng trống lớn (20.0) để tách biệt với ngoặc tiếp theo
                    cx += clause_w + 20.0;
                }
            }
        }

        next_frame().await
    }
}
