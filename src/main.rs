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
// TỪ ĐIỂN BIỂU TƯỢNG (SYMBOL DICTIONARY)
// ==========================================
const MAX_N: usize = 26; // Chừa sẵn 30 biểu tượng để ông tăng N
const VAR_SYMBOLS: [&str; MAX_N] = [
    "A", "B", "C", "D", "E", "F", "G", "H", "I", "J", "K", "L", "M", "N", "O", "P", "Q", "R", "S",
    "T", "U", "V", "W", "X", "Y", "Z",
];

// Hàm hỗ trợ vẽ Text Custom Font cho Unicode
fn draw_sym(text: &str, x: f32, y: f32, size: f32, color: Color, font: Option<&Font>) {
    draw_text_ex(
        text,
        x,
        y,
        TextParams {
            font,
            font_size: size as u16,
            color,
            ..Default::default()
        },
    );
}

fn measure_sym(text: &str, size: f32, font: Option<&Font>) -> TextDimensions {
    measure_text(text, font, size as u16, 1.0)
}

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
    last_flipped: Option<usize>,
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
        let m = (n as f32 * 4.26) as usize;

        let max_sols = ((1_usize << n) as f32 * (threshold_pct / 100.0)) as usize;
        let max_sols = max_sols.max(1);

        loop {
            let mut clauses = Vec::new();
            let mut seen_clauses = HashSet::new();

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

            let actual_sols = Self::count_solutions(n, &clauses);

            if actual_sols > 0 && actual_sols <= max_sols {
                // TẠO MẢNG RANDOM TRẠNG THÁI KHỞI ĐẦU
                let mut initial_vars = vec![false; n];
                for i in 0..n {
                    initial_vars[i] = rng.gen_bool(0.5);
                }

                // Cần kiểm tra xem xui xui random trúng ngay cái nghiệm luôn không
                // Nếu trúng nghiệm ngay lúc đầu thì win luôn (dù tỉ lệ cực thấp)
                let mut initial_win = true;
                for clause in &clauses {
                    let mut clause_sat = false;
                    for &(v_idx, req_sign) in &clause.literals {
                        if initial_vars[v_idx] == req_sign {
                            clause_sat = true;
                            break;
                        }
                    }
                    if !clause_sat {
                        initial_win = false;
                        break;
                    }
                }

                return Self {
                    n,
                    vars: initial_vars, // <--- ĐÃ ĐỔI THÀNH MẢNG RANDOM
                    clauses,
                    is_won: initial_win, // Cập nhật đúng trạng thái Win
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

fn window_conf() -> Conf {
    Conf {
        window_title: "NP-Reactor Symbolic".to_owned(),
        window_width: 1920,  // Tăng chiều rộng mặc định
        window_height: 1080, // Tăng chiều cao mặc định
        high_dpi: true,      // <--- CHÌA KHÓA: Bật High-DPI để hỗ trợ màn hình nét
        sample_count: 4,     // <--- Khử răng cưa (Anti-aliasing), để 4 hoặc 8 là mượt
        ..Default::default()
    }
}

// ==========================================
// VÒNG LẶP GAME CHÍNH
// ==========================================
#[macroquad::main(window_conf)]
async fn main() {
    // Tải font Unicode
    let font_bytes = std::fs::read("font.ttf");
    let custom_font = match font_bytes {
        Ok(bytes) => load_ttf_font_from_bytes(&bytes).ok(),
        Err(_) => {
            println!(
                "CẢNH BÁO: Không tìm thấy file 'font.ttf'. Các ký hiệu Unicode có thể bị lỗi ô vuông!"
            );
            None
        }
    };

    let mut current_n = 4;
    let mut current_threshold = 1.0;

    // Shuffle symbols once at the start of the game
    // let mut symbols = VAR_SYMBOLS;
    // symbols.shuffle(&mut ::rand::thread_rng());

    let mut game = GameState::randomerate(current_n, current_threshold);

    let mut scroll_y: f32 = 0.0;
    let mut max_scroll: f32 = 0.0;

    loop {
        let sw = screen_width();
        let sh = screen_height();

        // 1. TÍNH TOÁN TRƯỚC VỊ TRÍ LAYOUT
        let btn_w = if sw < 400.0 { (sw - 30.0) / 2.0 } else { 120.0 };
        let btn_h = 35.0;
        let gap = 10.0;

        let mut temp_bx = 10.0;
        let mut temp_by = 50.0;
        for _ in 0..4 {
            if temp_bx + btn_w > sw - 10.0 {
                temp_bx = 10.0;
                temp_by += btn_h + gap;
            }
            temp_bx += btn_w + gap;
        }
        let vars_area_y = temp_by + btn_h + 20.0;

        let var_size = 35.0;
        let var_gap = 10.0;
        let mut temp_vx = 10.0;
        let mut temp_vy = vars_area_y;
        for _ in 0..game.n {
            if temp_vx + var_size > sw - 10.0 {
                temp_vx = 10.0;
                temp_vy += var_size + var_gap;
            }
            temp_vx += var_size + var_gap;
        }
        let clauses_area_y = temp_vy + var_size + 30.0;

        // 2. XỬ LÝ SCROLL
        let (_, mouse_wheel_y) = mouse_wheel();
        scroll_y -= mouse_wheel_y * 20.0;

        if is_key_down(KeyCode::Up) {
            scroll_y -= 10.0;
        }
        if is_key_down(KeyCode::Down) {
            scroll_y += 10.0;
        }
        scroll_y = scroll_y.clamp(0.0, max_scroll);

        // 3. XÓA NỀN & VẼ CÁC MỆNH ĐỀ (CUỘN ĐƯỢC)
        let bg_color = Color::new(0.1, 0.1, 0.12, 1.0);
        clear_background(bg_color);

        let mut cx = 15.0;
        let mut cy = clauses_area_y - scroll_y;

        if game.is_won {
            let win_text = "RESONANCE ACHIEVED!";
            let text_dim = measure_text(win_text, None, 40, 1.0);
            draw_text(win_text, (sw - text_dim.width) / 2.0, cy + 80.0, 40.0, GOLD);
            cy += 150.0;
        } else {
            let mut unsat_count = 0;
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
                    let mut bg_c = if !required_sign {
                        Color::new(0.6, 0.1, 0.1, 1.0)
                    } else {
                        Color::new(0.1, 0.5, 0.1, 1.0)
                    };
                    let mut txt_c = WHITE;
                    if clause_sat {
                        bg_c.a = 0.4;
                        txt_c = Color::new(0.7, 0.7, 0.7, 1.0);
                    }

                    draw_rectangle(lx, cy, literal_w, literal_h, bg_c);

                    if game.last_flipped == Some(v_idx) {
                        draw_rectangle_lines(lx, cy, literal_w, literal_h, 2.0, SKYBLUE);
                    }

                    // VẼ KÝ HIỆU THAY VÌ CON SỐ
                    let text = VAR_SYMBOLS[v_idx % MAX_N];
                    let text_dim = measure_sym(text, 18.0, custom_font.as_ref());
                    draw_sym(
                        text,
                        lx + (literal_w - text_dim.width) / 2.0,
                        cy + (literal_h + text_dim.height) / 2.0 - 2.0,
                        18.0,
                        txt_c,
                        custom_font.as_ref(),
                    );

                    lx += literal_w + spacing;
                }
                cx += exact_clause_w + 20.0;
            }

            draw_text(
                &format!("{} clauses remain... (Scroll to view)", unsat_count),
                10.0,
                clauses_area_y - scroll_y - 5.0,
                16.0,
                ORANGE,
            );
        }

        let actual_bottom_y = cy + scroll_y + 50.0;
        max_scroll = (actual_bottom_y - sh).max(0.0);

        // 4. MASKING
        draw_rectangle(0.0, 0.0, sw, clauses_area_y - 25.0, bg_color);
        draw_line(
            10.0,
            clauses_area_y - 25.0,
            sw - 10.0,
            clauses_area_y - 25.0,
            2.0,
            GRAY,
        );

        // ===============================================
        // 5. VẼ UI BẢNG ĐIỀU KHIỂN & CÔNG TẮC
        // ===============================================
        let font_size = if sw < 600.0 { 16.0 } else { 22.0 };

        // TÍNH TOÁN TARGET ĐỂ SO SÁNH
        // 1. Giới hạn Toán học hiện tại (Schöning Algorithm)
        let target_soa = (1.3333_f64).powf(current_n as f64).round() as u32;
        // 2. Mục tiêu Đa thức để chứng minh P=NP (Dùng N^3 làm chuẩn)
        let target_pnp = (current_n as u32).pow(3);

        draw_text(
            &format!(
                "N={} | Steps: {} (SotA: ~{}, P=NP: ~{}) | Sols: {}",
                current_n, game.steps, target_soa, target_pnp, game.actual_sols
            ),
            10.0,
            30.0,
            font_size,
            YELLOW,
        );

        let mut bx = 10.0;
        let mut by = 50.0;

        let mut draw_btn = |text: &str, color: Color| -> bool {
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

            is_mouse_button_pressed(MouseButton::Left) && {
                let (mx, my) = mouse_position();
                mx >= rect.0 && mx <= rect.0 + rect.2 && my >= rect.1 && my <= rect.1 + rect.3
            }
        };

        if draw_btn("N + 1", Color::new(0.2, 0.6, 0.2, 1.0)) {
            current_n += 1;
            game = GameState::randomerate(current_n, current_threshold);
            scroll_y = 0.0;
        }
        if draw_btn("New Game", Color::new(0.8, 0.6, 0.1, 1.0)) {
            game = GameState::randomerate(current_n, current_threshold);
            scroll_y = 0.0;
        }
        if draw_btn("Thresh (+)", Color::new(0.2, 0.4, 0.6, 1.0)) {
            current_threshold += 0.5;
            game = GameState::randomerate(current_n, current_threshold);
            scroll_y = 0.0;
        }
        if draw_btn("Thresh (-)", Color::new(0.6, 0.2, 0.2, 1.0)) {
            current_threshold = (current_threshold - 0.5).max(0.1);
            game = GameState::randomerate(current_n, current_threshold);
            scroll_y = 0.0;
        }

        // VẼ DÀN CÔNG TẮC BẰNG KÝ HIỆU
        let mut vx = 10.0;
        let mut vy = vars_area_y;

        for i in 0..game.n {
            if vx + var_size > sw - 10.0 {
                vx = 10.0;
                vy += var_size + var_gap;
            }

            let color = if game.vars[i] {
                Color::new(0.1, 0.6, 0.1, 1.0)
            } else {
                Color::new(0.6, 0.1, 0.1, 1.0)
            };
            draw_rectangle(vx, vy, var_size, var_size, color);

            if game.last_flipped == Some(i) {
                draw_rectangle_lines(
                    vx - 2.0,
                    vy - 2.0,
                    var_size + 4.0,
                    var_size + 4.0,
                    3.0,
                    SKYBLUE,
                );
            }

            // VẼ KÝ HIỆU THAY VÌ CON SỐ
            let text = VAR_SYMBOLS[i % MAX_N];
            let text_dim = measure_sym(text, 22.0, custom_font.as_ref());
            draw_sym(
                text,
                vx + (var_size - text_dim.width) / 2.0,
                vy + (var_size + text_dim.height) / 2.0 - 4.0,
                22.0,
                WHITE,
                custom_font.as_ref(),
            );

            if !game.is_won && is_mouse_button_pressed(MouseButton::Left) {
                let (mx, my) = mouse_position();
                if mx >= vx && mx <= vx + var_size && my >= vy && my <= vy + var_size {
                    game.vars[i] = !game.vars[i];
                    game.steps += 1;
                    game.last_flipped = Some(i);
                    game.check_win_condition();
                }
            }
            vx += var_size + var_gap;
        }

        next_frame().await
    }
}
