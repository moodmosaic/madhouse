use std::fmt::{Debug, Formatter, Result as FmtResult};
use std::process::Command as SysCommand;
use std::sync::Arc;

use proptest::prelude::Just;
use proptest::prop_oneof;
use proptest::proptest;

fn main() {
    println!("Hello, world!");
}

pub struct ExampleState {
    value: i32,
}

impl ExampleState {
    pub fn new() -> Self {
        Self { value: 0 }
    }

    pub fn increment(&mut self) {
        self.value += 1;
    }

    pub fn decrement(&mut self) {
        self.value -= 1;
    }

    pub fn get_value(&self) -> i32 {
        self.value
    }
}

/// A trait that all commands must implement.
pub trait Command {
    fn check(&self, state: &ExampleState) -> bool;
    fn apply(&self, state: &mut ExampleState);
    fn label(&self) -> &'static str;
}

/// Increment command.
#[derive(Debug, Clone)]
pub struct IncrementCommand;

impl Command for IncrementCommand {
    fn check(&self, _state: &ExampleState) -> bool {
        true // Always allowed.
    }

    fn apply(&self, state: &mut ExampleState) {
        state.increment();
    }

    fn label(&self) -> &'static str {
        "A"
    }
}

/// Decrement command.
#[derive(Debug, Clone)]
pub struct DecrementCommand;

impl Command for DecrementCommand {
    fn check(&self, state: &ExampleState) -> bool {
        state.get_value() > 0 // Prevents negative values.
    }

    fn apply(&self, state: &mut ExampleState) {
        state.decrement();
    }

    fn label(&self) -> &'static str {
        "B"
    }
}

/// Command that spawns an external process.
#[derive(Debug, Clone)]
pub struct ShellProcCommand;

impl Command for ShellProcCommand {
    fn check(&self, _state: &ExampleState) -> bool {
        true // Always allowed.
    }

    fn apply(&self, _state: &mut ExampleState) {
        let output = SysCommand::new("echo")
            .arg("Hello, world!")
            .output()
            .expect("Failed to execute process");

        println!("{}", String::from_utf8_lossy(&output.stdout).trim_end());
    }

    fn label(&self) -> &'static str {
        "C"
    }
}

/// Wrapper to make `dyn Command` clonable and debuggable.
#[derive(Clone)]
struct CommandWrapper {
    command: Arc<dyn Command>,
}

impl CommandWrapper {
    fn new<C: Command + 'static>(cmd: C) -> Self {
        Self {
            command: Arc::new(cmd),
        }
    }
}

// Manually implement Debug for `CommandWrapper`.
impl Debug for CommandWrapper {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}", self.command.label()) // Print command label.
    }
}

proptest! {
  #[test]
  fn stateful_test(
      commands in proptest::collection::vec(
          prop_oneof![
              Just(CommandWrapper::new(IncrementCommand)),
              Just(CommandWrapper::new(DecrementCommand)),
              Just(CommandWrapper::new(ShellProcCommand)),
          ],
          1..10, // Change to something higher like 70.
      )
  ) {
      println!("\n=== New Test Run ===\n");
      let mut state = ExampleState::new();
      for cmd in &commands {
          if cmd.command.check(&state) {
              cmd.command.apply(&mut state);
          }
      }
      println!("Executed commands: {:?}", commands);
        // Fail only if the state gets too high.
        // This condition may be hit only rarely.
        assert!(state.get_value() < 10,
                "State too high: {}", state.get_value());
  }
}
