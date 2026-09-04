use nucleo_matcher::{Config, Matcher};

pub(crate) struct MatchContext {
    pub(crate) matcher: Matcher,
    pub(crate) haystack_buffer: Vec<char>,
    pub(crate) query_buffer: Vec<char>,
}

impl MatchContext {
    pub(crate) fn new() -> Self {
        Self {
            matcher: Matcher::new(Config::DEFAULT),
            haystack_buffer: Vec::new(),
            query_buffer: Vec::new(),
        }
    }
}
