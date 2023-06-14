use util::TestProgram;

mod util;

spawn_test!(shuts_down_on_ipc, |mut istat: TestProgram| {
    istat.assert_i3_header();
    istat.shutdown();
    istat.assert_next_line(None);
});
