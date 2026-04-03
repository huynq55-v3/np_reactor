use ::rand::{Rng, seq::SliceRandom as _};
use macroquad::prelude::*;
use std::collections::{HashMap, HashSet};

// Bridge (Cầu nối) để rand 0.8 chạy được trên Macroquad Web
fn macroquad_getrandom(buf: &mut [u8]) -> Result<(), getrandom::Error> {
    for byte in buf.iter_mut() {
        *byte = macroquad::rand::gen_range(0, 255) as u8;
    }
    Ok(())
}
getrandom::register_custom_getrandom!(macroquad_getrandom);

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
// GAME MODES (CHẾ ĐỘ ĐỊA HÌNH)
// ==========================================
#[derive(Clone, Copy, PartialEq)]
enum GameMode {
    Random,
    XorRandom,
    XorRing,
    AESAvalanche,
}

impl GameMode {
    fn next(&self) -> Self {
        match self {
            GameMode::Random => GameMode::XorRandom,
            GameMode::XorRandom => GameMode::XorRing,
            GameMode::XorRing => GameMode::AESAvalanche,
            GameMode::AESAvalanche => GameMode::Random,
        }
    }

    fn to_string(&self) -> &str {
        match self {
            GameMode::Random => "Mode: Random",
            GameMode::XorRandom => "Mode: XOR Rnd",
            GameMode::XorRing => "Mode: XOR Ring",
            GameMode::AESAvalanche => "Mode: AES Avalanche",
        }
    }
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
    var_names: Vec<String>,
    clauses: Vec<Clause>,
    is_won: bool,
    steps: u32,
    threshold_pct: f32,
    actual_sols: usize,
    last_flipped: Option<usize>,
    visited_counts: HashMap<Vec<bool>, u32>,
    ever_unsat: Vec<bool>,
    mode: GameMode,
}

impl GameState {
    fn has_solution(n: usize, clauses: &Vec<Clause>) -> bool {
        if n > 22 {
            return true;
        }
        let max_mask = 1_usize << n;
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
                return true;
            }
        }
        false
    }

    fn generate(n: usize, threshold_pct: f32, mode: GameMode) -> Self {
        let mut rng = ::rand::thread_rng();

        let mut var_names = Vec::new();
        for i in 0..n {
            var_names.push((i + 1).to_string());
        }

        match mode {
            // ---------------------------------------------------------
            // 1. RANDOM 3-SAT PURE
            // ---------------------------------------------------------
            GameMode::Random => {
                let m = (n as f32 * 4.26) as usize;

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

                    if Self::has_solution(n, &clauses) {
                        let mut initial_vars = vec![false; n];
                        for i in 0..n {
                            initial_vars[i] = rng.gen_bool(0.5);
                        }

                        let mut initial_win = true;
                        let mut ever_unsat = vec![false; clauses.len()];
                        for (i, clause) in clauses.iter().enumerate() {
                            let mut clause_sat = false;
                            for &(v_idx, req_sign) in &clause.literals {
                                if initial_vars[v_idx] == req_sign {
                                    clause_sat = true;
                                    break;
                                }
                            }
                            if !clause_sat {
                                initial_win = false;
                                ever_unsat[i] = true;
                            }
                        }

                        let mut initial_counts = HashMap::new();
                        initial_counts.insert(initial_vars.clone(), 1);

                        return Self {
                            n,
                            vars: initial_vars,
                            var_names,
                            clauses,
                            is_won: initial_win,
                            steps: 0,
                            threshold_pct,
                            actual_sols: 1,
                            last_flipped: None,
                            visited_counts: initial_counts,
                            ever_unsat,
                            mode,
                        };
                    }
                }
            }

            // ---------------------------------------------------------
            // 2 & 3. XOR RANDOM & XOR RING OF FIRE
            // ---------------------------------------------------------
            GameMode::XorRandom | GameMode::XorRing => {
                let mut secret_solution = vec![false; n];
                for i in 0..n {
                    secret_solution[i] = rng.gen_bool(0.5);
                }

                let mut clauses = Vec::new();

                if mode == GameMode::XorRandom {
                    let mut seen_triplets = HashSet::new();
                    let max_possible_triplets = if n >= 3 {
                        (n * (n - 1) * (n - 2)) / 6
                    } else {
                        0
                    };
                    let num_equations = (n + (n / 2)).min(max_possible_triplets);

                    while clauses.len() / 4 < num_equations {
                        let mut available_vars: Vec<usize> = (0..n).collect();
                        available_vars.shuffle(&mut rng);
                        let mut triplet =
                            vec![available_vars[0], available_vars[1], available_vars[2]];
                        triplet.sort();

                        let a = triplet[0];
                        let b = triplet[1];
                        let c = triplet[2];

                        if seen_triplets.insert(triplet.clone()) {
                            let xor_result =
                                secret_solution[a] ^ secret_solution[b] ^ secret_solution[c];
                            if xor_result == true {
                                clauses.push(Clause {
                                    literals: vec![(a, true), (b, true), (c, true)],
                                });
                                clauses.push(Clause {
                                    literals: vec![(a, true), (b, false), (c, false)],
                                });
                                clauses.push(Clause {
                                    literals: vec![(a, false), (b, true), (c, false)],
                                });
                                clauses.push(Clause {
                                    literals: vec![(a, false), (b, false), (c, true)],
                                });
                            } else {
                                clauses.push(Clause {
                                    literals: vec![(a, false), (b, false), (c, false)],
                                });
                                clauses.push(Clause {
                                    literals: vec![(a, false), (b, true), (c, true)],
                                });
                                clauses.push(Clause {
                                    literals: vec![(a, true), (b, false), (c, true)],
                                });
                                clauses.push(Clause {
                                    literals: vec![(a, true), (b, true), (c, false)],
                                });
                            }
                        }
                    }
                } else {
                    for i in 0..n {
                        let mut triplet = vec![i, (i + 1) % n, (i + 2) % n];
                        triplet.sort();
                        let a = triplet[0];
                        let b = triplet[1];
                        let c = triplet[2];

                        let xor_result =
                            secret_solution[a] ^ secret_solution[b] ^ secret_solution[c];

                        if xor_result == true {
                            clauses.push(Clause {
                                literals: vec![(a, true), (b, true), (c, true)],
                            });
                            clauses.push(Clause {
                                literals: vec![(a, true), (b, false), (c, false)],
                            });
                            clauses.push(Clause {
                                literals: vec![(a, false), (b, true), (c, false)],
                            });
                            clauses.push(Clause {
                                literals: vec![(a, false), (b, false), (c, true)],
                            });
                        } else {
                            clauses.push(Clause {
                                literals: vec![(a, false), (b, false), (c, false)],
                            });
                            clauses.push(Clause {
                                literals: vec![(a, false), (b, true), (c, true)],
                            });
                            clauses.push(Clause {
                                literals: vec![(a, true), (b, false), (c, true)],
                            });
                            clauses.push(Clause {
                                literals: vec![(a, true), (b, true), (c, false)],
                            });
                        }
                    }
                    for i in 0..(n / 2) {
                        let mut triplet = vec![i, (i + n / 2) % n, (i + n / 4) % n];
                        triplet.sort();
                        let a = triplet[0];
                        let b = triplet[1];
                        let c = triplet[2];

                        let xor_result =
                            secret_solution[a] ^ secret_solution[b] ^ secret_solution[c];
                        if xor_result == true {
                            clauses.push(Clause {
                                literals: vec![(a, true), (b, true), (c, true)],
                            });
                            clauses.push(Clause {
                                literals: vec![(a, true), (b, false), (c, false)],
                            });
                            clauses.push(Clause {
                                literals: vec![(a, false), (b, true), (c, false)],
                            });
                            clauses.push(Clause {
                                literals: vec![(a, false), (b, false), (c, true)],
                            });
                        } else {
                            clauses.push(Clause {
                                literals: vec![(a, false), (b, false), (c, false)],
                            });
                            clauses.push(Clause {
                                literals: vec![(a, false), (b, true), (c, true)],
                            });
                            clauses.push(Clause {
                                literals: vec![(a, true), (b, false), (c, true)],
                            });
                            clauses.push(Clause {
                                literals: vec![(a, true), (b, true), (c, false)],
                            });
                        }
                    }
                }

                let mut initial_vars = vec![false; n];
                for i in 0..n {
                    initial_vars[i] = rng.gen_bool(0.5);
                }

                let mut initial_win = true;
                let mut ever_unsat = vec![false; clauses.len()];
                for (i, clause) in clauses.iter().enumerate() {
                    let mut clause_sat = false;
                    for &(v_idx, req_sign) in &clause.literals {
                        if initial_vars[v_idx] == req_sign {
                            clause_sat = true;
                            break;
                        }
                    }
                    if !clause_sat {
                        initial_win = false;
                        ever_unsat[i] = true;
                    }
                }

                let mut initial_counts = HashMap::new();
                initial_counts.insert(initial_vars.clone(), 1);

                return Self {
                    n,
                    vars: initial_vars,
                    var_names,
                    clauses,
                    is_won: initial_win,
                    steps: 0,
                    threshold_pct,
                    actual_sols: 1,
                    last_flipped: None,
                    visited_counts: initial_counts,
                    ever_unsat,
                    mode,
                };
            }
            // ---------------------------------------------------------
            // 4. AES AVALANCHE (Mô phỏng mạng SPN hỗn loạn)
            // ---------------------------------------------------------
            GameMode::AESAvalanche => {
                let mut secret_solution = vec![false; n];
                for i in 0..n {
                    secret_solution[i] = rng.gen_bool(0.5);
                }
                let mut clauses = Vec::new();

                let num_equations = n * 2;
                for _ in 0..num_equations {
                    let k = rng.gen_range(3..=5.min(n));
                    let mut available_vars: Vec<usize> = (0..n).collect();
                    available_vars.shuffle(&mut rng);

                    let mut selected_vars = available_vars[0..k].to_vec();
                    selected_vars.sort();

                    let mut xor_sum = false;
                    for &v in &selected_vars {
                        xor_sum ^= secret_solution[v];
                    }

                    let max_mask = 1_usize << k;
                    for mask in 0..max_mask {
                        let mut parity = false;
                        for i in 0..k {
                            if (mask >> i) & 1 == 1 {
                                parity = !parity;
                            }
                        }

                        if parity != xor_sum {
                            let mut literals = Vec::new();
                            for i in 0..k {
                                let sign = (mask >> i) & 1 == 1;
                                literals.push((selected_vars[i], !sign));
                            }
                            clauses.push(Clause { literals });
                        }
                    }
                }

                let mut initial_vars = vec![false; n];
                for i in 0..n {
                    initial_vars[i] = rng.gen_bool(0.5);
                }

                let mut initial_win = true;
                let mut ever_unsat = vec![false; clauses.len()];
                for (i, clause) in clauses.iter().enumerate() {
                    let mut clause_sat = false;
                    for &(v_idx, req_sign) in &clause.literals {
                        if initial_vars[v_idx] == req_sign {
                            clause_sat = true;
                            break;
                        }
                    }
                    if !clause_sat {
                        initial_win = false;
                        ever_unsat[i] = true;
                    }
                }

                let mut initial_counts = HashMap::new();
                initial_counts.insert(initial_vars.clone(), 1);

                return Self {
                    n,
                    vars: initial_vars,
                    var_names,
                    clauses,
                    is_won: initial_win,
                    steps: 0,
                    threshold_pct,
                    actual_sols: 1,
                    last_flipped: None,
                    visited_counts: initial_counts,
                    ever_unsat,
                    mode,
                };
            }
        }
    }

    fn check_win_condition(&mut self) {
        let mut any_unsat = false;
        for (i, clause) in self.clauses.iter().enumerate() {
            let mut clause_sat = false;
            for &(v_idx, required_sign) in &clause.literals {
                if self.vars[v_idx] == required_sign {
                    clause_sat = true;
                    break;
                }
            }
            if !clause_sat {
                any_unsat = true;
                self.ever_unsat[i] = true;
            }
        }
        self.is_won = !any_unsat;
    }
}

fn window_conf() -> Conf {
    Conf {
        window_title: "NP-Reactor: Multi-Mode".to_owned(),
        window_width: 1920,
        window_height: 1080,
        high_dpi: true,
        sample_count: 4,
        ..Default::default()
    }
}

// ==========================================
// VÒNG LẶP GAME CHÍNH
// ==========================================
#[macroquad::main(window_conf)]
async fn main() {
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

    let mut current_n = 256;
    let mut current_threshold = 1.0;
    let mut current_mode = GameMode::AESAvalanche;

    let mut game = GameState::generate(current_n, current_threshold, current_mode);

    let mut scroll_y: f32 = 0.0;
    let mut max_scroll: f32 = 0.0;

    let bg_color = Color::new(0.4, 0.45, 0.5, 1.0);

    loop {
        let sw = screen_width();
        let sh = screen_height();

        // 1. SCROLL TỔNG THỂ (GLOBAL SCROLL)
        let (_, mouse_wheel_y) = mouse_wheel();
        scroll_y -= mouse_wheel_y * 40.0;

        if is_key_down(KeyCode::Up) {
            scroll_y -= 15.0;
        }
        if is_key_down(KeyCode::Down) {
            scroll_y += 15.0;
        }
        scroll_y = scroll_y.clamp(0.0, max_scroll);

        clear_background(bg_color);

        let global_offset_y = -scroll_y;

        // 2. TÍNH TOÁN LAYOUT CÁC NÚT ĐIỀU KHIỂN (HEADER)
        let btn_w = if sw < 400.0 { (sw - 30.0) / 2.0 } else { 120.0 };
        let btn_h = 35.0;
        let gap = 10.0;

        let mut temp_bx = 10.0;
        let mut temp_by = 50.0;

        for _ in 0..5 {
            if temp_bx + btn_w > sw - 10.0 {
                temp_bx = 10.0;
                temp_by += btn_h + gap;
            }
            temp_bx += btn_w + gap;
        }

        let vars_area_y = temp_by + btn_h + 35.0;

        // SỬA Ở ĐÂY 1: Thu nhỏ kích thước ô vuông và khoảng cách
        let var_size = 30.0; // Từ 45.0 xuống 30.0
        let var_gap = 5.0; // Từ 10.0 xuống 5.0
        let mut temp_vx = 10.0;
        let mut temp_vy = vars_area_y;
        for _ in 0..game.n {
            if temp_vx + var_size > sw - 10.0 {
                temp_vx = 10.0;
                temp_vy += var_size + var_gap + 15.0; // Khoảng cách hàng dọc cũng giảm
            }
            temp_vx += var_size + var_gap;
        }

        let clauses_area_y = temp_vy + var_size + 40.0;

        // 3. VẼ DÀN CÔNG TẮC
        let mut vx = 10.0;
        let mut vy = vars_area_y;

        for i in 0..game.n {
            if vx + var_size > sw - 10.0 {
                vx = 10.0;
                vy += var_size + var_gap + 15.0;
            }

            let (bg_c, txt_c) = if game.vars[i] {
                (WHITE, BLACK)
            } else {
                (BLACK, WHITE)
            };

            let draw_y = vy + global_offset_y;

            if draw_y > -50.0 && draw_y < sh + 50.0 {
                draw_rectangle(vx, draw_y, var_size, var_size, bg_c);

                if game.last_flipped == Some(i) {
                    draw_rectangle_lines(
                        vx - 2.0,
                        draw_y - 2.0,
                        var_size + 4.0,
                        var_size + 4.0,
                        2.0,
                        SKYBLUE,
                    );
                }

                let text = &game.var_names[i];
                // Thu nhỏ font tên biến xuống 14.0 để vừa với ô 30.0
                let text_dim = measure_sym(text, 14.0, custom_font.as_ref());
                draw_sym(
                    text,
                    vx + (var_size - text_dim.width) / 2.0,
                    draw_y + (var_size + text_dim.height) / 2.0 - 2.0,
                    14.0,
                    txt_c,
                    custom_font.as_ref(),
                );

                // Xử lý Click
                if !game.is_won && is_mouse_button_pressed(MouseButton::Left) {
                    let (mx, my) = mouse_position();
                    if mx >= vx && mx <= vx + var_size && my >= draw_y && my <= draw_y + var_size {
                        game.vars[i] = !game.vars[i];
                        game.steps += 1;
                        game.last_flipped = Some(i);
                        game.check_win_condition();
                        *game.visited_counts.entry(game.vars.clone()).or_insert(0) += 1;
                    }
                }
            }
            vx += var_size + var_gap;
        }

        // 4. VẼ CÁC MỆNH ĐỀ (SỬA Ở ĐÂY 2: CHỈ VẼ CÁC MỆNH ĐỀ INVALID)
        let mut cx = 15.0;
        let mut cy = clauses_area_y;

        if game.is_won {
            let win_text = "RESONANCE ACHIEVED!";
            let text_dim = measure_text(win_text, None, 40, 1.0);
            draw_text(
                win_text,
                (sw - text_dim.width) / 2.0,
                cy + global_offset_y + 80.0,
                40.0,
                GOLD,
            );
            cy += 150.0;
        } else {
            let mut unsat_count = 0;

            // Duyệt toàn bộ mệnh đề để đếm số lượng lỗi trước
            for clause in game.clauses.iter() {
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
            }

            // Vẽ thông báo tổng số lỗi
            draw_text(
                &format!(
                    "{} invalid clauses remaining... (Valid ones are hidden)",
                    unsat_count
                ),
                10.0,
                (clauses_area_y - 20.0) + global_offset_y,
                18.0,
                YELLOW,
            );

            // Vẽ các mệnh đề sai
            for (c_idx, clause) in game.clauses.iter().enumerate() {
                let mut clause_sat = false;
                for &(v_idx, required_sign) in &clause.literals {
                    if game.vars[v_idx] == required_sign {
                        clause_sat = true;
                        break;
                    }
                }

                // NẾU MỆNH ĐỀ ĐÚNG -> BỎ QUA KHÔNG VẼ NỮA
                if clause_sat {
                    continue;
                }

                let literal_w = 32.0; // Rộng vừa đủ cho 3 số
                let literal_h = 22.0;
                let spacing = 3.0;
                let exact_clause_w = (literal_w * clause.literals.len() as f32)
                    + (spacing * (clause.literals.len() - 1) as f32);
                let pad = 3.0;

                if cx + exact_clause_w + pad > sw - 10.0 {
                    cx = 15.0;
                    cy += literal_h + 15.0;
                }

                let draw_cy = cy + global_offset_y;

                if draw_cy > -50.0 && draw_cy < sh + 50.0 {
                    if !game.ever_unsat[c_idx] {
                        // TÔ ĐỎ NHỮNG CLAUSE CHƯA TỪNG BỊ SAI (Vừa mới sai lần đầu do user lật)
                        draw_rectangle_lines(
                            cx - pad,
                            draw_cy - pad,
                            exact_clause_w + pad * 2.0,
                            literal_h + pad * 2.0,
                            2.0,
                            RED,
                        );
                    } else {
                        // Màu vàng chuẩn cho mệnh đề sai
                        // draw_rectangle_lines(
                        //     cx - pad,
                        //     draw_cy - pad,
                        //     exact_clause_w + pad * 2.0,
                        //     literal_h + pad * 2.0,
                        //     2.0,
                        //     YELLOW,
                        // );
                    }

                    let mut lx = cx;
                    for &(v_idx, required_sign) in &clause.literals {
                        let (bg_c, txt_c) = if required_sign {
                            (WHITE, BLACK)
                        } else {
                            (BLACK, WHITE)
                        };
                        draw_rectangle(lx, draw_cy, literal_w, literal_h, bg_c);

                        if game.last_flipped == Some(v_idx) {
                            draw_rectangle_lines(lx, draw_cy, literal_w, literal_h, 2.0, SKYBLUE);
                        }

                        let text = &game.var_names[v_idx];
                        let text_dim = measure_sym(text, 14.0, custom_font.as_ref());
                        draw_sym(
                            text,
                            lx + (literal_w - text_dim.width) / 2.0,
                            draw_cy + (literal_h + text_dim.height) / 2.0 - 2.0,
                            14.0,
                            txt_c,
                            custom_font.as_ref(),
                        );
                        lx += literal_w + spacing;
                    }
                }
                cx += exact_clause_w + 20.0;
            }
        }

        let actual_bottom_y = cy + 100.0;
        max_scroll = (actual_bottom_y - sh).max(0.0);

        // 5. VẼ UI BẢNG ĐIỀU KHIỂN (CỐ ĐỊNH TRÊN CÙNG)
        draw_rectangle(
            0.0,
            0.0,
            sw,
            temp_by + btn_h + 15.0,
            Color::new(0.3, 0.35, 0.4, 0.95),
        );
        draw_line(
            0.0,
            temp_by + btn_h + 15.0,
            sw,
            temp_by + btn_h + 15.0,
            2.0,
            DARKGRAY,
        );

        let font_size = if sw < 600.0 { 16.0 } else { 22.0 };
        let target_pnp = (current_n as u32).pow(3);
        let current_state_count = game.visited_counts.get(&game.vars).unwrap_or(&1);
        let warning_color = if *current_state_count >= 3 {
            RED
        } else {
            YELLOW
        };

        draw_text(
            &format!(
                "N={} | Steps: {} (P=NP: ~{}) | Sols: {} | Cur State: {} | Threshold: {:.1}%",
                current_n,
                game.steps,
                target_pnp,
                if current_mode == GameMode::Random {
                    ">= 1"
                } else {
                    "1"
                },
                current_state_count,
                current_threshold
            ),
            10.0,
            30.0,
            font_size,
            warning_color,
        );

        let mut bx = 10.0;
        let mut by = 50.0;

        let mut draw_btn = |text: &str, color: Color| -> bool {
            let width = if text.starts_with("Mode") {
                btn_w + 20.0
            } else {
                btn_w
            };
            if bx + width > sw - 10.0 {
                bx = 10.0;
                by += btn_h + gap;
            }
            let rect = (bx, by, width, btn_h);
            draw_rectangle(bx, by, width, btn_h, color);
            let text_dim = measure_text(text, None, 18, 1.0);
            draw_text(
                text,
                bx + (width - text_dim.width) / 2.0,
                by + (btn_h + text_dim.height) / 2.0 - 3.0,
                18.0,
                WHITE,
            );
            bx += width + gap;

            is_mouse_button_pressed(MouseButton::Left) && {
                let (mx, my) = mouse_position();
                mx >= rect.0 && mx <= rect.0 + rect.2 && my >= rect.1 && my <= rect.1 + rect.3
            }
        };

        if draw_btn("N + 1", Color::new(0.2, 0.4, 0.2, 1.0)) {
            current_n += 1;
            game = GameState::generate(current_n, current_threshold, current_mode);
            scroll_y = 0.0;
        }
        if draw_btn("N + 10", Color::new(0.2, 0.5, 0.3, 1.0)) {
            current_n += 10;
            game = GameState::generate(current_n, current_threshold, current_mode);
            scroll_y = 0.0;
        }
        if draw_btn("New Game", Color::new(0.6, 0.4, 0.1, 1.0)) {
            game = GameState::generate(current_n, current_threshold, current_mode);
            scroll_y = 0.0;
        }
        if draw_btn("Thresh (+)", Color::new(0.2, 0.3, 0.5, 1.0)) {
            current_threshold += 0.5;
            game = GameState::generate(current_n, current_threshold, current_mode);
            scroll_y = 0.0;
        }
        if draw_btn("Thresh (-)", Color::new(0.5, 0.2, 0.2, 1.0)) {
            current_threshold = (current_threshold - 0.5).max(0.1);
            game = GameState::generate(current_n, current_threshold, current_mode);
            scroll_y = 0.0;
        }

        let mode_color = match current_mode {
            GameMode::Random => Color::new(0.2, 0.5, 0.8, 1.0),
            GameMode::XorRandom => Color::new(0.6, 0.2, 0.6, 1.0),
            GameMode::XorRing => Color::new(0.8, 0.2, 0.2, 1.0),
            GameMode::AESAvalanche => Color::new(0.9, 0.2, 0.2, 1.0),
        };

        if draw_btn(current_mode.to_string(), mode_color) {
            current_mode = current_mode.next();
            game = GameState::generate(current_n, current_threshold, current_mode);
            scroll_y = 0.0;
        }

        next_frame().await
    }
}
