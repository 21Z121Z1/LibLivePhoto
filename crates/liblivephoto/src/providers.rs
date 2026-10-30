//! Per-operation provider control. Standard Google and Apple parsing remains
//! independent of third-party enable lists; registry ownership is explicit.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HandlerOptions {
    pub disable_third_party: bool,
    pub enabled_third_party: Vec<String>,
}
impl HandlerOptions {
    pub fn allows(&self, name: &str) -> bool {
        !self.disable_third_party
            && (self.enabled_third_party.is_empty()
                || self.enabled_third_party.iter().any(|s| s == name))
    }
}
