//! Exact command-line parsing for the native package tool.

const DEFAULT_CONTENT: &str = "Welcome to Anodrel.";

#[derive(Debug, PartialEq, Eq)]
pub enum Command {
    Init {
        destination: String,
        application_id: String,
        display_name: String,
        content: String,
    },
    Verify {
        manifest_path: String,
    },
}

#[derive(Debug, PartialEq, Eq)]
pub struct UsageError;

pub fn parse(arguments: impl IntoIterator<Item = String>) -> Result<Command, UsageError> {
    let mut arguments = arguments.into_iter();
    match arguments.next().as_deref() {
        Some("init") => parse_init(arguments),
        Some("verify") => parse_verify(arguments),
        _ => Err(UsageError),
    }
}

fn parse_init(mut arguments: impl Iterator<Item = String>) -> Result<Command, UsageError> {
    let destination = arguments.next().ok_or(UsageError)?;
    let application_id = arguments.next().ok_or(UsageError)?;
    let display_name = arguments.next().ok_or(UsageError)?;
    let content = arguments
        .next()
        .unwrap_or_else(|| DEFAULT_CONTENT.to_owned());
    if arguments.next().is_some() {
        return Err(UsageError);
    }

    Ok(Command::Init {
        destination,
        application_id,
        display_name,
        content,
    })
}

fn parse_verify(mut arguments: impl Iterator<Item = String>) -> Result<Command, UsageError> {
    let manifest_path = arguments.next().ok_or(UsageError)?;
    if arguments.next().is_some() {
        return Err(UsageError);
    }
    Ok(Command::Verify { manifest_path })
}

#[cfg(test)]
mod tests {
    use super::{Command, UsageError, parse};

    #[test]
    fn parses_the_exact_init_and_verify_forms() {
        assert_eq!(
            parse(["init", "out/app", "org.example.app", "Example", "Hello"].map(String::from)),
            Ok(Command::Init {
                destination: "out/app".to_owned(),
                application_id: "org.example.app".to_owned(),
                display_name: "Example".to_owned(),
                content: "Hello".to_owned(),
            })
        );
        assert_eq!(
            parse(["verify", "out/app/anodrel.application.json"].map(String::from)),
            Ok(Command::Verify {
                manifest_path: "out/app/anodrel.application.json".to_owned(),
            })
        );
    }

    #[test]
    fn supplies_only_the_documented_default_and_rejects_extra_arguments() {
        assert!(matches!(
            parse(["init", "out/app", "org.example.app", "Example"].map(String::from)),
            Ok(Command::Init { content, .. }) if content == "Welcome to Anodrel."
        ));
        assert_eq!(
            parse(["verify", "manifest.json", "unexpected"].map(String::from)),
            Err(UsageError)
        );
    }
}
