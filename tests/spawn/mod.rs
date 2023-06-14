macro_rules! spawn_test {
    ($name:ident, $config:expr, $test_fn:expr) => {
        spawn_test!($name, $config, |x| x, $test_fn);
    };

    ($name:ident, $config:expr, $setup_fn:expr, $test_fn:expr) => {
        #[test]
        fn $name() {
            let mut test = crate::util::Test::new(stringify!($name), $config);
            $setup_fn(&mut test);
            let istat = crate::util::TestProgram::spawn(test);
            $test_fn(istat);
        }
    };
}

automod::dir!("tests/spawn");
