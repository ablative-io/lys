/// A Codex usage object's counts, as kept between `token_count` events.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Totals {
    /// All input tokens, cached ones among them.
    pub input: u64,
    /// Input tokens read from a cache.
    pub cached: u64,
    /// Output tokens.
    pub output: u64,
}

impl Totals {
    /// The totals a Codex usage object says, when it says all three.
    #[must_use]
    pub fn of(usage: &Value) -> Option<Self> {
        Some(Self {
            input: count(usage, "input_tokens")?,
            cached: count(usage, "cached_input_tokens").unwrap_or(0),
            output: count(usage, "output_tokens")?,
        })
    }

    /// What `self` adds over `before`; none when any count went down, which
    /// is a reset, not spend.
    #[must_use]
    pub fn since(&self, before: &Self) -> Option<Self> {
        Some(Self {
            input: self.input.checked_sub(before.input)?,
            cached: self.cached.checked_sub(before.cached)?,
            output: self.output.checked_sub(before.output)?,
        })
    }

    /// The spend figures of a difference.
    #[must_use]
    pub fn figures(&self, context: Option<u64>) -> (Figures, Vec<Unavailable>) {
        let figures = Figures {
            input_tokens: Some(self.input.saturating_sub(self.cached)),
            output_tokens: Some(self.output),
            cache_creation_tokens: None,
            cache_read_tokens: Some(self.cached),
            context_tokens: context,
            running_ms: None,
            ..Figures::default()
        };
        let mut notes = vec![
            Unavailable {
                figure: "dollars_micros".to_owned(),
                reason:
                    "Codex reports dollars only through its app-server; Lys does not read it yet"
                        .to_owned(),
            },
            Unavailable {
                figure: "plan_windows".to_owned(),
                reason: "codex_rollout_has_no_reported_plan_window".to_owned(),
            },
            Unavailable {
                figure: "cache_creation_tokens".to_owned(),
                reason: "codex_reports_none".to_owned(),
            },
            Unavailable {
                figure: "running_ms".to_owned(),
                reason: "measured_from_the_session_process".to_owned(),
            },
        ];
        note("context_tokens", context, "field_absent", &mut notes);
        (figures, notes)
    }
}
