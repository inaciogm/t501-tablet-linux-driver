// Derived from feveal's VINSA 1060 Plus V2 userspace driver.
use std::collections::HashMap;
use std::io::Error;

use evdev::{
    AbsInfo, AbsoluteAxisType, AttributeSet, EventType, InputEvent, Key, RelativeAxisType,
    UinputAbsSetup,
    uinput::{VirtualDevice, VirtualDeviceBuilder},
};

pub struct RawDataReader {
    pub data: Vec<u8>,
}

impl RawDataReader {
    const X_AXIS_HIGH: usize = 1;
    const X_AXIS_LOW: usize = 2;
    const Y_AXIS_HIGH: usize = 3;
    const Y_AXIS_LOW: usize = 4;
    const PRESSURE_HIGH: usize = 5;
    const PRESSURE_LOW: usize = 6;
    const PEN_PRESENCE: usize = 7;
    const PEN_BUTTONS: usize = 9;
    const TABLET_BUTTONS_HIGH: usize = 12;
    const TABLET_BUTTONS_LOW: usize = 11;

    pub fn new() -> Self {
        RawDataReader {
            data: vec![0u8; 64],
        }
    }

    fn u16_from_2_u8(high: u8, low: u8) -> u16 {
        (high as u16) << 8 | low as u16
    }

    fn x_axis(&self) -> i32 {
        let raw = Self::u16_from_2_u8(self.data[Self::X_AXIS_HIGH], self.data[Self::X_AXIS_LOW]);
        raw as i32
    }

    fn y_axis(&self) -> i32 {
        let raw = Self::u16_from_2_u8(self.data[Self::Y_AXIS_HIGH], self.data[Self::Y_AXIS_LOW]);
        raw as i32
    }

    fn pressure(&self) -> i32 {
        Self::u16_from_2_u8(
            self.data[Self::PRESSURE_HIGH],
            self.data[Self::PRESSURE_LOW],
        ) as i32
    }

    fn tablet_buttons_as_binary_flags(&self) -> u16 {
        Self::u16_from_2_u8(
            self.data[Self::TABLET_BUTTONS_HIGH],
            self.data[Self::TABLET_BUTTONS_LOW],
        ) | (0xcc << 8)
    }

    fn pen_buttons(&self) -> u8 {
        self.data[Self::PEN_BUTTONS]
    }

    fn pen_in_proximity(&self) -> bool {
        self.data[Self::PEN_PRESENCE] != 0 || self.data[Self::PEN_BUTTONS] != 0
    }
}

pub struct DeviceDispatcher {
    tablet_last_raw_pressed_buttons: u16,
    pen_last_raw_pressed_button: u8,
    tablet_button_id_to_key_code_map: HashMap<u8, Vec<Key>>,
    pen_button_id_to_key_code_map: HashMap<u8, Vec<Key>>,
    virtual_pen: VirtualDevice,
    virtual_mouse: VirtualDevice,
    virtual_keyboard: VirtualDevice,
    was_touching: bool,
    was_in_proximity: bool,
    pub is_scroll_mode: bool,
    scroll_last_y: i32,
    scroll_start_y: i32,
    scroll_start_x: i32,
    scroll_has_dragged: bool,
    scroll_accumulator: i32,
    last_x: i32,
    last_y: i32,
    last_valid_x: i32,
    mouse_area_scale: f32,
}

impl DeviceDispatcher {
    const PRESSED: i32 = 1;
    const RELEASED: i32 = 0;
    const HOLD: i32 = 2;

    pub const TOUCH_ENTER_RAW: i32 = 1480;
    pub const TOUCH_EXIT_RAW: i32 = 1600;
    pub const FULL_PRESS_RAW: i32 = 770;
    pub const PRESSURE_MAX: i32 = 6000;

    pub const SCROLL_STEP: i32 = 80;
    pub const DRAG_THRESHOLD: i32 = 60;
    pub fn new() -> Result<Self, Error> {
        let tablet_button_id_to_key_code_map = HashMap::from([
            (0, vec![Key::KEY_TAB]),                        // TAB
            (1, vec![Key::KEY_SPACE]),                      // SPACE
            (2, vec![Key::KEY_LEFTALT]),                    // ALT
            (3, vec![Key::KEY_LEFTCTRL]),                   // CTRL
            (4, vec![Key::KEY_PAGEUP]),                     // MOUSE UP
            (5, vec![Key::KEY_PAGEDOWN]),                   // MOUSE DOWN
            (6, vec![Key::KEY_LEFTBRACE]),                  // MOUSE AREA -
            (7, vec![Key::KEY_LEFTCTRL, Key::KEY_KPMINUS]), // CTRL- ZOOM
            (8, vec![Key::KEY_LEFTCTRL, Key::KEY_KPPLUS]),  // CTRL+ ZOOM
            (9, vec![Key::KEY_ESC]),                        // ESC CANCEL
            (12, vec![Key::KEY_B]),                         // TOGGLE MOUSE/TABLET
            (13, vec![Key::KEY_RIGHTBRACE]),                // MOUSE AREA +
        ]);

        let pen_button_id_to_key_code_map =
            HashMap::from([(4, vec![Key::BTN_STYLUS]), (6, vec![Key::BTN_STYLUS2])]);

        let virtual_pen = Self::virtual_pen_builder(
            &pen_button_id_to_key_code_map
                .values()
                .flatten()
                .cloned()
                .collect::<Vec<Key>>(),
        )?;
        let virtual_keyboard = Self::virtual_keyboard_builder(
            &tablet_button_id_to_key_code_map
                .values()
                .flatten()
                .cloned()
                .collect::<Vec<Key>>(),
        )?;
        let virtual_mouse = Self::virtual_mouse_builder()?;

        Ok(DeviceDispatcher {
            tablet_last_raw_pressed_buttons: 0xFFFF,
            pen_last_raw_pressed_button: 0,
            tablet_button_id_to_key_code_map,
            pen_button_id_to_key_code_map,
            virtual_pen,
            virtual_mouse,
            virtual_keyboard,
            was_touching: false,
            was_in_proximity: false,
            is_scroll_mode: false,
            scroll_last_y: 2048,
            scroll_start_y: 2048,
            scroll_start_x: 2048,
            scroll_has_dragged: false,
            scroll_accumulator: 0,
            last_x: 2048,
            last_y: 2048,
            mouse_area_scale: 0.3,
            last_valid_x: 2048,
        })
    }

    fn smooth_coordinates(&mut self, x: i32, y: i32) -> (i32, i32) {
        self.last_x = x;
        self.last_y = y;
        (x, y)
    }

    pub fn dispatch(&mut self, raw_data: &RawDataReader) -> Result<(), Error> {
        // evdev::VirtualDevice::emit() appends SYN_REPORT automatically. All
        // simultaneous pen values must therefore be passed in one call; if X
        // and Y are emitted separately, drawing applications see horizontal
        // and vertical sub-movements instead of one diagonal sample.
        let raw_pen_buttons = raw_data.pen_buttons();
        let is_barrel_scrolling = raw_pen_buttons == 4;

        if self.is_scroll_mode || is_barrel_scrolling {
            self.handle_scroll_input(raw_data)?;
        } else {
            let pen_events = self.pen_events(raw_data);
            if !pen_events.is_empty() {
                self.virtual_pen.emit(&pen_events)?;
            }
        }
        let tablet_events = self.tablet_events(raw_data);
        if !tablet_events.is_empty() {
            self.virtual_keyboard.emit(&tablet_events)?;
        }

        Ok(())
    }

    fn tablet_events(&mut self, raw_data: &RawDataReader) -> Vec<InputEvent> {
        let raw_button_as_binary_flags = raw_data.tablet_buttons_as_binary_flags();
        let events = self.tablet_key_events(raw_button_as_binary_flags);
        self.tablet_last_raw_pressed_buttons = raw_button_as_binary_flags;
        events
    }

    fn virtual_keyboard_builder(tablet_emitted_keys: &[Key]) -> Result<VirtualDevice, Error> {
        let mut key_set = AttributeSet::<Key>::new();
        for key in tablet_emitted_keys {
            key_set.insert(*key);
        }

        VirtualDeviceBuilder::new()?
            .name("Tomate MTM-1106 Pad")
            .with_keys(&key_set)?
            .build()
    }

    fn tablet_key_events(&mut self, raw_button_as_flags: u16) -> Vec<InputEvent> {
        let mut events = Vec::new();
        for id in (0..14).filter(|id| ![10, 11].contains(id)) {
            self.collect_tablet_key_events(id, raw_button_as_flags, &mut events);
        }
        events
    }

    fn collect_tablet_key_events(
        &mut self,
        i: u8,
        raw_button_as_flags: u16,
        events: &mut Vec<InputEvent>,
    ) {
        let id_as_binary_mask = 1 << i;
        let is_pressed = (raw_button_as_flags & id_as_binary_mask) == 0;
        let was_pressed = (self.tablet_last_raw_pressed_buttons & id_as_binary_mask) == 0;

        if let Some(state) = match (was_pressed, is_pressed) {
            (false, true) => Some(Self::PRESSED),
            (true, false) => Some(Self::RELEASED),
            (true, true) => Some(Self::HOLD),
            _ => None,
        } {
            // Button [ - Reduce mouse area
            if i == 6 && state == Self::PRESSED {
                self.mouse_area_scale = (self.mouse_area_scale * 0.8).max(0.1);
                eprintln!("Mouse area reduced: {:.0}%", self.mouse_area_scale * 100.0);
                return;
            }

            // Button ] - Enlarge mouse area
            if i == 13 && state == Self::PRESSED {
                self.mouse_area_scale = (self.mouse_area_scale * 1.2).min(0.4);
                eprintln!(
                    "Mouse area increased: {:.0}%",
                    self.mouse_area_scale * 100.0
                );
                return;
            }

            // Toggle Pen / Scroll mode with B button (button 12 on tablet)
            if i == 12 && state == Self::PRESSED {
                self.is_scroll_mode = !self.is_scroll_mode;
                self.scroll_accumulator = 0;
                self.was_touching = false;
                if self.is_scroll_mode {
                    eprintln!("Mode: SCROLL (drag pen to scroll pages; tap to click)");
                } else {
                    eprintln!("Mode: PEN (standard drawing tablet / stylus)");
                }
                return;
            }

            if let Some(keys) = self.tablet_button_id_to_key_code_map.get(&i) {
                for &key in keys {
                    events.push(InputEvent::new(EventType::KEY, key.code(), state));
                }
            }
        }
    }

    fn virtual_pen_builder(pen_emitted_keys: &[Key]) -> Result<VirtualDevice, Error> {
        let abs_x_setup =
            UinputAbsSetup::new(AbsoluteAxisType::ABS_X, AbsInfo::new(0, 0, 4096, 0, 0, 16));
        let abs_y_setup =
            UinputAbsSetup::new(AbsoluteAxisType::ABS_Y, AbsInfo::new(0, 0, 4096, 0, 0, 27));
        let abs_pressure_setup = UinputAbsSetup::new(
            AbsoluteAxisType::ABS_PRESSURE,
            AbsInfo::new(0, 0, 6000, 0, 0, 0),
        );

        let mut key_set = AttributeSet::<Key>::new();
        for key in pen_emitted_keys {
            key_set.insert(*key);
        }

        // BTN_TOUCH was emitted by the original code without being declared,
        // causing EINVAL/panics on current kernels. BTN_TOOL_PEN is required
        // for libinput/KWin to identify proximity correctly.
        for key in &[Key::BTN_TOOL_PEN, Key::BTN_TOUCH, Key::BTN_LEFT] {
            key_set.insert(*key);
        }

        VirtualDeviceBuilder::new()?
            .name("Tomate MTM-1106 Pen")
            .with_absolute_axis(&abs_x_setup)?
            .with_absolute_axis(&abs_y_setup)?
            .with_absolute_axis(&abs_pressure_setup)?
            .with_keys(&key_set)?
            .build()
    }

    fn virtual_mouse_builder() -> Result<VirtualDevice, Error> {
        let mut key_set = AttributeSet::<Key>::new();
        key_set.insert(Key::BTN_LEFT);
        key_set.insert(Key::BTN_RIGHT);
        key_set.insert(Key::BTN_MIDDLE);

        let mut rel_set = AttributeSet::<RelativeAxisType>::new();
        rel_set.insert(RelativeAxisType::REL_X);
        rel_set.insert(RelativeAxisType::REL_Y);
        rel_set.insert(RelativeAxisType::REL_WHEEL);
        rel_set.insert(RelativeAxisType::REL_WHEEL_HI_RES);
        rel_set.insert(RelativeAxisType::REL_HWHEEL);

        VirtualDeviceBuilder::new()?
            .name("Tomate MTM-1106 Mouse")
            .with_keys(&key_set)?
            .with_relative_axes(&rel_set)?
            .build()
    }

    fn handle_scroll_input(&mut self, raw_data: &RawDataReader) -> Result<(), Error> {
        let pen_in_proximity = raw_data.pen_in_proximity();
        let raw_pressure = raw_data.pressure();
        let is_touching = pen_in_proximity && self.calculate_touch_state(raw_pressure);

        let y_raw = raw_data.y_axis();
        let is_multimedia_area = y_raw >= 61000;
        if !is_multimedia_area {
            self.last_valid_x = raw_data.x_axis();
        }

        let (x, y) = if is_multimedia_area {
            (self.last_valid_x.clamp(0, 4096), 0)
        } else {
            (
                raw_data.x_axis().clamp(0, 4096),
                raw_data.y_axis().clamp(0, 4096),
            )
        };

        // Maintain visible cursor tracking so the user sees where they point
        if pen_in_proximity {
            let mut pen_events = Vec::with_capacity(4);
            if !self.was_in_proximity {
                pen_events.push(InputEvent::new(
                    EventType::KEY,
                    Key::BTN_TOOL_PEN.code(),
                    Self::PRESSED,
                ));
                self.was_in_proximity = true;
            }
            pen_events.push(InputEvent::new(
                EventType::ABSOLUTE,
                AbsoluteAxisType::ABS_X.0,
                x,
            ));
            pen_events.push(InputEvent::new(
                EventType::ABSOLUTE,
                AbsoluteAxisType::ABS_Y.0,
                y,
            ));
            pen_events.push(InputEvent::new(
                EventType::ABSOLUTE,
                AbsoluteAxisType::ABS_PRESSURE.0,
                0,
            ));
            self.virtual_pen.emit(&pen_events)?;
        } else if self.was_in_proximity {
            self.pen_leave_proximity()?;
        }

        let was_touching_prev = self.was_touching;
        self.was_touching = is_touching;

        match (was_touching_prev, is_touching) {
            (false, true) => {
                // Touch contact started on tablet surface
                self.scroll_last_y = y;
                self.scroll_start_y = y;
                self.scroll_start_x = x;
                self.scroll_has_dragged = false;
                self.scroll_accumulator = 0;
            }
            (true, true) => {
                // Pen moving while in contact with tablet surface
                let delta_y = y - self.scroll_last_y;
                self.scroll_last_y = y;

                let move_dist = (x - self.scroll_start_x)
                    .abs()
                    .max((y - self.scroll_start_y).abs());
                if move_dist > Self::DRAG_THRESHOLD {
                    self.scroll_has_dragged = true;
                }

                if self.scroll_has_dragged {
                    self.scroll_accumulator += delta_y;
                    let mut wheel_events = Vec::new();

                    while self.scroll_accumulator >= Self::SCROLL_STEP {
                        self.scroll_accumulator -= Self::SCROLL_STEP;
                        // Pen dragged down -> scroll down (-1 / -120)
                        wheel_events.push(InputEvent::new(
                            EventType::RELATIVE,
                            RelativeAxisType::REL_WHEEL.0,
                            -1,
                        ));
                        wheel_events.push(InputEvent::new(
                            EventType::RELATIVE,
                            RelativeAxisType::REL_WHEEL_HI_RES.0,
                            -120,
                        ));
                    }

                    while self.scroll_accumulator <= -Self::SCROLL_STEP {
                        self.scroll_accumulator += Self::SCROLL_STEP;
                        // Pen dragged up -> scroll up (1 / 120)
                        wheel_events.push(InputEvent::new(
                            EventType::RELATIVE,
                            RelativeAxisType::REL_WHEEL.0,
                            1,
                        ));
                        wheel_events.push(InputEvent::new(
                            EventType::RELATIVE,
                            RelativeAxisType::REL_WHEEL_HI_RES.0,
                            120,
                        ));
                    }

                    if !wheel_events.is_empty() {
                        self.virtual_mouse.emit(&wheel_events)?;
                    }
                }
            }
            (true, false) => {
                // Pen lifted from surface
                if !self.scroll_has_dragged {
                    // Tap click: emit on virtual_pen (the active tool holding the cursor)
                    self.virtual_pen.emit(&[
                        InputEvent::new(EventType::KEY, Key::BTN_TOUCH.code(), Self::PRESSED),
                        InputEvent::new(EventType::ABSOLUTE, AbsoluteAxisType::ABS_PRESSURE.0, 500),
                    ])?;
                    self.virtual_pen.emit(&[
                        InputEvent::new(EventType::KEY, Key::BTN_TOUCH.code(), Self::RELEASED),
                        InputEvent::new(EventType::ABSOLUTE, AbsoluteAxisType::ABS_PRESSURE.0, 0),
                    ])?;
                    // Also emit on virtual_mouse for pointer-only consumers
                    let _ = self.virtual_mouse.emit(&[InputEvent::new(
                        EventType::KEY,
                        Key::BTN_LEFT.code(),
                        Self::PRESSED,
                    )]);
                    let _ = self.virtual_mouse.emit(&[InputEvent::new(
                        EventType::KEY,
                        Key::BTN_LEFT.code(),
                        Self::RELEASED,
                    )]);
                }
                self.scroll_accumulator = 0;
            }
            (false, false) => {}
        }

        if !pen_in_proximity && self.was_in_proximity {
            self.pen_leave_proximity()?;
        }

        Ok(())
    }

    pub fn pen_leave_proximity(&mut self) -> Result<(), Error> {
        self.scroll_accumulator = 0;
        if !self.was_in_proximity && !self.was_touching {
            return Ok(());
        }

        let mut events = Vec::with_capacity(4);
        if self.was_touching {
            events.push(InputEvent::new(
                EventType::KEY,
                Key::BTN_TOUCH.code(),
                Self::RELEASED,
            ));
            self.was_touching = false;
        }

        if self.was_in_proximity {
            events.push(InputEvent::new(
                EventType::KEY,
                Key::BTN_TOOL_PEN.code(),
                Self::RELEASED,
            ));
            self.was_in_proximity = false;
        }

        events.push(InputEvent::new(
            EventType::ABSOLUTE,
            AbsoluteAxisType::ABS_PRESSURE.0,
            0,
        ));

        if let Some(keys) = self
            .pen_button_id_to_key_code_map
            .get(&self.pen_last_raw_pressed_button)
        {
            for key in keys {
                events.push(InputEvent::new(EventType::KEY, key.code(), Self::RELEASED));
            }
        }
        self.pen_last_raw_pressed_button = 0;

        self.virtual_pen.emit(&events)?;
        Ok(())
    }

    fn pen_events(&mut self, raw_data: &RawDataReader) -> Vec<InputEvent> {
        let mut events = Vec::with_capacity(8);
        let pen_in_proximity = raw_data.pen_in_proximity();

        if !pen_in_proximity {
            if self.was_touching {
                events.push(InputEvent::new(
                    EventType::KEY,
                    Key::BTN_TOUCH.code(),
                    Self::RELEASED,
                ));
                self.was_touching = false;
            }
            if self.was_in_proximity {
                events.push(InputEvent::new(
                    EventType::KEY,
                    Key::BTN_TOOL_PEN.code(),
                    Self::RELEASED,
                ));
                self.was_in_proximity = false;
            }
            events.push(InputEvent::new(
                EventType::ABSOLUTE,
                AbsoluteAxisType::ABS_PRESSURE.0,
                0,
            ));
            if let Some(keys) = self
                .pen_button_id_to_key_code_map
                .get(&self.pen_last_raw_pressed_button)
            {
                for key in keys {
                    events.push(InputEvent::new(EventType::KEY, key.code(), Self::RELEASED));
                }
            }
            self.pen_last_raw_pressed_button = 0;
            return events;
        }

        if !self.was_in_proximity {
            events.push(InputEvent::new(
                EventType::KEY,
                Key::BTN_TOOL_PEN.code(),
                Self::PRESSED,
            ));
            self.was_in_proximity = true;
        }

        let y_raw = raw_data.y_axis();
        let is_multimedia_area = y_raw >= 61000;

        if !is_multimedia_area {
            self.last_valid_x = raw_data.x_axis();
        }

        let raw_pressure = raw_data.pressure();
        let is_touching = self.calculate_touch_state(raw_pressure);

        // Kernel evdev requirement: BTN_TOUCH must be emitted FIRST
        // in the synchronization frame among tool state events.
        if is_touching != self.was_touching {
            events.push(InputEvent::new(
                EventType::KEY,
                Key::BTN_TOUCH.code(),
                if is_touching {
                    Self::PRESSED
                } else {
                    Self::RELEASED
                },
            ));
            self.was_touching = is_touching;
        }

        let (smoothed_x, smoothed_y) = if is_multimedia_area {
            (self.last_valid_x, 0)
        } else {
            self.smooth_coordinates(raw_data.x_axis(), raw_data.y_axis())
        };

        let normalized_pressure = self.normalize_pressure(raw_pressure, is_touching);

        events.extend(self.pen_position_events(
            smoothed_x,
            smoothed_y,
            normalized_pressure,
            is_multimedia_area,
        ));

        let raw_pen_buttons = raw_data.pen_buttons();
        self.collect_pen_button_events(raw_pen_buttons, &mut events);
        self.pen_last_raw_pressed_button = raw_pen_buttons;

        events
    }

    pub fn touch_state_with_hysteresis(was_touching: bool, raw_pressure: i32) -> bool {
        if raw_pressure <= Self::TOUCH_ENTER_RAW {
            true
        } else if raw_pressure >= Self::TOUCH_EXIT_RAW {
            false
        } else {
            was_touching
        }
    }

    pub fn normalize_pressure_value(raw_pressure: i32, is_touching: bool) -> i32 {
        if !is_touching {
            return 0;
        }

        let enter_threshold = Self::TOUCH_ENTER_RAW;
        let effective = enter_threshold - raw_pressure;
        if effective <= 0 {
            1
        } else {
            let scale_range = enter_threshold - Self::FULL_PRESS_RAW;
            let scaled = (effective as i64 * Self::PRESSURE_MAX as i64) / scale_range as i64;
            (scaled as i32).clamp(1, Self::PRESSURE_MAX)
        }
    }

    fn calculate_touch_state(&self, raw_pressure: i32) -> bool {
        Self::touch_state_with_hysteresis(self.was_touching, raw_pressure)
    }

    fn normalize_pressure(&self, raw_pressure: i32, is_touching: bool) -> i32 {
        Self::normalize_pressure_value(raw_pressure, is_touching)
    }

    fn pen_position_events(
        &self,
        x_axis: i32,
        y_axis: i32,
        pressure: i32,
        is_multimedia_area: bool,
    ) -> [InputEvent; 3] {
        let (x, y) = if is_multimedia_area {
            (self.last_valid_x, 0)
        } else {
            (x_axis.clamp(0, 4096), y_axis.clamp(0, 4096))
        };

        Self::absolute_pen_events(x, y, pressure)
    }

    fn absolute_pen_events(x: i32, y: i32, pressure: i32) -> [InputEvent; 3] {
        [
            InputEvent::new(EventType::ABSOLUTE, AbsoluteAxisType::ABS_X.0, x),
            InputEvent::new(EventType::ABSOLUTE, AbsoluteAxisType::ABS_Y.0, y),
            InputEvent::new(
                EventType::ABSOLUTE,
                AbsoluteAxisType::ABS_PRESSURE.0,
                pressure.clamp(0, Self::PRESSURE_MAX),
            ),
        ]
    }

    fn collect_pen_button_events(&mut self, pen_button: u8, events: &mut Vec<InputEvent>) {
        let Some((state, id)) = (match (self.pen_last_raw_pressed_button, pen_button) {
            (2, x) if x == 6 || x == 4 => Some((Self::PRESSED, x)),
            (x, 2) if x == 6 || x == 4 => Some((Self::RELEASED, x)),
            (x, y) if x != 2 && x == y => Some((Self::HOLD, x)),
            _ => None,
        }) else {
            return;
        };

        let Some(keys) = self.pen_button_id_to_key_code_map.get(&id) else {
            return;
        };

        for key in keys {
            events.push(InputEvent::new(EventType::KEY, key.code(), state));
        }
    }
}
#[cfg(test)]
mod tests {
    use super::{DeviceDispatcher, RawDataReader};
    use evdev::{AbsoluteAxisType, EventType};

    #[test]
    fn raw_report_decodes_coordinates_pressure_and_pen_state() {
        let mut report = RawDataReader::new();
        report.data[RawDataReader::X_AXIS_HIGH] = 0x12;
        report.data[RawDataReader::X_AXIS_LOW] = 0x34;
        report.data[RawDataReader::Y_AXIS_HIGH] = 0x23;
        report.data[RawDataReader::Y_AXIS_LOW] = 0x45;
        report.data[RawDataReader::PRESSURE_HIGH] = 0x06;
        report.data[RawDataReader::PRESSURE_LOW] = 0x40;
        report.data[RawDataReader::PEN_BUTTONS] = 0x02;

        assert_eq!(report.x_axis(), 0x1234);
        assert_eq!(report.y_axis(), 0x2345);
        assert_eq!(report.pressure(), 0x0640);
        assert_eq!(report.pen_buttons(), 0x02);
        assert!(report.pen_in_proximity());
    }

    #[test]
    fn touch_hysteresis_prevents_hover_flicker() {
        let mut touching = false;

        // Hovering at normal height: raw pressure = 1700 -> not touching, pressure = 0
        touching = DeviceDispatcher::touch_state_with_hysteresis(touching, 1700);
        assert!(!touching);
        assert_eq!(
            DeviceDispatcher::normalize_pressure_value(1700, touching),
            0
        );

        // Hovering very close to surface (near-surface hover): raw = 1520
        // 1520 > TOUCH_ENTER_RAW (1480), touch must NOT engage
        touching = DeviceDispatcher::touch_state_with_hysteresis(touching, 1520);
        assert!(!touching);
        assert_eq!(
            DeviceDispatcher::normalize_pressure_value(1520, touching),
            0
        );

        // Sensor noise while hovering near surface: 1490 (> 1480) -> untouched
        touching = DeviceDispatcher::touch_state_with_hysteresis(touching, 1490);
        assert!(!touching);
        assert_eq!(
            DeviceDispatcher::normalize_pressure_value(1490, touching),
            0
        );

        // Physical touch contact: raw = 1470 (<= 1480) -> touch begins
        touching = DeviceDispatcher::touch_state_with_hysteresis(touching, 1470);
        assert!(touching);
        assert!(DeviceDispatcher::normalize_pressure_value(1470, touching) > 0);

        // Lifting slightly: raw pressure rises back to 1500 (in hysteresis deadband 1480..1600)
        // Must REMAIN touching so strokes don't break during light contact
        touching = DeviceDispatcher::touch_state_with_hysteresis(touching, 1500);
        assert!(touching);

        // Pen lifts away from surface: raw pressure rises to 1620 (>= 1600 TOUCH_EXIT_RAW)
        // Touch is released cleanly
        touching = DeviceDispatcher::touch_state_with_hysteresis(touching, 1620);
        assert!(!touching);
        assert_eq!(
            DeviceDispatcher::normalize_pressure_value(1620, touching),
            0
        );
    }

    #[test]
    fn coordinates_and_pressure_share_one_event_batch() {
        let events = DeviceDispatcher::absolute_pen_events(123, 456, 789);

        assert_eq!(events.len(), 3);
        assert!(
            events
                .iter()
                .all(|event| event.event_type() == EventType::ABSOLUTE)
        );
        assert_eq!(events[0].code(), AbsoluteAxisType::ABS_X.0);
        assert_eq!(events[0].value(), 123);
        assert_eq!(events[1].code(), AbsoluteAxisType::ABS_Y.0);
        assert_eq!(events[1].value(), 456);
        assert_eq!(events[2].code(), AbsoluteAxisType::ABS_PRESSURE.0);
        assert_eq!(events[2].value(), 789);
    }
}
