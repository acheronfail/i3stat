use serde_json::json;

use crate::i3::{X11Test, TEST_CONFIG_STR};
use crate::util::get_current_exe;

x_test!(it_works, json!({ "items": [] }), |x_test: &X11Test| {
    // assert i3's using the right config
    assert!(x_test.i3_get_config().contains(TEST_CONFIG_STR));

    // assert bars - should only be one
    assert_eq!(x_test.i3_get_bars(), vec!["bar-0"]);

    // assert bar config
    let bar_config = x_test.i3_get_bar("bar-0");
    assert_eq!(bar_config.get("id").unwrap(), "bar-0");
    assert_eq!(bar_config.get("position").unwrap(), "top");

    // assert its running the right status_command
    let status_command = bar_config.get("status_command").unwrap();
    assert!(status_command
        .as_str()
        .unwrap()
        .contains(&get_current_exe().to_string_lossy().to_string()));
});
