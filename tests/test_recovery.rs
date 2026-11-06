#![cfg(feature = "testing")]

use proptest::prelude::*;
use proptest_arbitrary_adapter::arb;

use rustc_dep_info::Contents;

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 1024,
        failure_persistence: None,
        .. ProptestConfig::default()
    })]

    #[test]
    fn contents_round_trip(contents in arb::<Contents>()) {
        let printed = contents.to_string();
        let parsed: Contents = printed.parse().unwrap();
        prop_assert_eq!(parsed, contents);
    }
}
