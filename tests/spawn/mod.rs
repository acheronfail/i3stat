macro_rules! spawn_test {
    ($name:ident, $config:expr, $test_fn:expr) => {
        #[test]
        fn $name() {
            let mut istat = crate::util::TestProgram::run(stringify!($name), $config);
            assert_eq!(
                istat.next_line().unwrap().as_deref(),
                Some(r#"{"version":1,"click_events":true}"#)
            );
            assert_eq!(istat.next_line().unwrap().as_deref(), Some(r#"["#));
            $test_fn(istat);
        }
    };
}

automod::dir!("tests/spawn");
