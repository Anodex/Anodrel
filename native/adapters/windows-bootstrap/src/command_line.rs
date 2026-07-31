//! Exact, non-secret Windows executable and argument handling.

use std::fmt;

#[derive(Clone, Debug)]
pub struct BootstrapCommand {
    program: String,
    arguments: Vec<String>,
}

impl BootstrapCommand {
    pub fn new(program: impl Into<String>) -> Result<Self, CommandError> {
        let program = program.into();
        validate_component(&program)?;
        Ok(Self {
            program,
            arguments: Vec::new(),
        })
    }

    pub fn arg(mut self, argument: impl Into<String>) -> Result<Self, CommandError> {
        let argument = argument.into();
        validate_component(&argument)?;
        self.arguments.push(argument);
        Ok(self)
    }

    pub(crate) fn program(&self) -> &str {
        &self.program
    }

    pub(crate) fn command_line(&self) -> String {
        let mut values = Vec::with_capacity(self.arguments.len() + 1);
        values.push(quote_windows_argument(&self.program));
        values.extend(
            self.arguments
                .iter()
                .map(|argument| quote_windows_argument(argument)),
        );
        values.join(" ")
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommandError {
    EmptyComponent,
    EmbeddedNull,
}

impl fmt::Display for CommandError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyComponent => write!(formatter, "component cannot be empty"),
            Self::EmbeddedNull => write!(formatter, "component contains an embedded null"),
        }
    }
}

impl std::error::Error for CommandError {}

fn validate_component(value: &str) -> Result<(), CommandError> {
    if value.is_empty() {
        return Err(CommandError::EmptyComponent);
    }
    if value.contains('\0') {
        return Err(CommandError::EmbeddedNull);
    }
    Ok(())
}

fn quote_windows_argument(value: &str) -> String {
    if !value.is_empty() && !value.contains([' ', '\t', '"']) {
        return value.to_owned();
    }
    let mut output = String::with_capacity(value.len() + 2);
    output.push('"');
    let mut backslashes = 0;
    for character in value.chars() {
        match character {
            '\\' => backslashes += 1,
            '"' => {
                output.extend(std::iter::repeat_n('\\', backslashes * 2 + 1));
                output.push('"');
                backslashes = 0;
            }
            _ => {
                output.extend(std::iter::repeat_n('\\', backslashes));
                output.push(character);
                backslashes = 0;
            }
        }
    }
    output.extend(std::iter::repeat_n('\\', backslashes * 2));
    output.push('"');
    output
}
