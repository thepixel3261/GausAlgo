use leptos::prelude::*;

#[derive(Clone, Debug)]
struct NormalizedDisplay {
    original: String,
    normalized: String,
}

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

#[component]
pub fn App() -> impl IntoView {
    let (equation_input, set_equation_input) = signal(String::new());
    let (normalized_eqs, set_normalized_eqs) = signal(Vec::<NormalizedDisplay>::new());
    let (step_data_list, set_step_data_list) = signal(Vec::<StepData>::new());
    let (current_step, set_current_step) = signal(0usize);
    let (solutions, set_solutions) = signal(Vec::<SolutionData>::new());
    let (error_msg, set_error_msg) = signal(String::new());
    let (show_solution, set_show_solution) = signal(false);
    let (is_parametric, set_is_parametric) = signal(false);

    let total_steps = move || step_data_list.get().len();

    let solve = move |_| {
        let input = equation_input.get();
        let lines: Vec<&str> = input.lines()
            .map(|l| l.trim())
            .filter(|l| !l.is_empty())
            .collect();

        if lines.is_empty() {
            set_error_msg.set("Bitte geben Sie mindestens eine Gleichung ein.".to_string());
            return;
        }

        let mut equations = Vec::new();
        let mut norm_displays = Vec::new();
        for line in &lines {
            match crate::parser::parse_equation(line) {
                Ok(eq) => {
                    norm_displays.push(NormalizedDisplay {
                        original: line.to_string(),
                        normalized: format_normalized_eq(&eq),
                    });
                    equations.push(eq);
                }
                Err(e) => {
                    set_error_msg.set(format!("Fehler beim Parsen von '{}': {}", line, e));
                    return;
                }
            }
        }

        set_normalized_eqs.set(norm_displays);
        set_error_msg.set(String::new());

        let (matrix, _all_vars) = crate::gauss::AugmentedMatrix::from_equations(&equations);
        let (steps, solution) = crate::gauss::gaussian_elimination(&matrix);

        let step_data: Vec<StepData> = steps.iter().map(|s| {
            let n = s.matrix.rows;
            let mut rows: Vec<StepRowDisplay> = (0..n).map(|r| {
                let mut cells = Vec::new();
                for j in 0..s.matrix.cols {
                    let val = &s.matrix.data[r][j];
                    cells.push(format_cell(val));
                }
                cells.push(format_cell(&s.matrix.data[r][s.matrix.cols]));
                let op_class = if r < s.row_ops.len() {
                    match s.row_ops[r].op_type {
                        crate::gauss::OpType::Eliminate => "op-elim",
                        crate::gauss::OpType::Swap => "op-swap",
                        crate::gauss::OpType::NoChange => "op-none",
                    }
                } else { "op-none" };
                StepRowDisplay {
                    cells,
                    arrow_src: None,
                    arrow_dst: None,
                    op_class: op_class.to_string(),
                }
            }).collect();

            for (i, op) in s.row_ops.iter().enumerate() {
                if i < n {
                    rows[i].arrow_src = op.arrow_src.clone();
                    rows[i].arrow_dst = op.arrow_dst.clone();
                }
            }

            StepData {
                rows,
            }
        }).collect();

        set_step_data_list.set(step_data);
        set_current_step.set(0);

        match &solution {
            crate::gauss::Solution::Unique(sol) => {
                let sol_data: Vec<SolutionData> = sol.iter().map(|(v, val)| {
                    SolutionData {
                        var: v.clone(),
                        value: if val.is_integer() {
                            format!("{}", val.num)
                        } else {
                            format!("{}", val)
                        },
                    }
                }).collect();
                set_solutions.set(sol_data);
                set_is_parametric.set(false);
                set_show_solution.set(true);
            }
            crate::gauss::Solution::Parametric(params) => {
                let sol_data: Vec<SolutionData> = params.iter().map(|p| {
                    SolutionData {
                        var: p.var.clone(),
                        value: p.expr.clone(),
                    }
                }).collect();
                set_solutions.set(sol_data);
                set_is_parametric.set(true);
                set_show_solution.set(true);
            }
            crate::gauss::Solution::NoSolution => {
                set_solutions.set(vec![SolutionData {
                    var: "System".to_string(),
                    value: "Keine Lösung (inkonsistent)".to_string(),
                }]);
                set_is_parametric.set(false);
                set_show_solution.set(true);
            }
        }
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

    view! {
        <div class="app-container">
            <header class="app-header">
                <h1>"Gauß-Algorithmus Löser"</h1>
            </header>

            <main class="app-main">
                <section class="input-section">
                    <h2>"Gleichungen eingeben"</h2>
                    <textarea
                        class="equation-input"
                        placeholder="Eine Gleichung pro Zeile, z.B.:&#10;1x + 2 + 3y = 2i + 4&#10;1*6x + 3/5y - 2/7i = 4*6-2&#10;3x - y + z = 1"
                        prop:value=equation_input
                        on:input=move |ev| {
                            set_equation_input.set(event_target_value(&ev));
                        }
                        rows="5"
                    ></textarea>

                    <div class="button-row">
                        <button class="btn btn-primary" on:click=solve>
                                "System lösen"
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
                            <h2>"Äquivalenzumformung"</h2>
                            {normals.into_iter().map(|n| {
                                view! {
                                    <div class="normalize-row">
                                        <span class="norm-original">{n.original}</span>
                                        <span class="norm-arrow">" → "</span>
                                        <span class="norm-normalized">{n.normalized}</span>
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
                            <h2>"Schritt-für-Schritt-Elimination"</h2>

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
                            <h2>{if is_param { "Lösungsmenge".to_string() } else { "Lösung".to_string() }}</h2>
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

fn format_normalized_eq(eq: &crate::parser::NormalizedEquation) -> String {
    let mut parts = Vec::new();
    for (var, coeff) in &eq.terms {
        if coeff.is_zero() { continue; }
        let abs_c = coeff.abs();
        let c_str = if abs_c.is_one() { String::new() } else if abs_c.is_integer() { format!("{}", abs_c.num) } else { format!("{}", abs_c) };
        if coeff.is_negative() {
            parts.push(format!("-{}{}", c_str, var));
        } else if parts.is_empty() {
            parts.push(format!("{}{}", c_str, var));
        } else {
            parts.push(format!("+ {}{}", c_str, var));
        }
    }
    let const_str = if eq.constant.is_integer() {
        format!("{}", eq.constant.num)
    } else {
        format!("{}", eq.constant)
    };
    if parts.is_empty() {
        format!("0 = {}", const_str)
    } else {
        format!("{} = {}", parts.join(" "), const_str)
    }
}

fn format_cell(val: &crate::rational::Rational) -> String {
    if val.is_integer() {
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
