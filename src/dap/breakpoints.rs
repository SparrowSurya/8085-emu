//! Breakpoint and watchpoint management for 8085 debugging.

use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};

use crate::dap::eval::evaluate_expression;
use crate::dap::protocol::{Breakpoint, FunctionBreakpoint, Source, SourceBreakpoint};
use crate::dap::sourcemap::SourceMap;
use crate::machine::Machine;

#[derive(Debug, Clone)]
pub struct ActiveBreakpoint {
    pub id: i64,
    pub address: u16,
    pub source_path: PathBuf,
    pub line: usize,
    pub condition: Option<String>,
    pub hit_condition: Option<String>,
    pub hit_count: usize,
    pub is_temp: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BreakpointHit {
    pub id: i64,
    pub address: u16,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct BreakpointManager {
    next_id: i64,
    // File -> (Line -> ActiveBreakpoint)
    file_breakpoints: BTreeMap<PathBuf, BTreeMap<usize, ActiveBreakpoint>>,
    // Function name -> ActiveBreakpoint
    func_breakpoints: BTreeMap<String, ActiveBreakpoint>,
    // Set of addresses with temporary run-to breakpoints
    temp_breakpoints: HashSet<u16>,
}

impl BreakpointManager {
    pub fn new() -> Self {
        Self {
            next_id: 1,
            file_breakpoints: BTreeMap::new(),
            func_breakpoints: BTreeMap::new(),
            temp_breakpoints: HashSet::new(),
        }
    }

    pub fn set_line_breakpoints(
        &mut self,
        file: &Path,
        requested: &[SourceBreakpoint],
        source_map: &SourceMap,
    ) -> Vec<Breakpoint> {
        let file_buf = file.to_path_buf();
        let mut new_map = BTreeMap::new();
        let mut response_bps = Vec::new();

        for req in requested {
            let id = self.next_id;
            self.next_id += 1;

            if let Some((actual_line, addr)) = source_map.find_nearest_executable_line(file, req.line) {
                let active = ActiveBreakpoint {
                    id,
                    address: addr,
                    source_path: file_buf.clone(),
                    line: actual_line,
                    condition: req.condition.clone().filter(|s| !s.trim().is_empty()),
                    hit_condition: req.hit_condition.clone().filter(|s| !s.trim().is_empty()),
                    hit_count: 0,
                    is_temp: false,
                };
                new_map.insert(actual_line, active);

                response_bps.push(Breakpoint {
                    id: Some(id),
                    verified: true,
                    message: None,
                    source: Some(Source {
                        name: file.file_name().map(|n| n.to_string_lossy().to_string()),
                        path: Some(file.to_string_lossy().to_string()),
                    }),
                    line: Some(actual_line),
                    column: Some(1),
                    instruction_reference: Some(format!("0x{addr:04X}")),
                });
            } else {
                response_bps.push(Breakpoint {
                    id: Some(id),
                    verified: false,
                    message: Some("no executable code at requested line".to_string()),
                    source: Some(Source {
                        name: file.file_name().map(|n| n.to_string_lossy().to_string()),
                        path: Some(file.to_string_lossy().to_string()),
                    }),
                    line: Some(req.line),
                    column: None,
                    instruction_reference: None,
                });
            }
        }

        self.file_breakpoints.insert(file_buf, new_map);
        response_bps
    }

    pub fn set_function_breakpoints(
        &mut self,
        requested: &[FunctionBreakpoint],
        source_map: &SourceMap,
    ) -> Vec<Breakpoint> {
        self.func_breakpoints.clear();
        let mut response_bps = Vec::new();

        for req in requested {
            let id = self.next_id;
            self.next_id += 1;

            if let Some(addr) = source_map.symbol_to_address(&req.name) {
                let loc = source_map.address_to_location(addr);
                let (source, line) = if let Some(l) = loc {
                    (
                        Some(Source {
                            name: l.file_path.file_name().map(|n| n.to_string_lossy().to_string()),
                            path: Some(l.file_path.to_string_lossy().to_string()),
                        }),
                        Some(l.line),
                    )
                } else {
                    (None, None)
                };

                let active = ActiveBreakpoint {
                    id,
                    address: addr,
                    source_path: loc.map(|l| l.file_path.clone()).unwrap_or_default(),
                    line: line.unwrap_or(0),
                    condition: req.condition.clone().filter(|s| !s.trim().is_empty()),
                    hit_condition: req.hit_condition.clone().filter(|s| !s.trim().is_empty()),
                    hit_count: 0,
                    is_temp: false,
                };
                self.func_breakpoints.insert(req.name.clone(), active);

                response_bps.push(Breakpoint {
                    id: Some(id),
                    verified: true,
                    message: None,
                    source,
                    line,
                    column: Some(1),
                    instruction_reference: Some(format!("0x{addr:04X}")),
                });
            } else {
                response_bps.push(Breakpoint {
                    id: Some(id),
                    verified: false,
                    message: Some(format!("symbol '{}' not found", req.name)),
                    source: None,
                    line: None,
                    column: None,
                    instruction_reference: None,
                });
            }
        }

        response_bps
    }

    pub fn set_temp_breakpoint(&mut self, addr: u16) {
        self.temp_breakpoints.insert(addr);
    }

    pub fn clear_temp_breakpoints(&mut self) {
        self.temp_breakpoints.clear();
    }

    pub fn check_hit(
        &mut self,
        current_pc: u16,
        machine: &Machine,
        source_map: &SourceMap,
    ) -> Option<BreakpointHit> {
        // 1. Check temporary run-to breakpoints (Step Over / Step Out)
        if self.temp_breakpoints.contains(&current_pc) {
            self.temp_breakpoints.remove(&current_pc);
            return Some(BreakpointHit {
                id: 0,
                address: current_pc,
                message: Some("step target reached".to_string()),
            });
        }

        // 2. Check line breakpoints
        for file_bps in self.file_breakpoints.values_mut() {
            for bp in file_bps.values_mut() {
                if bp.address == current_pc {
                    bp.hit_count += 1;
                    if Self::evaluate_breakpoint_conditions(bp, machine, source_map) {
                        return Some(BreakpointHit {
                            id: bp.id,
                            address: current_pc,
                            message: None,
                        });
                    }
                }
            }
        }

        // 3. Check function breakpoints
        for bp in self.func_breakpoints.values_mut() {
            if bp.address == current_pc {
                bp.hit_count += 1;
                if Self::evaluate_breakpoint_conditions(bp, machine, source_map) {
                    return Some(BreakpointHit {
                        id: bp.id,
                        address: current_pc,
                        message: None,
                    });
                }
            }
        }

        None
    }

    fn evaluate_breakpoint_conditions(
        bp: &ActiveBreakpoint,
        machine: &Machine,
        source_map: &SourceMap,
    ) -> bool {
        // Check hit condition if present (e.g. `5`, `>= 10`, `% 2 == 0`)
        if let Some(ref hit_expr) = bp.hit_condition {
            let trimmed = hit_expr.trim();
            if let Ok(target_count) = trimmed.parse::<usize>() {
                if bp.hit_count < target_count {
                    return false;
                }
            } else if let Some(num_str) = trimmed.strip_prefix(">=") {
                if let Ok(target) = num_str.trim().parse::<usize>() {
                    if bp.hit_count < target {
                        return false;
                    }
                }
            }
        }

        // Check condition expression if present (e.g. `A == 0x0A`, `Z == 1`)
        if let Some(ref cond_expr) = bp.condition {
            match evaluate_expression(cond_expr, machine, Some(source_map)) {
                Ok(res) => {
                    if res.raw_value.unwrap_or(0) == 0 {
                        return false;
                    }
                }
                Err(_) => return false,
            }
        }

        true
    }
}
