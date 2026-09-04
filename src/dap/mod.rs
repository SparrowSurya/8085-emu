//! Intel 8085 Debug Adapter Protocol (DAP 1.6+) implementation.

pub mod breakpoints;
pub mod eval;
pub mod inspect;
pub mod protocol;
pub mod server;
pub mod session;
pub mod sourcemap;

pub use breakpoints::{ActiveBreakpoint, BreakpointHit, BreakpointManager};
pub use eval::{eval_term, evaluate_expression, EvalResult};
pub use inspect::{get_scopes, get_variables, set_variable};
pub use protocol::*;
pub use server::DapServer;
pub use session::{DebugSession, MachineSnapshot, ShadowFrame, StopReason};
pub use sourcemap::{SourceLocation, SourceMap, VariableSymbol};

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_dap_session_launch_and_step() {
        let mut session = DebugSession::new();
        let demo_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("programs/demo.e8085");
        let args = LaunchRequestArguments {
            program: demo_path.to_string_lossy().to_string(),
            stop_on_entry: Some(true),
            libraries: None,
            t_state_limit: None,
            step_mode: None,
            console: Some("internalConsole".to_string()),
            terminal_port: None,
        };

        let res = session.launch(&args);
        assert!(res.is_ok());
        assert_eq!(session.stop_reason, Some(StopReason::Entry));

        // Step in
        let step_res = session.step_in();
        assert!(step_res.is_ok());
        assert_eq!(session.instruction_count, 1);
        assert!(session.elapsed_t_states > 0);

        // Check scopes
        let scopes = session.get_scopes(0);
        assert_eq!(scopes.len(), 5);

        // Check variables
        let regs = session.get_variables(1000);
        assert!(regs.iter().any(|v| v.name == "A"));
        assert!(regs.iter().any(|v| v.name == "PC"));

        let flags = session.get_variables(3000);
        assert!(flags.iter().any(|v| v.name == "Flags Byte (PSW)"));
        assert!(flags.iter().any(|v| v.name == "Zero (Z)"));

        // Evaluate expression
        let eval_res = session.evaluate("A == 0");
        assert!(eval_res.is_ok());
    }

    #[test]
    fn test_dap_breakpoints_and_continue() {
        let mut session = DebugSession::new();
        let demo_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("programs/directives.e8085");
        let args = LaunchRequestArguments {
            program: demo_path.to_string_lossy().to_string(),
            stop_on_entry: Some(true),
            libraries: None,
            t_state_limit: None,
            step_mode: None,
            console: Some("internalConsole".to_string()),
            terminal_port: None,
        };

        session.launch(&args).unwrap();

        // Set line breakpoint
        let bps = session.set_line_breakpoints(
            &demo_path,
            &[SourceBreakpoint {
                line: 12,
                column: None,
                condition: None,
                hit_condition: None,
                log_message: None,
            }],
        );
        assert_eq!(bps.len(), 1);
        assert!(bps[0].verified);

        // Continue until breakpoint or halt
        let cont_res = session.continue_exec();
        assert!(cont_res.is_ok());
    }
}
