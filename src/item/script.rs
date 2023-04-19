use std::{
    process::Command,
    thread,
};

use sysinfo::System;

use super::{
    Item,
    ToItem,
};

pub struct Script {
    command: String,

    stdout: Option<String>,
}

impl Default for Script {
    fn default() -> Self {
        Script {
            command: "echo -n Hello, World!".into(),

            stdout: None,
        }
    }
}

impl ToItem for Script {
    fn to_item(&self) -> Item {
        match self.stdout.as_ref() {
            Some(s) => Item::new(s),
            None => Item::new(""),
        }
    }

    fn update(&mut self, _: &mut System) {
        // TODO: set interval and run this based on that

        self.stdout.get_or_insert_with(|| {
            thread::scope(|s| {
                s.spawn(|| {
                    let output = Command::new("sh")
                        .arg("-c")
                        .arg(&self.command)
                        .output()
                        .unwrap();

                    String::from_utf8_lossy(&output.stdout).to_string()
                })
                .join()
                .unwrap()
            })
        });
    }
}
