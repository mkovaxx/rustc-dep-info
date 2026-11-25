use crate::{Contents, EnvKey, EnvVal, Metadata, Source};
use indexmap::map::Entry;

pub trait MergeFnMut<K, V>: FnMut(&K, V, V) -> V {}

impl<F, K, V> MergeFnMut<K, V> for F where F: FnMut(&K, V, V) -> V {}

impl Contents {
    pub fn merge(
        mut self: Contents,
        other: Contents,
        mut merge_metadata: impl MergeFnMut<Source, Option<Metadata>>,
        mut merge_env: impl MergeFnMut<EnvKey, Option<EnvVal>>,
    ) -> Contents {
        for (path, metadata) in other.sources {
            match self.sources.entry(path) {
                Entry::Occupied(mut entry) => {
                    let existing = entry.get_mut().take();
                    entry.insert(merge_metadata(entry.key(), existing, metadata));
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
                    entry.insert(merge_env(entry.key(), existing, value));
                }
                Entry::Vacant(entry) => {
                    entry.insert(value);
                }
            }
        }
        self
    }
}
