use crate::menu_tree::{Entry, Menu};
use crate::Menu as SwellMenu;

pub fn fill_menu<R>(swell_menu: SwellMenu, menu: &Menu<R>) {
    if swell_menu.entry_count().is_ok_and(|count| count > 0) {
        swell_menu.add_separator();
    }
    for e in &menu.entries {
        fill_menu_recursive(swell_menu, e);
    }
}

fn fill_menu_recursive<R>(swell_menu: SwellMenu, entry: &Entry<R>) {
    match entry {
        Entry::Menu(m) => {
            let sub_menu = swell_menu.add_menu(m.text.as_str());
            for e in &m.entries {
                fill_menu_recursive(sub_menu, e);
            }
        }
        Entry::Item(i) => {
            swell_menu.add_item(i.id, i.text.as_str());
            if i.opts.checked {
                swell_menu.set_item_checked(i.id, true);
            }
            if !i.opts.enabled {
                swell_menu.set_item_enabled(i.id, false);
            }
        }
        Entry::Separator => {
            swell_menu.add_separator();
        }
        Entry::Nothing => {}
    }
}
