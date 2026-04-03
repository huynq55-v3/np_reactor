use ::rand::{Rng, seq::SliceRandom as _};
use macroquad::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::env;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;

fn macroquad_getrandom(buf: &mut [u8]) -> Result<(), getrandom::Error> {
    for byte in buf.iter_mut() {
        *byte = macroquad::rand::gen_range(0, 255) as u8;
    }
    Ok(())
}
getrandom::register_custom_getrandom!(macroquad_getrandom);

// Tên biến dạng số từ 1 đến N
fn generate_var_name(idx: usize) -> String {
    (idx + 1).to_string()
}

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
// CẤU TRÚC DỮ LIỆU MẠNG (JSON)
// ==========================================
#[derive(Clone, Serialize, Deserialize)]
struct Clause {
    literals: Vec<(usize, bool)>,
}

#[derive(Clone, Serialize, Deserialize)]
struct ServerMessage {
    n: usize,
    vars: Vec<bool>,
    var_names: Vec<String>,
    clauses: Vec<Clause>,
    ever_unsat: Vec<bool>,
    assigned_unsat: Vec<usize>,
    global_unsat_count: usize,
    steps: u32,
    is_won: bool,
}

#[derive(Serialize, Deserialize)]
enum ClientMessage {
    FlipVar(usize),
}

// ==========================================
// SERVER LOGIC (CHIA BÀI ĐỘNG)
// ==========================================
struct ServerState {
    n: usize,
    vars: Vec<bool>,
    var_names: Vec<String>,
    clauses: Vec<Clause>,
    ever_unsat: Vec<bool>,
    steps: u32,
    clients: Vec<TcpStream>,
}

impl ServerState {
    fn generate_aes_256() -> Self {
        let n = 256;
        let mut rng = ::rand::thread_rng();
        let mut var_names = Vec::new();
        for i in 0..n {
            var_names.push(generate_var_name(i));
        }

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
            let selected_vars = available_vars[0..k].to_vec();

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

                    // SẮP XẾP CÁC BIẾN TĂNG DẦN TRONG MỆNH ĐỀ
                    literals.sort_by_key(|&(v, _)| v);

                    clauses.push(Clause { literals });
                }
            }
        }

        let mut initial_vars = vec![false; n];
        for i in 0..n {
            initial_vars[i] = rng.gen_bool(0.5);
        }
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
                ever_unsat[i] = true;
            }
        }

        Self {
            n,
            vars: initial_vars,
            var_names,
            clauses,
            ever_unsat,
            steps: 0,
            clients: Vec::new(),
        }
    }

    fn broadcast(&mut self) {
        let mut unsat_indices = Vec::new();
        for (i, clause) in self.clauses.iter().enumerate() {
            let mut clause_sat = false;
            for &(v_idx, required_sign) in &clause.literals {
                if self.vars[v_idx] == required_sign {
                    clause_sat = true;
                    break;
                }
            }
            if !clause_sat {
                unsat_indices.push(i);
                self.ever_unsat[i] = true;
            }
        }

        let global_unsat_count = unsat_indices.len();
        let is_won = global_unsat_count == 0;
        let mut rng = ::rand::thread_rng();
        unsat_indices.shuffle(&mut rng);

        let num_clients = self.clients.len();
        if num_clients == 0 {
            return;
        }

        let chunk_size = (global_unsat_count as f32 / num_clients as f32).ceil() as usize;
        let mut active_clients = Vec::new();

        for (c_idx, mut stream) in self.clients.drain(..).enumerate() {
            let start = (c_idx * chunk_size).min(global_unsat_count);
            let end = ((c_idx + 1) * chunk_size).min(global_unsat_count);
            let assigned_unsat = if start < end {
                unsat_indices[start..end].to_vec()
            } else {
                Vec::new()
            };

            let msg = ServerMessage {
                n: self.n,
                vars: self.vars.clone(),
                var_names: self.var_names.clone(),
                clauses: self.clauses.clone(),
                ever_unsat: self.ever_unsat.clone(),
                assigned_unsat,
                global_unsat_count,
                steps: self.steps,
                is_won,
            };

            let json = serde_json::to_string(&msg).unwrap() + "\n";
            if stream.write_all(json.as_bytes()).is_ok() {
                active_clients.push(stream);
            }
        }
        self.clients = active_clients;
    }
}

fn run_server() {
    println!("Server NP-Reactor: 127.0.0.1:8888...");
    let listener = TcpListener::bind("127.0.0.1:8888").unwrap();
    let state = Arc::new(Mutex::new(ServerState::generate_aes_256()));

    let state_clone = Arc::clone(&state);
    thread::spawn(move || {
        for stream in listener.incoming() {
            if let Ok(stream) = stream {
                let mut s = state_clone.lock().unwrap();
                s.clients.push(stream.try_clone().unwrap());
                s.broadcast();

                let s_clone2 = Arc::clone(&state_clone);
                thread::spawn(move || {
                    let reader = BufReader::new(stream);
                    for line in reader.lines() {
                        if let Ok(l) = line {
                            if let Ok(ClientMessage::FlipVar(idx)) = serde_json::from_str(&l) {
                                let mut st = s_clone2.lock().unwrap();
                                st.vars[idx] = !st.vars[idx];
                                st.steps += 1;
                                st.broadcast();
                            }
                        }
                    }
                });
            }
        }
    });
    loop {
        thread::sleep(std::time::Duration::from_secs(100));
    }
}

// ==========================================
// CLIENT LOGIC
// ==========================================
fn window_conf() -> Conf {
    Conf {
        window_title: "NP-Reactor: Co-op Node".to_owned(),
        window_width: 1200,
        window_height: 800,
        high_dpi: true,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() > 1 && args[1] == "server" {
        run_server();
        return;
    }

    let stream = TcpStream::connect("127.0.0.1:8888").expect("Run server first!");
    let mut writer = stream.try_clone().unwrap();
    let (tx, rx) = mpsc::channel::<ServerMessage>();

    thread::spawn(move || {
        let reader = BufReader::new(stream);
        for line in reader.lines() {
            if let Ok(l) = line {
                if let Ok(msg) = serde_json::from_str::<ServerMessage>(&l) {
                    tx.send(msg).unwrap();
                }
            }
        }
    });

    let font_bytes = std::fs::read("font.ttf");
    let custom_font = match font_bytes {
        Ok(bytes) => load_ttf_font_from_bytes(&bytes).ok(),
        Err(_) => None,
    };

    let mut current_state: Option<ServerMessage> = None;
    let mut scroll_y = 0.0;
    let mut max_scroll = 0.0;
    let mut cooldown_timer = 0.0;

    loop {
        while let Ok(msg) = rx.try_recv() {
            current_state = Some(msg);
        }
        if cooldown_timer > 0.0 {
            cooldown_timer -= get_frame_time();
        }

        clear_background(Color::new(0.1, 0.12, 0.15, 1.0));
        let sw = screen_width();
        let sh = screen_height();

        if let Some(state) = &current_state {
            let (_, mouse_wheel_y) = mouse_wheel();
            scroll_y -= mouse_wheel_y * 35.0;
            scroll_y = scroll_y.clamp(0.0, max_scroll);
            let offset_y = -scroll_y;

            // HEADER
            draw_rectangle(0.0, 0.0, sw, 80.0, Color::new(0.15, 0.18, 0.22, 0.95));
            draw_text(
                &format!(
                    "ERRORS: {} | STEPS: {}",
                    state.global_unsat_count, state.steps
                ),
                20.0,
                35.0,
                22.0,
                RED,
            );
            let cool_col = if cooldown_timer > 0.0 { YELLOW } else { GREEN };
            draw_text(
                &format!("COOLDOWN: {:.1}s", cooldown_timer.max(0.0)),
                sw - 180.0,
                35.0,
                22.0,
                cool_col,
            );
            draw_text(
                &format!("TASKS: {} Clauses", state.assigned_unsat.len()),
                20.0,
                65.0,
                18.0,
                WHITE,
            );

            if state.is_won {
                draw_text("SYSTEM DECRYPTED!", sw / 2.0 - 150.0, sh / 2.0, 40.0, GOLD);
                next_frame().await;
                continue;
            }

            // FILTER VARIABLES
            let mut active_vars = HashSet::new();
            for &c_idx in &state.assigned_unsat {
                for &(v, _) in &state.clauses[c_idx].literals {
                    active_vars.insert(v);
                }
            }
            let mut active_vars_vec: Vec<usize> = active_vars.into_iter().collect();
            active_vars_vec.sort();

            // RENDER VARIABLES (SQUARES)
            let var_size = 35.0; // TĂNG LÊN 35.0
            let mut vx = 20.0;
            let mut vy = 110.0;

            for &v_idx in &active_vars_vec {
                if vx + var_size > sw - 20.0 {
                    vx = 20.0;
                    vy += 60.0;
                }
                let draw_y = vy + offset_y;

                if draw_y > 40.0 && draw_y < sh {
                    // RADAR
                    let (mut f, mut b) = (0, 0);
                    for cl in &state.clauses {
                        let (mut cur, mut other) = (false, false);
                        for &(lv, rs) in &cl.literals {
                            if state.vars[lv] == rs {
                                cur = true;
                            }
                            if lv != v_idx && state.vars[lv] == rs {
                                other = true;
                            }
                        }
                        if !cur {
                            for &(lv, rs) in &cl.literals {
                                if lv == v_idx && !state.vars[v_idx] == rs {
                                    f += 1;
                                }
                            }
                        } else if cur && !other {
                            b += 1;
                        }
                    }

                    let rad_txt = format!("{}/{}", f, b);
                    let r_col = if b > f {
                        RED
                    } else if f > 0 {
                        GREEN
                    } else {
                        GRAY
                    };
                    let r_dim = measure_text(&rad_txt, None, 12, 1.0);
                    draw_text(
                        &rad_txt,
                        vx + (var_size - r_dim.width) / 2.0,
                        draw_y - 5.0,
                        12.0,
                        r_col,
                    );

                    let (bg, tx) = if state.vars[v_idx] {
                        (WHITE, BLACK)
                    } else {
                        (BLACK, WHITE)
                    };
                    draw_rectangle(vx, draw_y, var_size, var_size, bg);

                    // CĂN GIỮA SỐ TRONG Ô VUÔNG
                    let name = &state.var_names[v_idx];
                    let n_dim = measure_sym(name, 16.0, custom_font.as_ref());
                    draw_sym(
                        name,
                        vx + (var_size - n_dim.width) / 2.0,
                        draw_y + (var_size + n_dim.height) / 2.0 - 2.0,
                        16.0,
                        tx,
                        custom_font.as_ref(),
                    );

                    if cooldown_timer <= 0.0 && is_mouse_button_pressed(MouseButton::Left) {
                        let m = mouse_position();
                        if m.0 >= vx
                            && m.0 <= vx + var_size
                            && m.1 >= draw_y
                            && m.1 <= draw_y + var_size
                        {
                            let msg = ClientMessage::FlipVar(v_idx);
                            writer
                                .write_all((serde_json::to_string(&msg).unwrap() + "\n").as_bytes())
                                .unwrap();
                            cooldown_timer = 5.0;
                        }
                    }
                }
                vx += var_size + 10.0;
            }

            // RENDER CLAUSES (RECTANGLES) - Xếp hàng ngang
            let mut cx = 20.0;
            let mut cy = vy + 70.0;
            for &c_idx in &state.assigned_unsat {
                let cl = &state.clauses[c_idx];
                let lit_w = 40.0; // TĂNG LÊN 40.0
                let lit_h = 28.0; // TĂNG LÊN 28.0
                let total_w =
                    (lit_w * cl.literals.len() as f32) + (2.0 * (cl.literals.len() - 1) as f32);

                if cx + total_w > sw - 20.0 {
                    cx = 20.0;
                    cy += 45.0; // Khoảng cách hàng dọc tăng lên cho vừa ô chữ nhật to hơn
                }

                let draw_cy = cy + offset_y;
                if draw_cy > 50.0 && draw_cy < sh {
                    let border = if !state.ever_unsat[c_idx] {
                        RED
                    } else {
                        YELLOW
                    };
                    draw_rectangle_lines(
                        cx - 3.0,
                        draw_cy - 3.0,
                        total_w + 6.0,
                        lit_h + 6.0,
                        2.0,
                        border,
                    );

                    let mut lx = cx;
                    for &(vi, rs) in &cl.literals {
                        let (bg, tx) = if rs { (WHITE, BLACK) } else { (BLACK, WHITE) };
                        draw_rectangle(lx, draw_cy, lit_w, lit_h, bg);

                        let ln = &state.var_names[vi];
                        let l_dim = measure_sym(ln, 14.0, custom_font.as_ref()); // Tăng font
                        draw_sym(
                            ln,
                            lx + (lit_w - l_dim.width) / 2.0,
                            draw_cy + (lit_h + l_dim.height) / 2.0 - 2.0,
                            14.0,
                            tx,
                            custom_font.as_ref(),
                        );
                        lx += lit_w + 2.0;
                    }
                }
                cx += total_w + 15.0;
            }
            max_scroll = (cy + 60.0 - sh).max(0.0);
        }
        next_frame().await
    }
}
