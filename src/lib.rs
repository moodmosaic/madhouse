//! # MADHOUSE
//!
//! Model-based Rust state machine testing.
//!
//! Tests state machines via sequences of command objects. Each command:
//! 1. Checks preconditions via check()
//! 2. Mutates state via apply()
//! 3. Verifies assertions
//!
//! ## Command flow
//!
//! ```text
//!                    +-------+
//!                    | State |
//!                    +-------+
//!                        ^
//!                        |
//!   +---------+     +----+----+     +-----------+
//!   | Command | --> | check() | --> |  apply()  |
//!   +---------+     +---------+     | [asserts] |
//!        ^                          +-----------+
//!        |
//!   +----------+
//!   | Strategy |
//!   +----------+
//! ```
//!
//! ## Modes
//!
//! - **Normal**: Commands run in specified order.
//! - **Random**: Commands chosen pseudorandomly (when MADHOUSE=1).
//!
//! ## Features
//!
//! - Trait-based command design
//! - Self-validating commands
//! - Timing information
//! - Test case shrinking
//!
//! ## Example
//!
//! ```rust
//! use madhouse::{
//!     execute_commands, prop_allof, Command, CommandWrapper, State,
//!     TestContext, scenario
//! };
//! use proptest::prelude::{Just, Strategy};
//! use proptest::strategy::ValueTree;
//! use std::sync::Arc;
//!
//! // Define your State implementation.
//! #[derive(Debug, Default)]
//! struct MyState {
//!     last_mined_block: u64,
//! }
//!
//! // Implement State trait for your state.
//! impl State for MyState {}
//!
//! // Define your TestContext implementation.
//! #[derive(Debug, Default, Clone)]
//! struct MyContext {
//!     parameters: Vec<u32>,
//! }
//!
//! // Implement TestContext trait for your context.
//! impl TestContext for MyContext {}
//!
//! // Define a simple increment command.
//! struct IncrementCommand;
//!
//! impl Command<MyState, MyContext> for IncrementCommand {
//!     fn check(&self, _state: &MyState) -> bool { true }
//!     fn apply(&self, state: &mut MyState) { state.last_mined_block += 1; }
//!     fn label(&self) -> String { "INCREMENT".to_string() }
//!     fn build(_ctx: Arc<MyContext>) -> impl Strategy<Value = CommandWrapper<MyState, MyContext>> {
//!         Just(CommandWrapper::new(IncrementCommand))
//!     }
//! }
//!
//! // Set up test context.
//! let test_context = Arc::new(MyContext::default());
//!
//! // Run the test scenario.
//! scenario! [test_context, IncrementCommand];
//!
//! // Manual execution.
//! let mut state = MyState::default();
//! let commands = vec![CommandWrapper::new(IncrementCommand)];
//! let executed = execute_commands(&commands, &mut state);
//! assert_eq!(state.last_mined_block, 1);
//! ```

use proptest::prelude::Strategy;
use std::fmt::{Debug, Formatter, Result as FmtResult};
use std::sync::Arc;
use std::time::Instant;

/// System state being tested.
///
/// # Examples
///
/// ```
/// use madhouse::State;
///
/// #[derive(Debug, Default)]
/// struct CounterState {
///     value: u64,
///     max_reached: u64,
///     increment_count: u64,
/// }
///
/// impl State for CounterState {}
/// ```
pub trait State: Debug {}

/// Test configuration.
///
/// # Examples
///
/// ```
/// use madhouse::TestContext;
///
/// #[derive(Debug, Clone, Default)]
/// struct CounterContext {
///     max_increment: u64,
///     allowed_operations: Vec<String>,
/// }
///
/// impl TestContext for CounterContext {}
/// ```
pub trait TestContext: Debug + Clone {}

/// Commands in the stateful testing framework.
///
/// Commands handle:
/// - Checking preconditions
/// - Applying state changes
/// - Providing descriptive labels
/// - Building strategies for generation
///
/// # Examples
///
/// ```
/// use madhouse::{Command, CommandWrapper, State, TestContext};
/// use proptest::prelude::*;
/// use std::sync::Arc;
///
/// // Define state and context.
/// #[derive(Debug, Default)]
/// struct CounterState {
///     count: u64,
///     max_value: u64,
/// }
/// impl State for CounterState {}
///
/// #[derive(Debug, Clone, Default)]
/// struct CounterContext {
///     increment_sizes: Vec<u64>,
/// }
/// impl TestContext for CounterContext {}
///
/// // Define a command to increment the counter.
/// struct IncrementCommand {
///     amount: u64,
/// }
///
/// impl Command<CounterState, CounterContext> for IncrementCommand {
///     // Check if we can apply this command.
///     fn check(&self, state: &CounterState) -> bool {
///         state.count + self.amount <= state.max_value
///     }
///
///     // Apply the command to the state.
///     fn apply(&self, state: &mut CounterState) {
///         state.count += self.amount;
///     }
///
///     // Provide a descriptive label.
///     fn label(&self) -> String {
///         format!("INCREMENT({})", self.amount)
///     }
///
///     // Build a strategy for generating instances.
///     fn build(ctx: Arc<CounterContext>) -> impl Strategy<Value = CommandWrapper<CounterState, CounterContext>> {
///         let increments = ctx.increment_sizes.clone();
///         (0..increments.len()).prop_map(move |idx| {
///             let amount = increments.get(idx).cloned().unwrap_or(1);
///             CommandWrapper::new(IncrementCommand { amount })
///         })
///     }
/// }
/// ```
pub trait Command<S: State, C: TestContext> {
    /// Checks if the command can be applied to the current state.
    ///
    /// # Arguments
    /// * `state` - Current state to check against.
    fn check(&self, state: &S) -> bool;

    /// Applies the command to the state, modifying it.
    ///
    /// # Arguments
    /// * `state` - State to modify.
    fn apply(&self, state: &mut S);

    /// Returns a human-readable label for the command.
    fn label(&self) -> String;

    /// Builds a proptest strategy for generating instances of this command.
    ///
    /// # Arguments
    /// * `ctx` - Test context used to parameterize command generation.
    fn build(ctx: Arc<C>) -> impl Strategy<Value = CommandWrapper<S, C>>
    where
        Self: Sized;
}

/// Wrapper for command trait objects.
/// Allows commands to be stored in collections while preserving concrete types.
///
/// # Examples
///
/// ```
/// use madhouse::{Command, CommandWrapper, State};
/// use proptest::prelude::*;
/// use std::sync::Arc;
///
/// // Define your state.
/// #[derive(Debug, Default)]
/// struct MyState { counter: u64 }
/// impl State for MyState {}
///
/// // Define your context.
/// #[derive(Debug, Clone, Default)]
/// struct MyContext {}
/// impl madhouse::TestContext for MyContext {}
///
/// // Define your command.
/// struct IncrementCmd;
/// impl Command<MyState, MyContext> for IncrementCmd {
///     fn check(&self, _state: &MyState) -> bool { true }
///     fn apply(&self, state: &mut MyState) { state.counter += 1; }
///     fn label(&self) -> String { "INCREMENT".to_string() }
///     fn build(_ctx: Arc<MyContext>) -> impl Strategy<Value = CommandWrapper<MyState, MyContext>> {
///         Just(CommandWrapper::new(IncrementCmd))
///     }
/// }
///
/// // Create and use the wrapper.
/// let cmd = IncrementCmd;
/// let wrapper = CommandWrapper::new(cmd);
/// assert_eq!(wrapper.command.label(), "INCREMENT");
/// ```
pub struct CommandWrapper<S: State, C: TestContext> {
    /// The wrapped command trait object.
    pub command: Arc<dyn Command<S, C>>,
}

impl<S: State, C: TestContext> CommandWrapper<S, C> {
    /// Creates a new command wrapper for the given command.
    ///
    /// # Arguments
    ///
    /// * `cmd` - The command to wrap.
    pub fn new<Cmd: Command<S, C> + 'static>(cmd: Cmd) -> Self {
        Self {
            command: Arc::new(cmd),
        }
    }
}

impl<S: State, C: TestContext> Clone for CommandWrapper<S, C> {
    fn clone(&self) -> Self {
        Self {
            command: Arc::clone(&self.command),
        }
    }
}

impl<S: State, C: TestContext> Debug for CommandWrapper<S, C> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}", self.command.label())
    }
}

/// Creates a strategy that returns a Vec containing values from all provided strategies.
///
/// # Examples
///
/// ```
/// use madhouse::prop_allof;
/// use proptest::prelude::*;
///
/// // Create strategies.
/// let strat1 = Just(1);
/// let strat2 = Just(2);
/// let strat3 = Just(3);
///
/// // Combine them with prop_allof.
/// let combined = prop_allof![strat1, strat2, strat3];
///
/// // The strategy will produce a vec with all values: [1, 2, 3].
/// proptest!(|(v in combined)| {
///     assert_eq!(v, vec![1, 2, 3]);
/// });
/// ```
#[macro_export]
macro_rules! prop_allof {
    ($strat:expr $(,)?) => {
        $strat.prop_map(|val| vec![val])
    };

    ($first:expr, $($rest:expr),+ $(,)?) => {
        {
            let first_strat = $first.prop_map(|val| vec![val]);
            let rest_strat = prop_allof!($($rest),+);

            (first_strat, rest_strat).prop_map(|(mut first_vec, rest_vec)| {
                first_vec.extend(rest_vec);
                first_vec
            })
        }
    };
}

/// Executes a sequence of commands and returns those executed.
///
/// This function:
/// 1. Filters commands based on check() method.
/// 2. Applies each valid command to the state.
/// 3. Measures execution time.
/// 4. Prints a summary of selected and executed commands.
///
/// # Arguments
/// * `commands` - Slice of commands to potentially execute.
/// * `state` - Mutable state that commands will modify.
///
/// # Returns
/// A vector of references to commands that were executed.
///
/// # Examples
///
/// ```
/// use madhouse::{Command, CommandWrapper, State, TestContext, execute_commands};
/// use proptest::prelude::*;
/// use std::sync::Arc;
///
/// // Define state and context.
/// #[derive(Debug, Default)]
/// struct CounterState {
///     value: u64,
/// }
/// impl State for CounterState {}
///
/// #[derive(Debug, Clone, Default)]
/// struct CounterContext {}
/// impl TestContext for CounterContext {}
///
/// // Define a simple command.
/// struct IncrementCommand(u64);
///
/// impl Command<CounterState, CounterContext> for IncrementCommand {
///     fn check(&self, _state: &CounterState) -> bool { true }
///     fn apply(&self, state: &mut CounterState) { state.value += self.0; }
///     fn label(&self) -> String { format!("INCREMENT({})", self.0) }
///     fn build(_ctx: Arc<CounterContext>) ->
///         impl Strategy<Value = CommandWrapper<CounterState, CounterContext>> {
///         Just(CommandWrapper::new(IncrementCommand(1)))
///     }
/// }
///
/// // Execute commands.
/// let mut state = CounterState::default();
/// let commands = vec![
///     CommandWrapper::new(IncrementCommand(3)),
///     CommandWrapper::new(IncrementCommand(5)),
/// ];
///
/// let executed = execute_commands(&commands, &mut state);
/// assert_eq!(executed.len(), 2);
/// assert_eq!(state.value, 8);
/// ```
pub fn execute_commands<'a, S: State, C: TestContext>(
    commands: &'a [CommandWrapper<S, C>],
    state: &mut S,
) -> Vec<&'a CommandWrapper<S, C>> {
    let mut executed = Vec::with_capacity(commands.len());
    let mut execution_times = Vec::with_capacity(commands.len());

    // ANSI color codes.
    let yellow = "\x1b[33m";
    let green = "\x1b[32m";
    let reset = "\x1b[0m";

    for cmd in commands {
        if cmd.command.check(state) {
            let start = Instant::now();
            cmd.command.apply(state);
            let duration = start.elapsed();
            executed.push(cmd);
            execution_times.push(duration);
        }
    }

    println!("Selected:");
    for (i, cmd) in commands.iter().enumerate() {
        println!("{:02}. {}{}{}", i + 1, yellow, cmd.command.label(), reset);
    }

    println!("Executed:");
    for (i, (cmd, time)) in executed.iter().zip(execution_times.iter()).enumerate() {
        println!(
            "{:02}. {}{}{} ({:.2?})",
            i + 1,
            green,
            cmd.command.label(),
            reset,
            time
        );
    }

    executed
}

/// Macro for running stateful tests.
///
/// While commands execute in the order specified in the macro,
/// their generation depends on several factors:
///
/// - In normal mode: Commands run in the order provided, but proptest strategies
///   may generate different values across runs unless the same seed is used.
/// - With MADHOUSE=1: The command sequence is permutated randomly.
/// - With PROPTEST_MAX_SHRINK_ITERS: Failed tests can be shrunk to minimal cases.
///
/// The framework honors any PROPTEST environment variables. By default,
/// the scenario runs with 1 test case and 0 shrink iterations to accommodate
/// heavyweight non-deterministic test setups found in complex systems.
///
/// # Arguments
///
/// * `test_context` - Test context for creating commands.
/// * `command1, command2, ...` - Either command types (e.g., `Inc`) or
///   fixed command instances (e.g., `(Inc { amount: 3 })`). Note that
///   expressions must be wrapped in parentheses.
///
/// # Examples
///
/// ```
/// use madhouse::{
///     execute_commands, prop_allof, Command, CommandWrapper, State,
///     TestContext, scenario
/// };
/// use proptest::prelude::Just;
/// use proptest::strategy::Strategy;
/// use std::sync::Arc;
///
/// // Define your application state.
/// #[derive(Debug, Default)]
/// struct AppState {
///     counter: u64,
/// }
/// impl State for AppState {}
///
/// // Define your test context.
/// #[derive(Debug, Clone, Default)]
/// struct AppContext {}
/// impl TestContext for AppContext {}
///
/// // Define some commands.
/// struct IncrementCommand {
///     amount: u64,
/// }
/// impl Command<AppState, AppContext> for IncrementCommand {
///     fn check(&self, _state: &AppState) -> bool { true }
///     fn apply(&self, state: &mut AppState) { state.counter += self.amount; }
///     fn label(&self) -> String { format!("INCREMENT({})", self.amount) }
///     fn build(_ctx: Arc<AppContext>) -> impl Strategy<Value = CommandWrapper<AppState, AppContext>> {
///         (1..=5u64).prop_map(|n| CommandWrapper::new(IncrementCommand { amount: n }))
///     }
/// }
///
/// struct ResetCommand;
/// impl Command<AppState, AppContext> for ResetCommand {
///     fn check(&self, state: &AppState) -> bool { state.counter > 0 }
///     fn apply(&self, state: &mut AppState) { state.counter = 0; }
///     fn label(&self) -> String { "RESET".to_string() }
///     fn build(_ctx: Arc<AppContext>) -> impl Strategy<Value = CommandWrapper<AppState, AppContext>> {
///         Just(CommandWrapper::new(ResetCommand))
///     }
/// }
///
/// // Run the test.
/// let ctx = Arc::new(AppContext::default());
/// scenario![
///     ctx,
///     IncrementCommand,
///     ResetCommand,
///     (IncrementCommand { amount: 42 })
/// ];
/// ```
#[macro_export]
macro_rules! scenario {
    ($test_context:expr, $($cmd:tt),+ $(,)?) => {
        {
            let test_context = $test_context.clone();
            let config = proptest::test_runner::Config {
                cases: 1,
                max_shrink_iters: 0,
                ..Default::default()
            };

            // Use MADHOUSE env var to determine test mode.
            let use_madhouse = std::env::var("MADHOUSE") == Ok("1".into());

            if use_madhouse {
                proptest::proptest!(config, |(commands in proptest::collection::vec(
                    proptest::prop_oneof![
                        $(scenario!(@to_strategy test_context.clone(), $cmd)),+
                    ],
                    1..16,
                ))| {
                    println!("\n=== New Test Run (MADHOUSE mode) ===\n");
                    let mut state = <_ as std::default::Default>::default();
                    execute_commands(&commands, &mut state);
                });
            } else {
                proptest::proptest!(config, |(commands in prop_allof![
                    $(scenario!(@to_strategy test_context.clone(), $cmd)),+
                ])| {
                    println!("\n=== New Test Run (deterministic mode) ===\n");
                    let mut state = <_ as std::default::Default>::default();
                    execute_commands(&commands, &mut state);
                });
            }
        }
    };

    (@to_strategy $ctx:expr, $cmd:ident) => {
        $cmd::build($ctx)
    };

    (@to_strategy $ctx:expr, $cmd:expr) => {
        proptest::prelude::Just(CommandWrapper::new($cmd))
    };
}

/// Common imports for working with madhouse scenarios.
///
/// Import everything needed for scenario testing with a single use statement:
/// ```
/// use madhouse::prelude::*;
/// ```
pub mod prelude {
    pub use crate::{
        execute_commands, prop_allof, scenario, Command, CommandWrapper, State, TestContext,
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::Just;

    #[derive(Clone, Debug, Default)]
    struct MyState {
        last_mined_block: u64,
    }

    impl State for MyState {}

    #[derive(Debug, Clone, Default)]
    struct MyContext {}

    impl TestContext for MyContext {}

    struct TestCommand {
        value: u32,
    }

    impl Command<MyState, MyContext> for TestCommand {
        fn check(&self, _state: &MyState) -> bool {
            true
        }

        fn apply(&self, state: &mut MyState) {
            state.last_mined_block += self.value as u64;
        }

        fn label(&self) -> String {
            format!("TEST({})", self.value)
        }

        fn build(
            _ctx: Arc<MyContext>,
        ) -> impl Strategy<Value = CommandWrapper<MyState, MyContext>> {
            Just(CommandWrapper::new(TestCommand { value: 1 }))
        }
    }

    #[test]
    fn test_command_wrapper() {
        let cmd = TestCommand { value: 42 };
        let wrapper = CommandWrapper::new(cmd);
        let mut state = MyState::default();
        assert!(wrapper.command.check(&state));

        wrapper.command.apply(&mut state);

        assert_eq!(state.last_mined_block, 42);
        assert_eq!(format!("{:?}", wrapper), "TEST(42)");
    }

    #[test]
    fn test_prop_allof_macro() {
        use proptest::prelude::*;

        // Test with one strategy.
        let strat1 = Just(1);
        let combined1 = prop_allof![strat1];

        proptest!(|(v in combined1)| {
            assert_eq!(v, vec![1]);
        });

        // Test with multiple strategies.
        let strat2 = Just(2);
        let strat3 = Just(3);
        let combined2 = prop_allof![strat1, strat2, strat3];

        proptest!(|(v in combined2)| {
            assert_eq!(v, vec![1, 2, 3]);
        });
    }

    #[test]
    fn test_execute_commands_empty() {
        let commands: Vec<CommandWrapper<MyState, MyContext>> = vec![];
        let mut state = MyState::default();

        let executed = execute_commands(&commands, &mut state);
        assert!(executed.is_empty());
    }

    #[test]
    fn test_execute_commands_all_rejected() {
        struct RejectCommand;

        impl Command<MyState, MyContext> for RejectCommand {
            fn check(&self, _state: &MyState) -> bool {
                false
            }
            fn apply(&self, _state: &mut MyState) {}
            fn label(&self) -> String {
                "REJECT".to_string()
            }
            fn build(
                _ctx: Arc<MyContext>,
            ) -> impl Strategy<Value = CommandWrapper<MyState, MyContext>> {
                Just(CommandWrapper::new(RejectCommand))
            }
        }

        let commands = vec![
            CommandWrapper::new(RejectCommand),
            CommandWrapper::new(RejectCommand),
        ];
        let mut state = MyState::default();

        let executed = execute_commands(&commands, &mut state);
        assert!(executed.is_empty());
    }
}

#[cfg(test)]
mod scenario_tests {
    use super::*;
    use proptest::prelude::Just;
    use std::sync::Arc;

    #[derive(Debug, Default, Clone)]
    struct MyState {
        action_chronicle: Vec<String>,
    }

    impl State for MyState {}

    #[derive(Debug, Clone, Default)]
    struct MyContext {}

    impl TestContext for MyContext {}

    macro_rules! my_command {
        ($name:ident, $label:expr) => {
            struct $name;
            impl Command<MyState, MyContext> for $name {
                fn check(&self, _state: &MyState) -> bool {
                    true
                }
                fn apply(&self, state: &mut MyState) {
                    state.action_chronicle.push($label.to_string());
                }
                fn label(&self) -> String {
                    $label.to_string()
                }
                fn build(
                    _ctx: Arc<MyContext>,
                ) -> impl Strategy<Value = CommandWrapper<MyState, MyContext>> {
                    Just(CommandWrapper::new($name))
                }
            }
        };
    }

    my_command!(A, "A");
    my_command!(B, "B");
    my_command!(C, "C");
    my_command!(D, "D");
    my_command!(E, "E");
    my_command!(F, "F");

    #[test]
    fn run_scenario() {
        let ctx = Arc::new(MyContext::default());
        scenario![ctx, A, B, C, D, E, F];
    }
}

#[cfg(test)]
mod shrinking_scenario_tests {
    use super::*;
    use std::sync::Arc;

    #[derive(Debug, Default)]
    struct CounterState {
        value: u32,
    }

    impl State for CounterState {}

    #[derive(Debug, Clone, Default)]
    struct CounterContext {}

    impl TestContext for CounterContext {}

    struct IncrementCommand {
        amount: u32,
    }

    impl Command<CounterState, CounterContext> for IncrementCommand {
        fn check(&self, _state: &CounterState) -> bool {
            true
        }

        fn apply(&self, state: &mut CounterState) {
            state.value += self.amount;

            assert!(
                state.value <= 100,
                "Counter value exceeded maximum allowed: {}",
                state.value
            );
        }

        fn label(&self) -> String {
            format!("INCREMENT({})", self.amount)
        }

        fn build(
            _ctx: Arc<CounterContext>,
        ) -> impl Strategy<Value = CommandWrapper<CounterState, CounterContext>> {
            // Generate increment amounts between 1 and 50.
            (1..=50u32).prop_map(|amount| CommandWrapper::new(IncrementCommand { amount }))
        }
    }

    // Command that adds a smaller amount to demonstrate shrinking better.
    struct SmallIncrementCommand {
        amount: u32,
    }

    impl Command<CounterState, CounterContext> for SmallIncrementCommand {
        fn check(&self, _state: &CounterState) -> bool {
            true
        }

        fn apply(&self, state: &mut CounterState) {
            state.value += self.amount;
            assert!(
                state.value <= 100,
                "Counter value exceeded maximum allowed: {}",
                state.value
            );
        }

        fn label(&self) -> String {
            format!("SMALL_INCREMENT({})", self.amount)
        }

        fn build(
            _ctx: Arc<CounterContext>,
        ) -> impl Strategy<Value = CommandWrapper<CounterState, CounterContext>> {
            (1..=10u32).prop_map(|amount| CommandWrapper::new(SmallIncrementCommand { amount }))
        }
    }

    // To see shrinking in action, run:
    // MADHOUSE=1 PROPTEST_MAX_SHRINK_ITERS=50 \
    // cargo test demonstrate_shrinking -- --nocapture
    #[test]
    fn demonstrate_shrinking() {
        let ctx = Arc::new(CounterContext::default());
        scenario![ctx, IncrementCommand, SmallIncrementCommand];
    }

    #[test]
    fn demonstrate_shrinking_with_concrete_command() {
        let ctx = Arc::new(CounterContext::default());
        scenario![
            ctx,
            IncrementCommand,
            SmallIncrementCommand,
            (IncrementCommand { amount: 42 })
        ];
    }
}
