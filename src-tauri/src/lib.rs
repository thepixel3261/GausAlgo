mod rational;
mod parser;
mod gauss;

use serde::{Serialize, Deserialize};

/// Serializable cell value: raw num/den so frontend can format
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CellVal {
    pub num: i128,
    pub den: i128,
}

impl From<rational::Rational> for CellVal {
    fn from(r: rational::Rational) -> Self {
        CellVal { num: r.num, den: r.den }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RowOpInfo {
    pub op_type: String,
    pub arrow_src: Option<String>,
    pub arrow_dst: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StepInfo {
    pub matrix: Vec<Vec<CellVal>>,
    pub vars: Vec<String>,
    pub row_ops: Vec<RowOpInfo>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SolveResponse {
    pub steps: Vec<StepInfo>,
    pub normalized_eqs: Vec<String>,
    pub solution_type: String,
    pub solution_vars: Vec<String>,
    pub solution_values: Vec<String>,
}

fn format_normalized_eq(eq: &parser::NormalizedEquation) -> String {
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

#[tauri::command]
fn solve(equations: Vec<String>) -> Result<SolveResponse, String> {
    if equations.is_empty() {
        return Err("Bitte geben Sie mindestens eine Gleichung ein.".to_string());
    }

    let mut eqs = Vec::new();
    let mut norm_strings = Vec::new();
    for line in &equations {
        match parser::parse_equation(line) {
            Ok(eq) => {
                norm_strings.push(format_normalized_eq(&eq));
                eqs.push(eq);
            }
            Err(e) => return Err(format!("Fehler beim Parsen von '{}': {}", line, e)),
        }
    }

    let (matrix, _all_vars) = gauss::AugmentedMatrix::from_equations(&eqs);
    let (steps, solution) = gauss::gaussian_elimination(&matrix);

    let step_infos: Vec<StepInfo> = steps.iter().map(|s| {
        let step_matrix: Vec<Vec<CellVal>> = s.matrix.data.iter()
            .map(|row| row.iter().map(|v| CellVal::from(v.clone())).collect())
            .collect();
        let row_ops: Vec<RowOpInfo> = s.row_ops.iter().map(|op| RowOpInfo {
            op_type: match op.op_type {
                gauss::OpType::NoChange => "NoChange".to_string(),
                gauss::OpType::Eliminate => "Eliminate".to_string(),
                gauss::OpType::Swap => "Swap".to_string(),
            },
            arrow_src: op.arrow_src.clone(),
            arrow_dst: op.arrow_dst.clone(),
        }).collect();
        StepInfo {
            matrix: step_matrix,
            vars: s.matrix.vars.clone(),
            row_ops,
        }
    }).collect();

    match solution {
        gauss::Solution::Unique(sol) => {
            let sol_vars: Vec<String> = sol.keys().cloned().collect();
            let sol_values: Vec<String> = sol.values().map(|v| {
                if v.is_integer() { format!("{}", v.num) } else { format!("{}", v) }
            }).collect();
            Ok(SolveResponse {
                steps: step_infos,
                normalized_eqs: norm_strings,
                solution_type: "unique".to_string(),
                solution_vars: sol_vars,
                solution_values: sol_values,
            })
        }
        gauss::Solution::Parametric(params) => {
            let sol_vars: Vec<String> = params.iter().map(|p| p.var.clone()).collect();
            let sol_values: Vec<String> = params.iter().map(|p| p.expr.clone()).collect();
            Ok(SolveResponse {
                steps: step_infos,
                normalized_eqs: norm_strings,
                solution_type: "parametric".to_string(),
                solution_vars: sol_vars,
                solution_values: sol_values,
            })
        }
        gauss::Solution::NoSolution => {
            Ok(SolveResponse {
                steps: step_infos,
                normalized_eqs: norm_strings,
                solution_type: "no_solution".to_string(),
                solution_vars: vec!["System".to_string()],
                solution_values: vec!["Keine Lösung (inkonsistent)".to_string()],
            })
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![solve])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
