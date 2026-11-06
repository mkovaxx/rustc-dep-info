use crate::{Contents, EnvVal, Metadata};
use indexmap::map::Entry;

impl Contents {
    pub fn merge(
        mut self: Contents,
        other: Contents,
        merge_metadata: impl Fn(Option<Metadata>, Option<Metadata>) -> Option<Metadata>,
        merge_env: impl Fn(Option<EnvVal>, Option<EnvVal>) -> Option<EnvVal>,
    ) -> Contents {
        for (path, metadata) in other.sources {
            match self.sources.entry(path) {
                Entry::Occupied(mut entry) => {
                    let existing = entry.get_mut().take();
                    entry.insert(merge_metadata(existing, metadata));
                }
                Entry::Vacant(entry) => {
                    entry.insert(metadata);
                }
            }
        }
        self.targets.extend(other.targets);
        for (name, value) in other.env {
            match self.env.entry(name) {
                Entry::Occupied(mut entry) => {
                    let existing = entry.get_mut().take();
                    entry.insert(merge_env(existing, value));
                }
                Entry::Vacant(entry) => {
                    entry.insert(value);
                }
            }
        }
        self
    }
}
