use crate::rational::Rational;
use crate::parser::NormalizedEquation;
use std::collections::BTreeMap;

#[derive(Clone, Debug)]
pub struct AugmentedMatrix {
    pub data: Vec<Vec<Rational>>,
    pub vars: Vec<String>,
    pub rows: usize,
    pub cols: usize,
}

#[derive(Clone, Debug)]
pub struct RowOp {
    pub op_type: OpType,
    pub arrow_src: Option<String>,
    pub arrow_dst: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum OpType {
    NoChange,
    Eliminate,
    Swap,
}

#[derive(Clone, Debug)]
pub struct EliminationStep {
    pub matrix: AugmentedMatrix,
    pub row_ops: Vec<RowOp>,
}

#[derive(Clone, Debug)]
pub struct ParametricVar {
    pub var: String,
    pub expr: String,
}

#[derive(Clone, Debug)]
pub enum Solution {
    Unique(BTreeMap<String, Rational>),
    Parametric(Vec<ParametricVar>),
    NoSolution,
}

impl AugmentedMatrix {
    pub fn from_equations(equations: &[NormalizedEquation]) -> (Self, Vec<String>) {
        let mut all_vars_set: BTreeMap<String, usize> = BTreeMap::new();
        for eq in equations {
            for var in eq.terms.keys() {
                if !all_vars_set.contains_key(var) {
                    all_vars_set.insert(var.clone(), all_vars_set.len());
                }
            }
        }

        let all_vars: Vec<String> = all_vars_set.keys().cloned().collect();
        let n = equations.len();
        let m = all_vars.len();
        let mut data = Vec::new();

        for eq in equations {
            let mut row = vec![Rational::zero(); m + 1];
            for (var, coeff) in &eq.terms {
                if let Some(idx) = all_vars_set.get(var) {
                    row[*idx] = coeff.clone();
                }
            }
            row[m] = eq.constant.clone();
            data.push(row);
        }

        (AugmentedMatrix { data, vars: all_vars.clone(), rows: n, cols: m }, all_vars)
    }

}

fn swap_label(a: usize, b: usize) -> String {
    format!("R{} ↔ R{}", a + 1, b + 1)
}

fn format_multiplier(val: &Rational) -> String {
    if val.is_integer() {
        format!("{}", val.num)
    } else {
        format!("{}", val)
    }
}

pub fn gaussian_elimination(
    matrix: &AugmentedMatrix,
) -> (Vec<EliminationStep>, Solution) {
    let n = matrix.rows;
    let m = matrix.cols;
    let mut data = matrix.data.clone();
    let vars = matrix.vars.clone();

    let mut steps: Vec<EliminationStep> = Vec::new();

    steps.push(EliminationStep {
        matrix: AugmentedMatrix {
            data: data.clone(),
            vars: vars.clone(),
            rows: n,
            cols: m,
        },
        row_ops: (0..n).map(|_i| RowOp {
            op_type: OpType::NoChange,
            arrow_src: None,
            arrow_dst: None,
        }).collect(),
    });

    let mut pivot_row = 0;

    for col in 0..m {
        if pivot_row >= n {
            break;
        }

        let mut found = None;
        for r in pivot_row..n {
            if !data[r][col].is_zero() {
                found = Some(r);
                break;
            }
        }

        let pivot_col = match found {
            Some(r) => r,
            None => continue,
        };

        if pivot_col != pivot_row {
            data.swap(pivot_col, pivot_row);
            let swap_label = swap_label(pivot_col, pivot_row);
            let ops: Vec<RowOp> = (0..n).map(|i| {
                if i == pivot_row || i == pivot_col {
                    RowOp {
                        op_type: OpType::Swap,
                        arrow_src: Some(swap_label.clone()),
                        arrow_dst: Some(swap_label.clone()),
                    }
                } else {
                    RowOp {
                        op_type: OpType::NoChange,
                        arrow_src: None,
                        arrow_dst: None,
                    }
                }
            }).collect();

            steps.push(EliminationStep {
                matrix: AugmentedMatrix {
                    data: data.clone(),
                    vars: vars.clone(),
                    rows: n,
                    cols: m,
                },
                row_ops: ops,
            });
        }

        let pivot_val = data[pivot_row][col].clone();

        for r in (pivot_row + 1)..n {
            if data[r][col].is_zero() {
                continue;
            }

            let elim_val = data[r][col].clone();

            for j in col..=m {
                let new_val = pivot_val.clone() * data[r][j].clone() - elim_val.clone() * data[pivot_row][j].clone();
                data[r][j] = new_val;
            }

            let pv_str = format_multiplier(&pivot_val);
            let src_str = format_multiplier(&(-elim_val));

            let ops: Vec<RowOp> = (0..n).map(|i| {
                if i == pivot_row {
                    RowOp {
                        op_type: OpType::Eliminate,
                        arrow_src: Some(src_str.clone()),
                        arrow_dst: None,
                    }
                } else if i == r {
                    RowOp {
                        op_type: OpType::Eliminate,
                        arrow_src: None,
                        arrow_dst: Some(pv_str.clone()),
                    }
                } else {
                    RowOp {
                        op_type: OpType::NoChange,
                        arrow_src: None,
                        arrow_dst: None,
                    }
                }
            }).collect();

            steps.push(EliminationStep {
                matrix: AugmentedMatrix {
                    data: data.clone(),
                    vars: vars.clone(),
                    rows: n,
                    cols: m,
                },
                row_ops: ops,
            });
        }

        pivot_row += 1;
    }

    let solution = compute_solution(&data, &vars, n, m);

    (steps, solution)
}

fn compute_solution(data: &[Vec<Rational>], vars: &[String], n: usize, m: usize) -> Solution {
    let mut basic_vars: Vec<Option<usize>> = vec![None; m];
    let mut r = 0;

    for col in 0..m {
        if r >= n { break; }
        if !data[r][col].is_zero() {
            basic_vars[col] = Some(r);
            r += 1;
        }
    }

    for r in 0..n {
        let all_zero = (0..m).all(|c| data[r][c].is_zero());
        if all_zero && !data[r][m].is_zero() {
            return Solution::NoSolution;
        }
    }

    let free_vars: Vec<usize> = (0..m).filter(|c| basic_vars[*c].is_none()).collect();
    let has_all_basic = basic_vars.iter().all(|b| b.is_some());

    if free_vars.is_empty() && has_all_basic {
        let mut solution: BTreeMap<String, Rational> = BTreeMap::new();

        for col in (0..m).rev() {
            if let Some(r) = basic_vars[col] {
                let mut rhs = data[r][m].clone();
                for j in (col + 1)..m {
                    if !data[r][j].is_zero() {
                        if let Some(val) = solution.get(&vars[j]) {
                            rhs = rhs - data[r][j].clone() * val.clone();
                        }
                    }
                }
                if !data[r][col].is_one() {
                    rhs = rhs / data[r][col].clone();
                }
                solution.insert(vars[col].clone(), rhs);
            }
        }

        return Solution::Unique(solution);
    }

    if free_vars.is_empty() {
        return Solution::Unique(BTreeMap::new());
    }

    let free_var_names: Vec<String> = free_vars.iter().map(|i| vars[*i].clone()).collect();

    let param_names: Vec<String> = (0..free_var_names.len()).map(|i| {
        if free_var_names.len() == 1 { "t".to_string() }
        else if free_var_names.len() <= 3 {
            ['t', 's', 'u'][i].to_string()
        } else {
            format!("t{}", i + 1)
        }
    }).collect();

    let mut params: Vec<ParametricVar> = Vec::new();

    // Track algebraic expression for each var (coefficients per free var + constant)
    #[derive(Clone)]
    struct ParamExpr {
        coeffs: Vec<Rational>,
        constant: Rational,
    }
    let num_free = free_vars.len();

    let mut solved_exprs: Vec<Option<ParamExpr>> = vec![None; m];

    // Assign free vars first: var = param (1 * param + 0)
    for (fi, fv) in free_vars.iter().enumerate() {
        let mut coeffs = vec![Rational::zero(); num_free];
        coeffs[fi] = Rational::one();
        solved_exprs[*fv] = Some(ParamExpr { coeffs, constant: Rational::zero() });
    }

    for col in (0..m).rev() {
        if basic_vars[col].is_none() {
            continue; // already handled as free var
        }
        let r = basic_vars[col].unwrap();
        let pivot = data[r][col].clone();
        let mut coeffs = vec![Rational::zero(); num_free];
        let mut constant = data[r][m].clone();

        for j in (col + 1)..m {
            let c = data[r][j].clone();
            if c.is_zero() { continue; }

            if let Some(ref expr) = solved_exprs[j] {
                constant = constant - c.clone() * expr.constant.clone();
                for fi in 0..num_free {
                    coeffs[fi] = coeffs[fi] - c.clone() * expr.coeffs[fi].clone();
                }
            }
        }

        // Divide by pivot
        if !pivot.is_one() {
            for fi in 0..num_free {
                coeffs[fi] = coeffs[fi].clone() / pivot.clone();
            }
            constant = constant / pivot.clone();
        }

        // Format as string
        let mut parts: Vec<String> = Vec::new();
        for (fi, coeff) in coeffs.iter().enumerate() {
            if coeff.is_zero() { continue; }
            let abs_c = coeff.abs();
            let c_str = if abs_c.is_one() { String::new() } else if abs_c.is_integer() { format!("{}", abs_c.num) } else { format!("{}", abs_c) };
            if coeff.is_negative() {
                parts.push(format!("-{}{}", c_str, param_names[fi]));
            } else {
                if parts.is_empty() {
                    parts.push(format!("{}{}", c_str, param_names[fi]));
                } else {
                    parts.push(format!("+ {}{}", c_str, param_names[fi]));
                }
            }
        }

        if !constant.is_zero() {
            let abs_rhs = constant.abs();
            let rhs_str = if abs_rhs.is_integer() { format!("{}", abs_rhs.num) } else { format!("{}", abs_rhs) };
            if constant.is_negative() {
                if parts.is_empty() {
                    parts.push(format!("-{}", rhs_str));
                } else {
                    parts.push(format!("- {}", rhs_str));
                }
            } else {
                if parts.is_empty() {
                    parts.push(rhs_str);
                } else {
                    parts.push(format!("+ {}", rhs_str));
                }
            }
        }

        let expr = if parts.is_empty() { "0".to_string() } else { parts.join(" ") };

        solved_exprs[col] = Some(ParamExpr { coeffs, constant });
        params.push(ParametricVar {
            var: vars[col].clone(),
            expr,
        });
    }

    // Free vars at the bottom
    for (fi, fv) in free_vars.iter().enumerate() {
        params.push(ParametricVar {
            var: vars[*fv].clone(),
            expr: param_names[fi].clone(),
        });
    }

    params.reverse();

    Solution::Parametric(params)
}


