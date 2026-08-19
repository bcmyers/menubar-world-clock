#![cfg_attr(not(target_os = "macos"), allow(dead_code))]

#[cfg(not(target_os = "macos"))]
compile_error!("world-clock uses AppKit and can only be built on macOS");

mod config;

use std::cell::{OnceCell, RefCell};
use std::path::PathBuf;

use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use objc2::rc::Retained;
use objc2::runtime::{AnyObject, NSObject, ProtocolObject};
use objc2::{AnyThread, ClassType, DefinedClass, MainThreadOnly, define_class, msg_send, sel};
use objc2_app_kit::{
    NSApplication, NSApplicationActivationPolicy, NSAutoresizingMaskOptions, NSFont,
    NSFontWeightRegular, NSImage, NSMenu, NSMenuDelegate, NSMenuItem, NSStatusBar, NSStatusItem,
    NSTextAlignment, NSTextField, NSVariableStatusItemLength, NSView, NSWorkspace,
};
use objc2_foundation::{
    MainThreadMarker, NSDate, NSObjectProtocol, NSPoint, NSRect, NSRunLoop, NSRunLoopCommonModes,
    NSSize, NSString, NSTimer, NSURL,
};

use crate::config::ResolvedClock;

const MENU_WIDTH: f64 = 340.0;
const ROW_HEIGHT: f64 = 25.0;
const TEXT_HEIGHT: f64 = 19.0;
const LEFT_INSET: f64 = 14.0;
const RIGHT_INSET: f64 = 14.0;
const TIME_WIDTH: f64 = 104.0;
const COLUMN_GAP: f64 = 22.0;

struct ClockRow {
    timezone: Tz,
    time_field: Retained<NSTextField>,
}

#[derive(Default)]
struct ClockAppIvars {
    menu: OnceCell<Retained<NSMenu>>,
    rows: RefCell<Vec<ClockRow>>,
    status_item: OnceCell<Retained<NSStatusItem>>,
    timer: RefCell<Option<Retained<NSTimer>>>,
    config_path: RefCell<Option<PathBuf>>,
}

define_class!(
    // SAFETY: NSObject has no subclassing requirements. All AppKit access is
    // confined to the main thread by MainThreadOnly.
    #[unsafe(super = NSObject)]
    #[thread_kind = MainThreadOnly]
    #[ivars = ClockAppIvars]
    struct ClockApp;

    // SAFETY: NSObjectProtocol has no additional safety requirements.
    unsafe impl NSObjectProtocol for ClockApp {}

    // SAFETY: NSMenuDelegate has no additional safety requirements, and
    // ClockApp is restricted to AppKit's main thread.
    unsafe impl NSMenuDelegate for ClockApp {
        #[unsafe(method(menuWillOpen:))]
        fn menu_will_open(&self, _menu: &NSMenu) {
            self.update_times();
            self.start_timer();
        }

        #[unsafe(method(menuDidClose:))]
        fn menu_did_close(&self, _menu: &NSMenu) {
            self.stop_timer();
        }
    }

    impl ClockApp {
        #[unsafe(method(tick:))]
        fn tick(&self, _timer: &NSTimer) {
            self.update_times();
        }

        #[unsafe(method(reloadConfiguration:))]
        fn reload_configuration(&self, _sender: Option<&AnyObject>) {
            if let Err(error) = self.rebuild_menu() {
                eprintln!("World Clock: {error}");
                self.show_config_error(&error);
            }
        }

        #[unsafe(method(openConfiguration:))]
        fn open_configuration(&self, _sender: Option<&AnyObject>) {
            let path = self.ivars().config_path.borrow().clone();
            if let Some(path) = path {
                let path = path.to_string_lossy();
                let url = NSURL::fileURLWithPath(&NSString::from_str(path.as_ref()));
                if !NSWorkspace::sharedWorkspace().openURL(&url) {
                    eprintln!("World Clock: macOS could not open the configuration");
                }
            } else {
                eprintln!("World Clock: the configuration path is unavailable");
            }
        }

        #[unsafe(method(quit:))]
        fn quit(&self, _sender: Option<&AnyObject>) {
            self.stop_timer();
            NSApplication::sharedApplication(self.mtm()).terminate(None);
        }
    }
);

impl ClockApp {
    fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(ClockAppIvars::default());
        // SAFETY: This is NSObject's designated initializer and `this` was
        // allocated as a ClockApp instance.
        unsafe { msg_send![super(this), init] }
    }

    fn start(&self) {
        let mtm = self.mtm();
        let menu = NSMenu::new(mtm);
        menu.setDelegate(Some(ProtocolObject::from_ref(self)));
        self.ivars()
            .menu
            .set(menu.clone())
            .expect("menu is initialized once");

        let status_item =
            NSStatusBar::systemStatusBar().statusItemWithLength(NSVariableStatusItemLength);
        let button = status_item.button(mtm).expect("status item has a button");
        let icon = NSImage::imageWithSystemSymbolName_accessibilityDescription(
            &NSString::from_str("globe.americas.fill"),
            Some(&NSString::from_str("World Clock")),
        )
        .expect("the World Clock SF Symbol is available");
        icon.setTemplate(true);
        button.setTitle(&NSString::new());
        button.setImage(Some(&icon));
        button.setToolTip(Some(&NSString::from_str("World Clock")));
        status_item.setMenu(Some(&menu));
        self.ivars()
            .status_item
            .set(status_item)
            .expect("status item is initialized once");

        if let Ok(path) = config::config_path() {
            *self.ivars().config_path.borrow_mut() = Some(path);
        }

        if let Err(error) = self.rebuild_menu() {
            eprintln!("World Clock: {error}");
            self.show_config_error(&error);
        }
    }

    fn rebuild_menu(&self) -> Result<(), config::ConfigError> {
        let (path, clocks) = config::load_or_create()?;
        *self.ivars().config_path.borrow_mut() = Some(path);

        let menu = self.ivars().menu.get().expect("menu has been created");
        menu.removeAllItems();
        self.ivars().rows.borrow_mut().clear();

        for clock in clocks {
            self.add_clock_row(menu, clock);
        }

        self.add_menu_actions(menu);

        self.update_times();
        Ok(())
    }

    fn show_config_error(&self, error: &config::ConfigError) {
        let menu = self.ivars().menu.get().expect("menu has been created");
        menu.removeAllItems();
        self.ivars().rows.borrow_mut().clear();

        let heading = NSMenuItem::sectionHeaderWithTitle(
            &NSString::from_str("Configuration Error"),
            self.mtm(),
        );
        menu.addItem(&heading);

        let full_error = error.to_string();
        let detail = NSMenuItem::new(self.mtm());
        detail.setTitle(&NSString::from_str(&error_summary(&full_error)));
        detail.setToolTip(Some(&NSString::from_str(&full_error)));
        detail.setEnabled(false);
        menu.addItem(&detail);

        self.add_menu_actions(menu);
    }

    fn add_menu_actions(&self, menu: &NSMenu) {
        menu.addItem(&NSMenuItem::separatorItem(self.mtm()));
        self.add_action_item(menu, "Open Configuration…", sel!(openConfiguration:), "");
        self.add_action_item(
            menu,
            "Reload Configuration",
            sel!(reloadConfiguration:),
            "r",
        );
        menu.addItem(&NSMenuItem::separatorItem(self.mtm()));
        self.add_action_item(menu, "Quit World Clock", sel!(quit:), "q");
    }

    fn add_clock_row(&self, menu: &NSMenu, clock: ResolvedClock) {
        let mtm = self.mtm();
        let row_view = NSView::initWithFrame(
            NSView::alloc(mtm),
            NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(MENU_WIDTH, ROW_HEIGHT)),
        );

        // Full monospacing is intentional for both columns, not just the digits.
        let font = NSFont::monospacedSystemFontOfSize_weight(13.0, unsafe { NSFontWeightRegular });

        let time_x = MENU_WIDTH - RIGHT_INSET - TIME_WIDTH;
        let city_width = time_x - COLUMN_GAP - LEFT_INSET;
        let text_y = (ROW_HEIGHT - TEXT_HEIGHT) / 2.0;

        let city_field = NSTextField::labelWithString(&NSString::from_str(&clock.city), mtm);
        city_field.setFont(Some(&font));
        city_field.setAlignment(NSTextAlignment::Left);
        city_field.setFrame(NSRect::new(
            NSPoint::new(LEFT_INSET, text_y),
            NSSize::new(city_width, TEXT_HEIGHT),
        ));
        city_field.setAutoresizingMask(NSAutoresizingMaskOptions::ViewMaxXMargin);

        let time_field = NSTextField::labelWithString(&NSString::from_str("--- --:--:--"), mtm);
        time_field.setFont(Some(&font));
        time_field.setAlignment(NSTextAlignment::Left);
        time_field.setFrame(NSRect::new(
            NSPoint::new(time_x, text_y),
            NSSize::new(TIME_WIDTH, TEXT_HEIGHT),
        ));
        time_field.setAutoresizingMask(NSAutoresizingMaskOptions::ViewMinXMargin);

        row_view.addSubview(&city_field);
        row_view.addSubview(&time_field);

        let item = NSMenuItem::new(mtm);
        item.setView(Some(&row_view));
        menu.addItem(&item);

        self.ivars().rows.borrow_mut().push(ClockRow {
            timezone: clock.timezone,
            time_field,
        });
    }

    fn add_action_item(&self, menu: &NSMenu, title: &str, action: objc2::runtime::Sel, key: &str) {
        // SAFETY: Each selector is implemented by ClockApp and each key equivalent
        // is a valid NSString.
        let item = unsafe {
            NSMenuItem::initWithTitle_action_keyEquivalent(
                NSMenuItem::alloc(self.mtm()),
                &NSString::from_str(title),
                Some(action),
                &NSString::from_str(key),
            )
        };
        // SAFETY: `self` remains alive for the entire NSApplication run loop and
        // implements the action selectors used above.
        unsafe {
            item.setTarget(Some(self.as_super()));
        }
        menu.addItem(&item);
    }

    fn update_times(&self) {
        let now = Utc::now();
        for row in self.ivars().rows.borrow().iter() {
            row.time_field
                .setStringValue(&NSString::from_str(&format_time(now, row.timezone)));
        }
    }

    fn start_timer(&self) {
        if self.ivars().timer.borrow().is_some() {
            return;
        }

        // Start at the next wall-clock second instead of one second after the
        // menu opened, keeping the display in phase with the macOS clock.
        let now = NSDate::date();
        let next_second =
            NSDate::dateWithTimeIntervalSince1970(now.timeIntervalSince1970().floor() + 1.0);

        // An unscheduled timer in common run-loop modes continues firing while
        // AppKit tracks the open menu, so displayed seconds remain live.
        // SAFETY: `tick:` is registered with the expected NSTimer argument.
        let timer = unsafe {
            NSTimer::initWithFireDate_interval_target_selector_userInfo_repeats(
                NSTimer::alloc(),
                &next_second,
                1.0,
                self.as_super(),
                sel!(tick:),
                None,
                true,
            )
        };
        // SAFETY: The timer and run-loop mode are valid Foundation objects.
        unsafe {
            NSRunLoop::mainRunLoop().addTimer_forMode(&timer, NSRunLoopCommonModes);
        }
        *self.ivars().timer.borrow_mut() = Some(timer);
    }

    fn stop_timer(&self) {
        if let Some(timer) = self.ivars().timer.borrow_mut().take() {
            timer.invalidate();
        }
    }
}

fn error_summary(error: &str) -> String {
    const MAX_CHARS: usize = 96;

    let normalized = error.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.chars().count() <= MAX_CHARS {
        return normalized;
    }

    let mut summary = normalized.chars().take(MAX_CHARS - 1).collect::<String>();
    summary.push('…');
    summary
}

fn format_time(now: DateTime<Utc>, timezone: Tz) -> String {
    // `%a` is the abbreviated weekday; `%H:%M:%S` is zero-padded 24-hour time.
    // No `%Z` is included, so abbreviations such as PDT and EDT never appear.
    now.with_timezone(&timezone)
        .format("%a %H:%M:%S")
        .to_string()
}

fn main() {
    let mtm = MainThreadMarker::new().expect("World Clock must start on the main thread");
    let application = NSApplication::sharedApplication(mtm);
    application.setActivationPolicy(NSApplicationActivationPolicy::Accessory);

    let clock_app = ClockApp::new(mtm);
    clock_app.start();

    application.run();
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;

    #[test]
    fn time_format_has_weekday_zero_padded_24_hour_time_and_seconds() {
        let now = Utc.with_ymd_and_hms(2026, 8, 18, 5, 6, 7).unwrap();
        assert_eq!(format_time(now, chrono_tz::UTC), "Tue 05:06:07");
        assert!(!format_time(now, chrono_tz::UTC).contains("UTC"));
    }

    #[test]
    fn error_summary_is_single_line_and_bounded() {
        let error = format!("bad config\n{}", "x".repeat(120));
        let summary = error_summary(&error);

        assert!(!summary.contains('\n'));
        assert_eq!(summary.chars().count(), 96);
        assert!(summary.ends_with('…'));
    }
}
