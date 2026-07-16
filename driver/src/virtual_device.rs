// Derived from feveal's VINSA 1060 Plus V2 userspace driver.
use std::collections::HashMap;
use std::io::Error;

use evdev::{
    AbsInfo, AbsoluteAxisType, AttributeSet, EventType, InputEvent, Key, UinputAbsSetup,
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
}

pub struct DeviceDispatcher {
    tablet_last_raw_pressed_buttons: u16,
    pen_last_raw_pressed_button: u8,
    tablet_button_id_to_key_code_map: HashMap<u8, Vec<Key>>,
    pen_button_id_to_key_code_map: HashMap<u8, Vec<Key>>,
    virtual_pen: VirtualDevice,
    virtual_keyboard: VirtualDevice,
    was_touching: bool,
    was_in_proximity: bool,
    is_mouse_mode: bool,
    last_x: i32,
    last_y: i32,
    last_valid_x: i32,
    mouse_area_scale: f32,
}

impl DeviceDispatcher {
    const PRESSED: i32 = 1;
    const RELEASED: i32 = 0;
    const HOLD: i32 = 2;

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

        Ok(DeviceDispatcher {
            tablet_last_raw_pressed_buttons: 0xFFFF,
            pen_last_raw_pressed_button: 0,
            tablet_button_id_to_key_code_map,
            pen_button_id_to_key_code_map,
            virtual_pen,
            virtual_keyboard,
            was_touching: false,
            was_in_proximity: false,
            // A drawing tablet must start in absolute/full-area mode.
            is_mouse_mode: false,
            last_x: 2048,
            last_y: 2048,
            mouse_area_scale: 0.3,
            last_valid_x: 2048,
        })
    }

    fn smooth_coordinates(&mut self, x: i32, y: i32) -> (i32, i32) {
        // Keep the optional mouse mode smooth, but do not add drawing lag in
        // tablet mode. The upstream V2 code retained 75% of the previous
        // coordinate, which made precise strokes trail behind the pen.
        let (smoothed_x, smoothed_y) = if self.is_mouse_mode {
            ((self.last_x + x) / 2, (self.last_y + y) / 2)
        } else {
            (x, y)
        };

        self.last_x = smoothed_x;
        self.last_y = smoothed_y;

        (smoothed_x, smoothed_y)
    }

    pub fn dispatch(&mut self, raw_data: &RawDataReader) -> Result<(), Error> {
        // evdev::VirtualDevice::emit() appends SYN_REPORT automatically. All
        // simultaneous pen values must therefore be passed in one call; if X
        // and Y are emitted separately, drawing applications see horizontal
        // and vertical sub-movements instead of one diagonal sample.
        let pen_events = self.pen_events(raw_data);
        self.virtual_pen.emit(&pen_events)?;

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

            // Toggle with B button
            if i == 12 && state == Self::PRESSED {
                self.is_mouse_mode = !self.is_mouse_mode;
                eprintln!(
                    "Mode: {}",
                    if self.is_mouse_mode {
                        "MOUSE"
                    } else {
                        "TABLET"
                    }
                );
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
        for key in &[Key::BTN_TOOL_PEN, Key::BTN_TOUCH] {
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

    fn pen_events(&mut self, raw_data: &RawDataReader) -> Vec<InputEvent> {
        let mut events = Vec::with_capacity(8);
        let y_raw = raw_data.y_axis();
        let is_multimedia_area = y_raw >= 61000;

        if !is_multimedia_area {
            self.last_valid_x = raw_data.x_axis();
        }

        let raw_pen_buttons = raw_data.pen_buttons();
        self.collect_pen_proximity_event(raw_pen_buttons != 0, &mut events);
        self.collect_pen_button_events(raw_pen_buttons, &mut events);
        self.pen_last_raw_pressed_button = raw_pen_buttons;

        // Pressure normalization by mode
        let normalized_pressure = if self.is_mouse_mode {
            Self::normalize_pressure_mode(raw_data.pressure(), 800, 2)
        } else {
            Self::normalize_pressure_mode(raw_data.pressure(), 510, 3)
        };

        let (smoothed_x, smoothed_y) = if is_multimedia_area {
            (self.last_valid_x, 0) // Multimedia area: last X, top Y
        } else {
            self.smooth_coordinates(raw_data.x_axis(), raw_data.y_axis())
        };

        events.extend(self.pen_position_events(
            smoothed_x,
            smoothed_y,
            normalized_pressure,
            is_multimedia_area,
        ));

        self.collect_pen_touch_event(normalized_pressure, &mut events);
        events
    }

    fn normalize_pressure_mode(raw_pressure: i32, threshold: i32, scaling: i32) -> i32 {
        match 2000 - raw_pressure {
            x if x <= threshold => 0,
            x => x * scaling,
        }
    }

    fn pen_position_events(
        &self,
        x_axis: i32,
        y_axis: i32,
        pressure: i32,
        is_multimedia_area: bool,
    ) -> [InputEvent; 3] {
        let (x, y) = if is_multimedia_area {
            (self.last_valid_x, 0) // Use last valid X and top position
        } else if self.is_mouse_mode {
            let center_x = 1024;
            let center_y = 2048;
            let range = (4096.0 * self.mouse_area_scale) as i32;
            let scale_factor = 4096 / range.max(1);

            let scaled_x = ((x_axis - center_x) * scale_factor) + 2048;
            let scaled_y = ((y_axis - center_y) * scale_factor) + 2048;

            (scaled_x.clamp(0, 4096), scaled_y.clamp(0, 4096))
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
                pressure.clamp(0, 6000),
            ),
        ]
    }

    fn collect_pen_proximity_event(&mut self, in_proximity: bool, events: &mut Vec<InputEvent>) {
        if in_proximity != self.was_in_proximity {
            events.push(InputEvent::new(
                EventType::KEY,
                Key::BTN_TOOL_PEN.code(),
                if in_proximity {
                    Self::PRESSED
                } else {
                    Self::RELEASED
                },
            ));
            self.was_in_proximity = in_proximity;
        }
    }

    fn collect_pen_touch_event(&mut self, normalized_pressure: i32, events: &mut Vec<InputEvent>) {
        let is_touching = normalized_pressure > 0;
        if let Some(state) = match (self.was_touching, is_touching) {
            (false, true) => Some(Self::PRESSED),
            (true, false) => Some(Self::RELEASED),
            _ => None,
        } {
            events.push(InputEvent::new(
                EventType::KEY,
                Key::BTN_TOUCH.code(),
                state,
            ));
        }
        self.was_touching = is_touching;
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
    }

    #[test]
    fn tablet_pressure_dead_zone_and_scale_are_stable() {
        assert_eq!(DeviceDispatcher::normalize_pressure_mode(1490, 510, 3), 0);
        assert_eq!(
            DeviceDispatcher::normalize_pressure_mode(1489, 510, 3),
            1533
        );
        assert_eq!(DeviceDispatcher::normalize_pressure_mode(0, 510, 3), 6000);
        assert_eq!(DeviceDispatcher::normalize_pressure_mode(2500, 510, 3), 0);
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
