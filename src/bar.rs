use std::collections::HashMap;
use std::fmt::Debug;
use std::ops::{Index, IndexMut};

use hex_color::HexColor;
use serde_json::Value;

use crate::error::Result;
use crate::i3::{I3Item, I3Markup};
use crate::theme::Theme;

type ColorAdjusters = HashMap<HexColor, Box<dyn Fn(&HexColor) -> HexColor>>;

pub struct Bar {
    /// The actual bar items - represents the latest state of each individual bar item
    items: Vec<I3Item>,
    /// Cache for any colour adjusters created
    color_adjusters: ColorAdjusters,
}

impl Debug for Bar {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Bar")
            .field("items", &self.items)
            .field(
                "color_adjusters",
                &self.color_adjusters.keys().collect::<Vec<_>>(),
            )
            .finish()
    }
}

impl Index<usize> for Bar {
    type Output = I3Item;

    fn index(&self, index: usize) -> &Self::Output {
        &self.items[index]
    }
}

impl IndexMut<usize> for Bar {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.items[index]
    }
}

impl Bar {
    /// Construct a new bar
    pub fn new(item_count: usize) -> Bar {
        Bar {
            items: vec![I3Item::empty(); item_count],
            color_adjusters: ColorAdjusters::new(),
        }
    }

    /// Are there any urgent items?
    pub fn any_urgent(&self) -> bool {
        self.items
            .iter()
            .any(|item| item.get_urgent().is_some_and(|urgent| *urgent))
    }

    /// Convert the bar to json
    pub fn to_json(&mut self, theme: &Theme) -> Result<String> {
        Ok(serde_json::to_string(&self.get_items(theme))?)
    }

    /// Convert the bar to a `Value`
    pub fn to_value(&mut self, theme: &Theme) -> Result<Value> {
        Ok(serde_json::to_value(self.get_items(theme))?)
    }

    fn get_items(&mut self, theme: &Theme) -> Vec<I3Item> {
        if theme.powerline_enable {
            self.create_powerline_bar(theme)
        } else {
            self.create_bar(theme)
        }
    }

    /// Return a list of items representing the bar
    fn create_bar(&mut self, theme: &Theme) -> Vec<I3Item> {
        self.items
            .iter()
            .cloned()
            .map(|item| {
                if let Some(true) = item.get_urgent() {
                    item.color(theme.urgent_fg)
                        .background_color(theme.urgent_bg)
                        // disable urgent here, since we override it ourselves to style it more nicely
                        // but we set it as additional data just in case someone wants to use it
                        .urgent(false)
                        .with_data("urgent", true.into())
                } else {
                    item
                }
            })
            .collect()
    }

    /// Return a list of items representing the bar formatted as a powerline
    fn create_powerline_bar(&mut self, theme: &Theme) -> Vec<I3Item> {
        let visible_items = self.items.iter().filter(|i| !i.is_empty()).count();

        // start the powerline index so the theme colours are consistent from right to left
        let powerline_len = theme.powerline.len();
        let mut powerline_bar = vec![];
        let mut powerline_idx = powerline_len - (visible_items % powerline_len);

        // each time we iterate over an item, we place in a separator and then the item itself
        for i in 0..self.items.len() {
            let item = &self.items[i];
            if item.is_empty() {
                continue;
            }

            let instance = i.to_string();
            debug_assert_eq!(item.get_instance().unwrap(), &instance);

            let prev_color = &theme.powerline[powerline_idx % powerline_len];
            let this_color = &theme.powerline[(powerline_idx + 1) % powerline_len];
            powerline_idx += 1;

            let is_urgent = *item.get_urgent().unwrap_or(&false);
            let item_fg = if is_urgent {
                theme.urgent_fg
            } else {
                this_color.fg
            };
            let item_bg = if is_urgent {
                theme.urgent_bg
            } else {
                match item.get_background_color() {
                    Some(bg) => *bg,
                    None => this_color.bg,
                }
            };

            // create the powerline separator
            let mut sep_item = I3Item::new(theme.powerline_separator.to_span())
                .instance(instance)
                .separator(false)
                .markup(I3Markup::Pango)
                .separator_block_width_px(0)
                .color(item_bg)
                .with_data("powerline_sep", true.into());

            // ensure the separator meshes with the previous item's background
            // the first separator doesn't blend with any other item (hence > 0)
            if i > 0 {
                // find the first previous item which isn't empty
                let prev_item = self.items.iter().take(i).rev().find(|i| !i.is_empty());
                if let Some(prev_item) = prev_item {
                    if *prev_item.get_urgent().unwrap_or(&false) {
                        sep_item = sep_item.background_color(theme.urgent_bg);
                    } else {
                        sep_item =
                            sep_item.background_color(match prev_item.get_background_color() {
                                Some(bg) => *bg,
                                None => prev_color.bg,
                            });
                    }
                }
            }

            // replace `config.theme.dim` so it's easy to see
            let adjusted_dim = self
                .color_adjusters
                .entry(theme.dim)
                .or_insert_with(|| Box::new(make_color_adjuster(&theme.bg, &theme.dim)))(
                &item_bg
            );

            powerline_bar.push(sep_item);
            powerline_bar.push(
                item.clone()
                    .full_text(format!(
                        " {} ",
                        // replace `config.theme.dim` use in pango spans
                        item.full_text
                            .replace(&theme.dim.to_string(), &adjusted_dim.to_string())
                    ))
                    .separator(false)
                    .separator_block_width_px(0)
                    .color(match item.get_color() {
                        _ if is_urgent => item_fg,
                        Some(color) if color == &theme.dim => adjusted_dim,
                        Some(color) => *color,
                        _ => item_fg,
                    })
                    .background_color(item_bg)
                    // disable urgent here, since we override it ourselves to style the powerline more nicely
                    // but we set it as additional data just in case someone wants to use it
                    .urgent(false)
                    .with_data("urgent", true.into()),
            );
        }

        powerline_bar
    }
}

/// HACK: this assumes that RGB colours scale linearly - I don't know if they do or not.
/// Used to render the powerline bar and make sure that dim text is visible.
fn make_color_adjuster(bg: &HexColor, fg: &HexColor) -> impl Fn(&HexColor) -> HexColor {
    let r = fg.r.abs_diff(bg.r);
    let g = fg.g.abs_diff(bg.g);
    let b = fg.b.abs_diff(bg.b);
    move |c| {
        HexColor::rgb(
            r.saturating_add(c.r),
            g.saturating_add(c.g),
            b.saturating_add(c.b),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn properly_format_separator_with_empty() {
        let mut bar = Bar::new(3);

        // first item: has a red background
        bar[0] = I3Item::new("0")
            .instance("0")
            .background_color(HexColor::RED);
        // second item: empty (should not be displayed)
        bar[1] = I3Item::new("").instance("1");
        // third item: separator of this one should skip second item, and be the first item's colour
        bar[2] = I3Item::new("2").instance("2");

        let items = bar.create_powerline_bar(&Theme::default());
        // 4 because bar[1] is empty and should be skipped
        assert_eq!(items.len(), 4);
        // separator should be red
        assert_eq!(items[2].get_background_color(), Some(&HexColor::RED));
    }

    #[test]
    fn format_sep_with_all_empty() {
        let mut bar = Bar::new(3);

        bar[0] = I3Item::new("").instance("0");
        bar[1] = I3Item::new("").instance("1");
        bar[2] = I3Item::new("foo")
            .instance("2")
            .background_color(HexColor::RED);

        let items = bar.create_powerline_bar(&Theme::default());
        assert_eq!(items.len(), 2);
        // separator should be red
        assert_eq!(items[0].get_color(), Some(&HexColor::RED));
        assert_eq!(items[0].get_background_color(), None);
        // item itself is red
        assert_eq!(items[1].get_background_color(), Some(&HexColor::RED));
    }
}
