//! Engine option handling.

use crate::game_state::GameMode;

/// Engine configuration set via `setoption` commands.
#[derive(Debug)]
pub struct EngineOptions {
    /// Game mode (currently only FreeForAll).
    pub game_mode: GameMode,
    /// Beam width for search (placeholder for Stage 7).
    pub beam_width: u32,
    /// Maximum rounds before auto-stop. None = unlimited.
    pub max_rounds: Option<u32>,
}

impl Default for EngineOptions {
    fn default() -> Self {
        Self {
            game_mode: GameMode::FreeForAll,
            beam_width: 15,
            max_rounds: None,
        }
    }
}

/// Result of applying a setoption command.
pub enum SetOptionResult {
    /// Option applied successfully.
    Ok,
    /// LogFile path to enable (handled by Protocol).
    EnableLogFile(String),
    /// Disable LogFile (handled by Protocol).
    DisableLogFile,
    /// Unknown option name.
    UnknownOption(String),
    /// Invalid value for known option.
    InvalidValue(String),
}

/// Apply a setoption command. Returns the result for Protocol to act on.
pub fn apply_option(options: &mut EngineOptions, name: &str, value: &str) -> SetOptionResult {
    match name {
        "GameMode" => match value {
            "FreeForAll" => {
                options.game_mode = GameMode::FreeForAll;
                SetOptionResult::Ok
            }
            _ => SetOptionResult::InvalidValue(format!(
                "unknown GameMode '{value}' (available: FreeForAll)"
            )),
        },
        "BeamWidth" => match value.parse::<u32>() {
            Ok(w) if w > 0 => {
                options.beam_width = w;
                SetOptionResult::Ok
            }
            _ => SetOptionResult::InvalidValue(format!(
                "BeamWidth must be a positive integer, got '{value}'"
            )),
        },
        "MaxRounds" => match value.parse::<u32>() {
            Ok(0) => {
                options.max_rounds = None;
                SetOptionResult::Ok
            }
            Ok(n) => {
                options.max_rounds = Some(n);
                SetOptionResult::Ok
            }
            Err(_) => SetOptionResult::InvalidValue(format!(
                "MaxRounds must be a non-negative integer, got '{value}'"
            )),
        },
        "LogFile" => {
            if value == "none" {
                SetOptionResult::DisableLogFile
            } else {
                SetOptionResult::EnableLogFile(value.to_string())
            }
        }
        _ => SetOptionResult::UnknownOption(name.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_options() {
        let opts = EngineOptions::default();
        assert!(matches!(opts.game_mode, GameMode::FreeForAll));
        assert_eq!(opts.beam_width, 15);
        assert!(opts.max_rounds.is_none());
    }

    #[test]
    fn test_set_beam_width() {
        let mut opts = EngineOptions::default();
        assert!(matches!(
            apply_option(&mut opts, "BeamWidth", "32"),
            SetOptionResult::Ok
        ));
        assert_eq!(opts.beam_width, 32);
    }

    #[test]
    fn test_set_beam_width_invalid() {
        let mut opts = EngineOptions::default();
        assert!(matches!(
            apply_option(&mut opts, "BeamWidth", "abc"),
            SetOptionResult::InvalidValue(_)
        ));
    }

    #[test]
    fn test_set_max_rounds() {
        let mut opts = EngineOptions::default();
        assert!(matches!(
            apply_option(&mut opts, "MaxRounds", "50"),
            SetOptionResult::Ok
        ));
        assert_eq!(opts.max_rounds, Some(50));
    }

    #[test]
    fn test_set_max_rounds_zero_disables() {
        let mut opts = EngineOptions::default();
        opts.max_rounds = Some(10);
        assert!(matches!(
            apply_option(&mut opts, "MaxRounds", "0"),
            SetOptionResult::Ok
        ));
        assert!(opts.max_rounds.is_none());
    }

    #[test]
    fn test_logfile_enable() {
        let mut opts = EngineOptions::default();
        match apply_option(&mut opts, "LogFile", "/tmp/log.txt") {
            SetOptionResult::EnableLogFile(path) => assert_eq!(path, "/tmp/log.txt"),
            _ => panic!("expected EnableLogFile"),
        }
    }

    #[test]
    fn test_logfile_disable() {
        let mut opts = EngineOptions::default();
        assert!(matches!(
            apply_option(&mut opts, "LogFile", "none"),
            SetOptionResult::DisableLogFile
        ));
    }

    #[test]
    fn test_unknown_option() {
        let mut opts = EngineOptions::default();
        assert!(matches!(
            apply_option(&mut opts, "Bogus", "value"),
            SetOptionResult::UnknownOption(_)
        ));
    }
}
