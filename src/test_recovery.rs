use proptest::prelude::*;

use crate::Contents;

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 1024,
        failure_persistence: None,
        .. ProptestConfig::default()
    })]

    #[test]
    fn contents_round_trip(contents in any::<Contents>()) {
        let printed = contents.to_string();
        let parsed: Contents = printed.parse().unwrap();
        prop_assert_eq!(parsed, contents);
    }
}
