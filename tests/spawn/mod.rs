macro_rules! spawn_test {
    ($name:ident, $config:expr, $test_fn:expr) => {
        #[test]
        fn $name() {
            let istat = crate::util::TestProgram::run(stringify!($name), $config);
            $test_fn(istat);
        }
    };
}

automod::dir!("tests/spawn");
