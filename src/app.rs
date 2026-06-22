use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::{Serialize, Deserialize};
use js_sys::{Reflect, JsString, Function, Array};
use wasm_bindgen::{JsCast, JsValue};

/// Call a Tauri command from WASM via window.__TAURI_INTERNALS__.invoke()
async fn tauri_invoke<T: serde::de::DeserializeOwned>(cmd: &str, args: &impl serde::Serialize) -> Result<T, String> {
    let global = js_sys::global();
    let internals = Reflect::get(&global, &JsString::from("__TAURI_INTERNALS__"))
        .map_err(|_| "Nicht innerhalb von Tauri ausgeführt".to_string())?;
    let invoke_val = Reflect::get(&internals, &JsString::from("invoke"))
        .map_err(|_| "Tauri invoke nicht gefunden".to_string())?;
    let invoke_fn: Function = invoke_val.dyn_into()
        .map_err(|_| "invoke ist keine Funktion".to_string())?;

    let args_val = serde_wasm_bindgen::to_value(args)
        .map_err(|e| e.to_string())?;
    let cmd_val: JsValue = JsString::from(cmd).into();

    let promise_args = Array::new();
    promise_args.push(&cmd_val);
    promise_args.push(&args_val);

    let promise_val = invoke_fn.apply(&internals, &promise_args)
        .map_err(|e| format!("invoke-Aufruf fehlgeschlagen: {:?}", e))?;

    let promise: js_sys::Promise = promise_val.dyn_into()
        .map_err(|_| "invoke hat kein Promise zurückgegeben".to_string())?;

    let result = wasm_bindgen_futures::JsFuture::from(promise)
        .await
        .map_err(|e| format!("invoke abgelehnt: {:?}", e))?;

    serde_wasm_bindgen::from_value(result)
        .map_err(|e| e.to_string())
}

// ── Cross-boundary types (mirrors backend) ──

#[derive(Clone, Debug, Serialize, Deserialize)]
struct CellVal {
    num: i64,
    den: i64,
}

#[derive(Clone, Debug, Deserialize)]
struct RowOpInfo {
    op_type: String,
    arrow_src: Option<String>,
    arrow_dst: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
struct StepInfo {
    matrix: Vec<Vec<CellVal>>,
    vars: Vec<String>,
    row_ops: Vec<RowOpInfo>,
}

#[derive(Clone, Debug, Deserialize)]
struct SolveResponse {
    steps: Vec<StepInfo>,
    normalized_eqs: Vec<String>,
    solution_type: String,
    solution_vars: Vec<String>,
    solution_values: Vec<String>,
}

// ── Frontend types ──

#[derive(Clone, Debug)]
struct StepRowDisplay {
    cells: Vec<String>,
    arrow_src: Option<String>,
    arrow_dst: Option<String>,
    op_class: String,
}

#[derive(Clone, Debug)]
struct StepData {
    rows: Vec<StepRowDisplay>,
}

#[derive(Clone, Debug)]
struct SolutionData {
    var: String,
    value: String,
}

// ── Helpers ──

fn format_cell(val: &CellVal) -> String {
    if val.den == 1 {
        if val.num >= 0 {
            format!(" {} ", val.num)
        } else {
            format!("{}", val.num)
        }
    } else {
        if val.num < 0 {
            format!("-{}/{}", -val.num, val.den)
        } else {
            format!("{}/{}", val.num, val.den)
        }
    }
}

// ── Component ──

#[component]
pub fn App() -> impl IntoView {
    let (equation_input, set_equation_input) = signal(String::new());
    let (normalized_eqs, set_normalized_eqs) = signal(Vec::<String>::new());
    let (step_data_list, set_step_data_list) = signal(Vec::<StepData>::new());
    let (current_step, set_current_step) = signal(0usize);
    let (solutions, set_solutions) = signal(Vec::<SolutionData>::new());
    let (error_msg, set_error_msg) = signal(String::new());
    let (show_solution, set_show_solution) = signal(false);
    let (is_parametric, set_is_parametric) = signal(false);
    let (loading, set_loading) = signal(false);

    let total_steps = move || step_data_list.get().len();

    let solve = move |_| {
        let input = equation_input.get();
        let lines: Vec<String> = input.lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .collect();

        if lines.is_empty() {
            set_error_msg.set("Bitte geben Sie mindestens eine Gleichung ein.".to_string());
            return;
        }

        set_error_msg.set(String::new());
        set_loading.set(true);

        let lines_clone = lines.clone();
        spawn_local(async move {
            match tauri_invoke::<SolveResponse>("solve", &serde_json::json!({ "equations": lines_clone })).await {
                Ok(resp) => {
                    set_normalized_eqs.set(resp.normalized_eqs);

                    let step_data: Vec<StepData> = resp.steps.iter().map(|s| {
                        let n = s.matrix.len();
                        let rows: Vec<StepRowDisplay> = (0..n).map(|r| {
                            let mut cells = Vec::new();
                            for val in &s.matrix[r] {
                                cells.push(format_cell(val));
                            }
                            let op_class = if r < s.row_ops.len() {
                                match s.row_ops[r].op_type.as_str() {
                                    "Eliminate" => "op-elim",
                                    "Swap" => "op-swap",
                                    _ => "op-none",
                                }
                            } else { "op-none" };
                            StepRowDisplay {
                                cells,
                                arrow_src: if r < s.row_ops.len() { s.row_ops[r].arrow_src.clone() } else { None },
                                arrow_dst: if r < s.row_ops.len() { s.row_ops[r].arrow_dst.clone() } else { None },
                                op_class: op_class.to_string(),
                            }
                        }).collect();
                        StepData { rows }
                    }).collect();

                    set_step_data_list.set(step_data);
                    set_current_step.set(0);

                    match resp.solution_type.as_str() {
                        "unique" => {
                            let sol_data: Vec<SolutionData> = resp.solution_vars.into_iter()
                                .zip(resp.solution_values.into_iter())
                                .map(|(var, value)| SolutionData { var, value })
                                .collect();
                            set_solutions.set(sol_data);
                            set_is_parametric.set(false);
                            set_show_solution.set(true);
                        }
                        "parametric" => {
                            let sol_data: Vec<SolutionData> = resp.solution_vars.into_iter()
                                .zip(resp.solution_values.into_iter())
                                .map(|(var, value)| SolutionData { var, value })
                                .collect();
                            let param_count = sol_data.len();
                            set_solutions.set(sol_data);
                            set_is_parametric.set(true);
                            set_show_solution.set(true);
                        }
                        "no_solution" => {
                            set_solutions.set(vec![SolutionData {
                                var: "System".to_string(),
                                value: "Keine Lösung (inkonsistent)".to_string(),
                            }]);
                            set_is_parametric.set(false);
                            set_show_solution.set(true);
                        }
                        _ => {}
                    }
                }
                Err(e) => {
                    set_error_msg.set(format!("Fehler: {}", e));
                }
            }
            set_loading.set(false);
        });
    };

    let prev_step = move |_| {
        let cur = current_step.get();
        if cur > 0 {
            set_current_step.set(cur - 1);
        }
    };

    let next_step = move |_| {
        let cur = current_step.get();
        let total = total_steps();
        if total > 0 && cur < total - 1 {
            set_current_step.set(cur + 1);
        }
    };

    let reset_playback = move |_| {
        set_current_step.set(0);
    };

    let go_end = move |_| {
        let total = total_steps();
        if total > 0 {
            set_current_step.set(total - 1);
        }
    };

    let clear_all = move |_| {
        set_equation_input.set(String::new());
        set_normalized_eqs.set(Vec::new());
        set_step_data_list.set(Vec::new());
        set_solutions.set(Vec::new());
        set_current_step.set(0);
        set_show_solution.set(false);
        set_error_msg.set(String::new());
    };

    let placeholder_text = "Geben Sie eine Gleichung pro Zeile ein, z.B.:\n1x + 2 + 3y = 2i + 4\n1*6x + 3/5y - 2/7i = 4*6-2\n3x - y + z = 1";

    view! {
        <div class="app-container">
            <header class="app-header">
                <h1>"Gauß-Algorithmus Löser"</h1>
                <p class="subtitle">"Lineare Gleichungssysteme lösen · Schritt-für-Schritt-Wiedergabe · Beliebige Variablen"</p>
            </header>

            <main class="app-main">
                <section class="input-section">
                    <h2>"Gleichungen eingeben"</h2>
                    <textarea
                        class="equation-input"
                        placeholder={placeholder_text}
                        prop:value=equation_input
                        on:input=move |ev| {
                            set_equation_input.set(event_target_value(&ev));
                        }
                        rows="5"
                    ></textarea>

                    <div class="button-row">
                        <button class="btn btn-primary" on:click=solve disabled=move||{loading.get()}>
                            {move || if loading.get() { "Löse..." } else { "System lösen" }}
                        </button>
                        <button class="btn btn-secondary" on:click=clear_all>
                            "Zurücksetzen"
                        </button>
                    </div>

                    {move || {
                        let err = error_msg.get();
                        if !err.is_empty() {
                            view! { <div class="error-msg">{err}</div> }.into_any()
                        } else {
                            view! { <div></div> }.into_any()
                        }
                    }}
                </section>

                {move || {
                    let normals = normalized_eqs.get();
                    if normals.is_empty() {
                        return view! { <div></div> }.into_any();
                    }
                    view! {
                        <section class="normalize-section">
                            <h2>"Äquivalente Umformung"</h2>
                            {normals.into_iter().map(|n| {
                                view! {
                                    <div class="normalize-row">
                                        <span class="norm-normalized">{n}</span>
                                    </div>
                                }
                            }).collect_view()}
                        </section>
                    }.into_any()
                }}

                {move || {
                    let steps = step_data_list.get();
                    if steps.is_empty() {
                        return view! { <div></div> }.into_any();
                    }

                    let cur = current_step.get();
                    let total = steps.len();
                    let step = steps[cur].clone();
                    let step_rows = step.rows;
                    let cur_plus_one = cur + 1;

                    view! {
                        <section class="playback-section">
                            <h2>"Schrittweise Elimination"</h2>

                            <div class="step-counter">
                                <span class="step-badge">{format!("Schritt {} von {}", cur_plus_one, total)}</span>
                            </div>

                            <div class="matrix-container">
                                <div class="matrix-layout">
                                    <div class="matrix-wrapper">
                                        <div class="matrix-bracket matrix-bracket-left"></div>
                                        <div class="matrix-grid">
                                            {step_rows.iter().map(|row| {
                                                view! {
                                                    <div class="matrix-row">
                                                        <div class="matrix-cells">
                                                            {let cc = row.cells.len();
                                                            row.cells.iter().enumerate().map(|(col_idx, cell)| {
                                                                let is_last = col_idx == cc - 1;
                                                                let cell_cls = if is_last { "matrix-cell sep" } else { "matrix-cell" };
                                                                view! { <span class={cell_cls}>{cell.clone()}</span> }
                                                            }).collect_view()}
                                                        </div>
                                                    </div>
                                                }
                                            }).collect_view()}
                                        </div>
                                        <div class="matrix-bracket matrix-bracket-right"></div>
                                    </div>
                                    <div class="matrix-ops">
                                        {step_rows.iter().map(|row| {
                                            let arrow_src = row.arrow_src.clone();
                                            let arrow_dst = row.arrow_dst.clone();
                                            let op_class = row.op_class.clone();
                                            view! {
                                                <div class={format!("row-op {}", op_class)}>
                                                    {if let Some(s) = arrow_src.clone() {
                                                        view! { <span class="op-src">{s}</span> }.into_any()
                                                    } else if let Some(d) = arrow_dst.clone() {
                                                        view! { <span class="op-dst">{d}</span> }.into_any()
                                                    } else {
                                                        view! { <span></span> }.into_any()
                                                    }}
                                                </div>
                                            }
                                        }).collect_view()}
                                    </div>
                                </div>
                            </div>

                            <div class="playback-controls">
                                <button class="btn btn-control" on:click=reset_playback disabled=move||{current_step.get()==0}>
                                    "⏮"
                                </button>
                                <button class="btn btn-control" on:click=prev_step disabled=move||{current_step.get()==0}>
                                    "◀"
                                </button>
                                <span class="step-indicator">{format!("{}/{}", cur_plus_one, total)}</span>
                                <button class="btn btn-control" on:click=next_step disabled=move||{current_step.get()>=total_steps()-1}>
                                    "▶"
                                </button>
                                <button class="btn btn-control" on:click=go_end disabled=move||{current_step.get()>=total_steps()-1}>
                                    "⏭"
                                </button>
                            </div>
                        </section>
                    }.into_any()
                }}

                {move || {
                    if !show_solution.get() {
                        return view! { <div></div> }.into_any();
                    }

                    let sols = solutions.get();
                    let is_param = is_parametric.get();
                    let param_count = sols.len();

                    view! {
                        <section class="solution-section">
                            <h2>{if is_param { "Lösungsmannigfaltigkeit".to_string() } else { "Lösung".to_string() }}</h2>
                            <div class="solution-grid">
                                {sols.into_iter().map(|s| {
                                    view! {
                                        <div class="solution-row">
                                            <span class="solution-var">{s.var}</span>
                                            <span class="solution-eq">"="</span>
                                            <span class="solution-val">{s.value}</span>
                                        </div>
                                    }
                                }).collect_view()}
                            </div>
                            {if is_param && param_count > 0 {
                                view! {
                                    <p class="param-note">
                                        "Freie Parameter ∈ ℝ"
                                    </p>
                                }.into_any()
                            } else {
                                view! { <div></div> }.into_any()
                            }}
                        </section>
                    }.into_any()
                }}
            </main>
        </div>
    }
}
