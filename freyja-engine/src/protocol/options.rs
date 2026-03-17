//! Engine option handling.

use crate::game_state::GameMode;
use crate::mcts::MctsConfig;
use crate::search::{DEFAULT_BEAM_WIDTH, DEFAULT_MAX_QNODES, SearchConfig};

/// Engine configuration set via `setoption` commands.
#[derive(Debug)]
pub struct EngineOptions {
    /// Game mode (currently only FreeForAll).
    pub game_mode: GameMode,
    /// Beam width for search.
    pub beam_width: u32,
    /// Maximum rounds before auto-stop. None = unlimited.
    pub max_rounds: Option<u32>,

    // ── Stage 13: Time + Beam Tuning ──
    /// Fraction of time budget allocated to Max^n Phase 1 (0.0 - 1.0).
    pub time_split_ratio: f32,
    /// Maximum total nodes (main + qsearch) before search aborts.
    /// 0 = use protocol-provided limit or no limit.
    pub max_nodes: u64,
    /// Maximum quiescence nodes before soft abort.
    pub max_qnodes: u64,
    /// Move noise level (0-100). 0 = deterministic, >0 = probabilistic.
    pub move_noise: u32,
    /// Per-depth beam schedule (comma-separated). None = flat beam_width.
    pub beam_schedule: Option<Vec<usize>>,
    /// Adaptive beam based on position complexity.
    pub adaptive_beam: bool,
    /// Opponent beam ratio: fraction of root player's beam for opponent nodes.
    pub opponent_beam_ratio: f32,

    // ── MCTS parameters ──
    /// Gumbel Top-k for root candidate selection.
    pub gumbel_k: usize,
    /// Softmax temperature for prior policy.
    pub prior_temperature: f32,
    /// Progressive history weight.
    pub ph_weight: f32,
    /// Prior coefficient in non-root UCB selection.
    pub c_prior: f32,
}

impl Default for EngineOptions {
    fn default() -> Self {
        let mcts_defaults = MctsConfig::default();
        Self {
            game_mode: GameMode::FreeForAll,
            beam_width: DEFAULT_BEAM_WIDTH as u32,
            max_rounds: None,
            time_split_ratio: 0.5,
            max_nodes: 0,
            max_qnodes: DEFAULT_MAX_QNODES,
            move_noise: 0,
            beam_schedule: None,
            adaptive_beam: false,
            opponent_beam_ratio: crate::search::DEFAULT_OPPONENT_BEAM_RATIO,
            gumbel_k: mcts_defaults.gumbel_k,
            prior_temperature: mcts_defaults.prior_temperature,
            ph_weight: mcts_defaults.ph_weight,
            c_prior: mcts_defaults.c_prior,
        }
    }
}

impl EngineOptions {
    /// Build a SearchConfig from current options.
    pub fn search_config(&self) -> SearchConfig {
        let mut config = SearchConfig {
            beam_width: self.beam_width as usize,
            max_qnodes: self.max_qnodes,
            ..SearchConfig::default()
        };
        // Apply beam schedule if set
        if let Some(ref schedule) = self.beam_schedule {
            let mut arr = [config.beam_width; crate::search::MAX_DEPTH];
            for (i, &w) in schedule.iter().enumerate() {
                if i < arr.len() {
                    arr[i] = w;
                }
            }
            config.beam_schedule = Some(arr);
        }
        config.move_noise = self.move_noise;
        config.adaptive_beam = self.adaptive_beam;
        config.opponent_beam_ratio = self.opponent_beam_ratio;
        config
    }

    /// Build a MctsConfig from current options.
    pub fn mcts_config(&self) -> MctsConfig {
        MctsConfig {
            gumbel_k: self.gumbel_k,
            prior_temperature: self.prior_temperature,
            ph_weight: self.ph_weight,
            c_prior: self.c_prior,
            ..MctsConfig::default()
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

        // ── Stage 13 options ──
        "TimeSplitRatio" => match value.parse::<f32>() {
            Ok(r) if (0.0..=1.0).contains(&r) => {
                options.time_split_ratio = r;
                SetOptionResult::Ok
            }
            _ => SetOptionResult::InvalidValue(format!(
                "TimeSplitRatio must be a float in [0.0, 1.0], got '{value}'"
            )),
        },
        "MaxNodes" => match value.parse::<u64>() {
            Ok(n) => {
                options.max_nodes = n;
                SetOptionResult::Ok
            }
            Err(_) => SetOptionResult::InvalidValue(format!(
                "MaxNodes must be a non-negative integer, got '{value}'"
            )),
        },
        "MaxQnodes" => match value.parse::<u64>() {
            Ok(n) => {
                options.max_qnodes = n;
                SetOptionResult::Ok
            }
            Err(_) => SetOptionResult::InvalidValue(format!(
                "MaxQnodes must be a non-negative integer, got '{value}'"
            )),
        },
        "MoveNoise" => match value.parse::<u32>() {
            Ok(n) if n <= 100 => {
                options.move_noise = n;
                SetOptionResult::Ok
            }
            _ => SetOptionResult::InvalidValue(format!(
                "MoveNoise must be an integer in [0, 100], got '{value}'"
            )),
        },
        "BeamSchedule" => {
            let parts: Result<Vec<usize>, _> = value
                .split(',')
                .map(|s| s.trim().parse::<usize>())
                .collect();
            match parts {
                Ok(schedule) if !schedule.is_empty() && schedule.iter().all(|&w| w > 0) => {
                    options.beam_schedule = Some(schedule);
                    SetOptionResult::Ok
                }
                _ => SetOptionResult::InvalidValue(format!(
                    "BeamSchedule must be comma-separated positive integers, got '{value}'"
                )),
            }
        }
        "AdaptiveBeam" => match value {
            "true" | "1" => {
                options.adaptive_beam = true;
                SetOptionResult::Ok
            }
            "false" | "0" => {
                options.adaptive_beam = false;
                SetOptionResult::Ok
            }
            _ => SetOptionResult::InvalidValue(format!(
                "AdaptiveBeam must be true/false, got '{value}'"
            )),
        },
        "OpponentBeamRatio" => match value.parse::<f32>() {
            Ok(r) if (0.0..=1.0).contains(&r) => {
                options.opponent_beam_ratio = r;
                SetOptionResult::Ok
            }
            _ => SetOptionResult::InvalidValue(format!(
                "OpponentBeamRatio must be a float in [0.0, 1.0], got '{value}'"
            )),
        },

        // ── MCTS parameters ──
        "GumbelK" => match value.parse::<usize>() {
            Ok(k) if k > 0 => {
                options.gumbel_k = k;
                SetOptionResult::Ok
            }
            _ => SetOptionResult::InvalidValue(format!(
                "GumbelK must be a positive integer, got '{value}'"
            )),
        },
        "PriorTemperature" => match value.parse::<f32>() {
            Ok(t) if t > 0.0 => {
                options.prior_temperature = t;
                SetOptionResult::Ok
            }
            _ => SetOptionResult::InvalidValue(format!(
                "PriorTemperature must be a positive float, got '{value}'"
            )),
        },
        "PHWeight" => match value.parse::<f32>() {
            Ok(w) if w >= 0.0 => {
                options.ph_weight = w;
                SetOptionResult::Ok
            }
            _ => SetOptionResult::InvalidValue(format!(
                "PHWeight must be a non-negative float, got '{value}'"
            )),
        },
        "CPrior" => match value.parse::<f32>() {
            Ok(c) if c >= 0.0 => {
                options.c_prior = c;
                SetOptionResult::Ok
            }
            _ => SetOptionResult::InvalidValue(format!(
                "CPrior must be a non-negative float, got '{value}'"
            )),
        },

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
        assert_eq!(opts.beam_width, DEFAULT_BEAM_WIDTH as u32);
        assert!(opts.max_rounds.is_none());
        assert_eq!(opts.time_split_ratio, 0.5);
        assert_eq!(opts.max_nodes, 0);
        assert_eq!(opts.max_qnodes, DEFAULT_MAX_QNODES);
        assert_eq!(opts.move_noise, 0);
        assert!(opts.beam_schedule.is_none());
        assert!(!opts.adaptive_beam);
        assert_eq!(opts.gumbel_k, 16);
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

    // ── Stage 13 option tests ──

    #[test]
    fn test_set_time_split_ratio() {
        let mut opts = EngineOptions::default();
        assert!(matches!(
            apply_option(&mut opts, "TimeSplitRatio", "0.7"),
            SetOptionResult::Ok
        ));
        assert!((opts.time_split_ratio - 0.7).abs() < 0.001);
    }

    #[test]
    fn test_set_time_split_ratio_invalid() {
        let mut opts = EngineOptions::default();
        assert!(matches!(
            apply_option(&mut opts, "TimeSplitRatio", "1.5"),
            SetOptionResult::InvalidValue(_)
        ));
    }

    #[test]
    fn test_set_max_nodes() {
        let mut opts = EngineOptions::default();
        assert!(matches!(
            apply_option(&mut opts, "MaxNodes", "5000000"),
            SetOptionResult::Ok
        ));
        assert_eq!(opts.max_nodes, 5_000_000);
    }

    #[test]
    fn test_set_max_qnodes() {
        let mut opts = EngineOptions::default();
        assert!(matches!(
            apply_option(&mut opts, "MaxQnodes", "1000000"),
            SetOptionResult::Ok
        ));
        assert_eq!(opts.max_qnodes, 1_000_000);
    }

    #[test]
    fn test_set_move_noise() {
        let mut opts = EngineOptions::default();
        assert!(matches!(
            apply_option(&mut opts, "MoveNoise", "30"),
            SetOptionResult::Ok
        ));
        assert_eq!(opts.move_noise, 30);
    }

    #[test]
    fn test_set_move_noise_invalid() {
        let mut opts = EngineOptions::default();
        assert!(matches!(
            apply_option(&mut opts, "MoveNoise", "200"),
            SetOptionResult::InvalidValue(_)
        ));
    }

    #[test]
    fn test_set_beam_schedule() {
        let mut opts = EngineOptions::default();
        assert!(matches!(
            apply_option(&mut opts, "BeamSchedule", "30,20,12,8"),
            SetOptionResult::Ok
        ));
        assert_eq!(opts.beam_schedule, Some(vec![30, 20, 12, 8]));
    }

    #[test]
    fn test_set_beam_schedule_invalid() {
        let mut opts = EngineOptions::default();
        assert!(matches!(
            apply_option(&mut opts, "BeamSchedule", "30,0,12"),
            SetOptionResult::InvalidValue(_)
        ));
    }

    #[test]
    fn test_set_adaptive_beam() {
        let mut opts = EngineOptions::default();
        assert!(matches!(
            apply_option(&mut opts, "AdaptiveBeam", "true"),
            SetOptionResult::Ok
        ));
        assert!(opts.adaptive_beam);
    }

    #[test]
    fn test_set_gumbel_k() {
        let mut opts = EngineOptions::default();
        assert!(matches!(
            apply_option(&mut opts, "GumbelK", "8"),
            SetOptionResult::Ok
        ));
        assert_eq!(opts.gumbel_k, 8);
    }

    #[test]
    fn test_set_prior_temperature() {
        let mut opts = EngineOptions::default();
        assert!(matches!(
            apply_option(&mut opts, "PriorTemperature", "25.0"),
            SetOptionResult::Ok
        ));
        assert!((opts.prior_temperature - 25.0).abs() < 0.001);
    }

    #[test]
    fn test_set_ph_weight() {
        let mut opts = EngineOptions::default();
        assert!(matches!(
            apply_option(&mut opts, "PHWeight", "0.5"),
            SetOptionResult::Ok
        ));
        assert!((opts.ph_weight - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_set_c_prior() {
        let mut opts = EngineOptions::default();
        assert!(matches!(
            apply_option(&mut opts, "CPrior", "2.0"),
            SetOptionResult::Ok
        ));
        assert!((opts.c_prior - 2.0).abs() < 0.001);
    }

    #[test]
    fn test_search_config_from_options() {
        let mut opts = EngineOptions::default();
        opts.beam_width = 12;
        opts.max_qnodes = 500_000;
        let config = opts.search_config();
        assert_eq!(config.beam_width, 12);
        assert_eq!(config.max_qnodes, 500_000);
    }

    #[test]
    fn test_search_config_with_beam_schedule() {
        let mut opts = EngineOptions::default();
        opts.beam_schedule = Some(vec![30, 20, 12]);
        let config = opts.search_config();
        let schedule = config.beam_schedule.unwrap();
        assert_eq!(schedule[0], 30);
        assert_eq!(schedule[1], 20);
        assert_eq!(schedule[2], 12);
        // Remaining depths should use default beam_width
        assert_eq!(schedule[3], opts.beam_width as usize);
    }

    #[test]
    fn test_mcts_config_from_options() {
        let mut opts = EngineOptions::default();
        opts.gumbel_k = 8;
        opts.prior_temperature = 25.0;
        let config = opts.mcts_config();
        assert_eq!(config.gumbel_k, 8);
        assert!((config.prior_temperature - 25.0).abs() < 0.001);
    }
}
