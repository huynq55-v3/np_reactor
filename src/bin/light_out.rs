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

// Cấu trúc SAT
#[derive(Clone)]
struct Clause {
    literals: Vec<(usize, bool)>,
}

struct GameState {
    n: usize,
    vars: Vec<bool>,        // Đây là các "Nước đi" ông đang thử (1-25)
    clauses: Vec<Clause>,   // Ràng buộc của 25 bóng đèn
    initial_map: Vec<bool>, // Bản đồ bóng đèn ban đầu (Sáng/Tối)
    is_won: bool,
    steps: u32,
    last_flipped: Option<usize>,
    target_solution: Vec<bool>, // Đáp án máy tính giải sẵn để đối chiếu
}

impl GameState {
    fn generate() -> Self {
        let n = 25;
        let mut rng = ::rand::thread_rng();

        // 1. Tạo một "Bản đồ ban đầu" ngẫu nhiên nhưng đảm bảo giải được
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
            initial_map[i] = parity; // Bóng i sẽ sáng nếu tổng số lần bấm hàng xóm là lẻ
        }

        // 2. Chuyển đổi thành SAT Clauses
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

            // Ép hàm XOR: sum(neighbors) == initial_map[i]
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

        Self {
            n,
            vars: vec![false; n], // Bắt đầu bằng việc chưa bấm nút nào
            clauses,
            initial_map,
            is_won: false,
            steps: 0,
            last_flipped: None,
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

#[macroquad::main("Lights Out SAT Converter")]
async fn main() {
    let mut game = GameState::generate();
    let mut scroll_y = 0.0;

    loop {
        clear_background(Color::new(0.1, 0.1, 0.12, 1.0));
        let sw = screen_width();
        let sh = screen_height();

        // UI Header
        draw_text("LIGHTS OUT SAT CONVERTER", 20.0, 30.0, 25.0, WHITE);
        draw_text(
            &format!(
                "Steps: {} | Satisfaction: {}",
                game.steps,
                if game.is_won { "SOLVED" } else { "UNSAT" }
            ),
            20.0,
            60.0,
            20.0,
            YELLOW,
        );
        draw_text(
            "Top Grid: Your Moves (Variables) | Bottom: Bulbs (Clauses)",
            20.0,
            85.0,
            18.0,
            GRAY,
        );

        // 1. VẼ DÀN BIẾN (Đây là bảng nút bấm để ông giải)
        let var_size = 40.0;
        let gap = 10.0;
        let start_x = 20.0;
        let start_y = 110.0;

        for i in 0..game.n {
            let x = start_x + (i % 5) as f32 * (var_size + gap);
            let y = start_y + (i / 5) as f32 * (var_size + gap);

            let (bg, txt) = if game.vars[i] {
                (WHITE, BLACK)
            } else {
                (BLACK, WHITE)
            };
            draw_rectangle(x, y, var_size, var_size, bg);

            // Highlight nước đi đúng từ đáp án SAT (Nếu giữ Space)
            if is_key_down(KeyCode::Space) && game.target_solution[i] {
                draw_rectangle_lines(x - 3.0, y - 3.0, var_size + 6.0, var_size + 6.0, 3.0, GREEN);
            }

            draw_text(&format!("{}", i + 1), x + 12.0, y + 26.0, 18.0, txt);

            if is_mouse_button_pressed(MouseButton::Left) {
                let m = mouse_position();
                if m.0 >= x && m.0 <= x + var_size && m.1 >= y && m.1 <= y + var_size {
                    game.vars[i] = !game.vars[i]; // CHỈ LẬT ĐÚNG 1 BIT NÀY
                    game.steps += 1;
                    game.check();
                }
            }
        }

        // 2. VẼ TRẠNG THÁI CÁC BÓNG ĐÈN (Dựa trên bản đồ ban đầu + Nước đi của ông)
        // Đây chính là việc "Visualize" SAT
        let map_x = 350.0;
        draw_text(
            "CURRENT BULB STATES (The Constraints)",
            map_x,
            100.0,
            20.0,
            SKYBLUE,
        );
        for i in 0..game.n {
            let r = i / 5;
            let c = i % 5;
            let x = map_x + c as f32 * (var_size + gap);
            let y = start_y + r as f32 * (var_size + gap);

            // Logic: Trạng thái đèn = Ban đầu XOR (Tổng các nước đi xung quanh)
            let mut current_parity = game.initial_map[i];
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
            for &idx in &neighbors {
                current_parity ^= game.vars[idx];
            }

            let color = if current_parity {
                YELLOW
            } else {
                Color::new(0.2, 0.2, 0.2, 1.0)
            };
            draw_rectangle(x, y, var_size, var_size, color);
            draw_rectangle_lines(x, y, var_size, var_size, 1.0, GRAY);
            if current_parity {
                draw_text("ON", x + 10.0, y + 26.0, 15.0, BLACK);
            }
        }

        if game.is_won {
            draw_text("MAP SOLVED!", sw / 2.0 - 50.0, sh - 50.0, 30.0, GOLD);
        } else {
            draw_text(
                "HOLD [SPACE] TO SEE SAT SOLUTION",
                20.0,
                sh - 30.0,
                18.0,
                GREEN,
            );
        }

        next_frame().await
    }
}
