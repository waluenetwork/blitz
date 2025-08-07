use std::sync::{Arc, Mutex};
use tracing::debug;

use smithay::{
    backend::input::{
        InputBackend, KeyboardKeyEvent, PointerButtonEvent, PointerMotionEvent,
        PointerAxisEvent, TouchDownEvent, TouchUpEvent, TouchMotionEvent,
        KeyState as SmithayKeyState, ButtonState, Axis, AxisSource,
    },
    input::{
        keyboard::ModifiersState,
    },
    utils::{Logical, Point},
};

#[derive(Debug, Clone)]
pub enum BlitzEvent {
    Mouse(UiEvent),
    Keyboard(UiEvent),
    Touch(UiEvent),
}

use blitz_traits::events::{
    BlitzKeyEvent, KeyState, UiEvent, BlitzMouseButtonEvent, MouseEventButton, MouseEventButtons,
};
use keyboard_types::{Code, Key, Location, Modifiers};

use crate::error::BlitzSmithayError;

#[derive(Debug, Clone)]
pub struct BlitzMouseEvent {
    pub button: MouseButton,
    pub pressed: bool,
    pub position: Point<f64, Logical>,
    pub modifiers: Modifiers,
}

#[derive(Debug, Clone)]
pub struct BlitzScrollEvent {
    pub delta_x: f64,
    pub delta_y: f64,
    pub position: Point<f64, Logical>,
    pub modifiers: Modifiers,
}

#[derive(Debug, Clone)]
pub struct BlitzTouchEvent {
    pub id: u64,
    pub phase: TouchPhase,
    pub position: Point<f64, Logical>,
    pub force: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Other(u16),
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TouchPhase {
    Started,
    Moved,
    Ended,
    Cancelled,
}

pub struct WaylandEventHandler {
    event_queue: Arc<Mutex<Vec<UiEvent>>>,
    modifiers_state: ModifiersState,
}

impl WaylandEventHandler {
    pub fn new() -> Self {
        Self {
            event_queue: Arc::new(Mutex::new(Vec::new())),
            modifiers_state: ModifiersState::default(),
        }
    }
    
    pub fn handle_keyboard_event<B: InputBackend>(
        &mut self,
        event: &impl KeyboardKeyEvent<B>,
    ) -> Result<(), BlitzSmithayError> {
        debug!("Handling keyboard event: key_code={:?} state={:?}", 
               event.key_code(), event.state());
        
        let blitz_event = self.smithay_keyboard_to_blitz(event)?;
        
        {
            let mut queue = self.event_queue
                .lock()
                .map_err(|_| BlitzSmithayError::EventQueueLocked)?;
            queue.push(UiEvent::KeyDown(blitz_event));
        }
        
        Ok(())
    }
    
    pub fn handle_pointer_button_event<B: InputBackend>(
        &mut self,
        event: &impl PointerButtonEvent<B>,
    ) -> Result<(), BlitzSmithayError> {
        debug!("Handling pointer button event: button={:?} state={:?}", 
               event.button_code(), event.state());
        
        let blitz_event = self.smithay_pointer_button_to_blitz(event)?;
        
        {
            let mut queue = self.event_queue
                .lock()
                .map_err(|_| BlitzSmithayError::EventQueueLocked)?;
            queue.push(UiEvent::MouseDown(blitz_event));
        }
        
        Ok(())
    }
    
    pub fn handle_pointer_motion_event<B: InputBackend>(
        &mut self,
        event: &impl PointerMotionEvent<B>,
    ) -> Result<(), BlitzSmithayError> {
        debug!("Handling pointer motion event: delta=({}, {})", 
               event.delta_x(), event.delta_y());
        
        let blitz_event = self.smithay_pointer_motion_to_blitz(event)?;
        
        {
            let mut queue = self.event_queue
                .lock()
                .map_err(|_| BlitzSmithayError::EventQueueLocked)?;
            queue.push(UiEvent::MouseDown(blitz_event));
        }
        
        Ok(())
    }
    
    pub fn handle_pointer_axis_event<B: InputBackend>(
        &mut self,
        event: &impl PointerAxisEvent<B>,
    ) -> Result<(), BlitzSmithayError> {
        debug!("Handling pointer axis event");
        
        let blitz_event = self.smithay_pointer_axis_to_blitz(event)?;
        
        {
            let mut queue = self.event_queue
                .lock()
                .map_err(|_| BlitzSmithayError::EventQueueLocked)?;
            queue.push(UiEvent::MouseDown(blitz_event));
        }
        
        Ok(())
    }
    
    pub fn handle_touch_down_event<B: InputBackend>(
        &mut self,
        event: &impl TouchDownEvent<B>,
    ) -> Result<(), BlitzSmithayError> {
        debug!("Handling touch down event: slot={:?} position=({}, {})", 
               event.slot(), event.x(), event.y());
        
        let blitz_event = self.smithay_touch_down_to_blitz(event)?;
        
        {
            let mut queue = self.event_queue
                .lock()
                .map_err(|_| BlitzSmithayError::EventQueueLocked)?;
            queue.push(UiEvent::MouseDown(blitz_event));
        }
        
        Ok(())
    }
    
    pub fn handle_touch_up_event<B: InputBackend>(
        &mut self,
        _event: &impl TouchUpEvent<B>,
    ) -> Result<(), BlitzSmithayError> {
        debug!("Handling touch up event: slot={:?}", _event.slot());
        
        let blitz_event = self.smithay_touch_up_to_blitz(_event)?;
        
        {
            let mut queue = self.event_queue
                .lock()
                .map_err(|_| BlitzSmithayError::EventQueueLocked)?;
            queue.push(UiEvent::MouseUp(blitz_event));
        }
        
        Ok(())
    }
    
    pub fn handle_touch_motion_event<B: InputBackend>(
        &mut self,
        event: &impl TouchMotionEvent<B>,
    ) -> Result<(), BlitzSmithayError> {
        debug!("Handling touch motion event: slot={:?} position=({}, {})", 
               event.slot(), event.x(), event.y());
        
        let blitz_event = self.smithay_touch_motion_to_blitz(event)?;
        
        {
            let mut queue = self.event_queue
                .lock()
                .map_err(|_| BlitzSmithayError::EventQueueLocked)?;
            queue.push(UiEvent::MouseMove(blitz_event));
        }
        
        Ok(())
    }
    
    pub fn drain_events(&mut self) -> Result<Vec<UiEvent>, BlitzSmithayError> {
        let mut queue = self.event_queue
            .lock()
            .map_err(|_| BlitzSmithayError::EventQueueLocked)?;
        Ok(queue.drain(..).collect())
    }
    
    fn smithay_keyboard_to_blitz<B: InputBackend>(
        &self,
        event: &impl KeyboardKeyEvent<B>,
    ) -> Result<BlitzKeyEvent, BlitzSmithayError> {
        let key_code = event.key_code();
        let state = match event.state() {
            SmithayKeyState::Pressed => KeyState::Pressed,
            SmithayKeyState::Released => KeyState::Released,
        };
        
        let code = self.smithay_keycode_to_blitz_code(key_code.into());
        let key = self.smithay_keycode_to_blitz_key(key_code.into());
        let modifiers = self.smithay_modifiers_to_blitz_modifiers();
        
        Ok(BlitzKeyEvent {
            key,
            code,
            modifiers,
            location: Location::Standard,
            is_auto_repeating: false,
            is_composing: false,
            state,
            text: None,
        })
    }
    
    fn smithay_pointer_button_to_blitz<B: InputBackend>(
        &self,
        event: &impl PointerButtonEvent<B>,
    ) -> Result<BlitzMouseButtonEvent, BlitzSmithayError> {
        let _button = self.smithay_button_to_blitz_button(event.button_code());
        let _pressed = match event.state() {
            ButtonState::Pressed => true,
            ButtonState::Released => false,
        };
        
        Ok(BlitzMouseButtonEvent {
            x: 0.0,
            y: 0.0,
            button: MouseEventButton::Main,
            buttons: MouseEventButtons::Primary,
            mods: self.smithay_modifiers_to_blitz_modifiers(),
        })
    }
    
    fn smithay_pointer_motion_to_blitz<B: InputBackend>(
        &self,
        event: &impl PointerMotionEvent<B>,
    ) -> Result<BlitzMouseButtonEvent, BlitzSmithayError> {
        Ok(BlitzMouseButtonEvent {
            x: event.delta_x() as f32,
            y: event.delta_y() as f32,
            button: MouseEventButton::Main,
            buttons: MouseEventButtons::None,
            mods: self.smithay_modifiers_to_blitz_modifiers(),
        })
    }
    
    fn smithay_pointer_axis_to_blitz<B: InputBackend>(
        &self,
        event: &impl PointerAxisEvent<B>,
    ) -> Result<BlitzMouseButtonEvent, BlitzSmithayError> {
        let (_delta_x, _delta_y) = match event.source() {
            AxisSource::Wheel | AxisSource::WheelTilt => {
                let delta_x = event.amount(Axis::Horizontal).unwrap_or(0.0);
                let delta_y = event.amount(Axis::Vertical).unwrap_or(0.0);
                (delta_x, delta_y)
            },
            AxisSource::Finger | AxisSource::Continuous => {
                let delta_x = event.amount(Axis::Horizontal).unwrap_or(0.0);
                let delta_y = event.amount(Axis::Vertical).unwrap_or(0.0);
                (delta_x, delta_y)
            }
        };
        
        Ok(BlitzMouseButtonEvent {
            x: _delta_x as f32,
            y: _delta_y as f32,
            button: MouseEventButton::Main,
            buttons: MouseEventButtons::None,
            mods: self.smithay_modifiers_to_blitz_modifiers(),
        })
    }
    
    fn smithay_touch_down_to_blitz<B: InputBackend>(
        &self,
        event: &impl TouchDownEvent<B>,
    ) -> Result<BlitzMouseButtonEvent, BlitzSmithayError> {
        Ok(BlitzMouseButtonEvent {
            x: event.x() as f32,
            y: event.y() as f32,
            button: MouseEventButton::Main,
            buttons: MouseEventButtons::Primary,
            mods: self.smithay_modifiers_to_blitz_modifiers(),
        })
    }
    
    fn smithay_touch_up_to_blitz<B: InputBackend>(
        &self,
        _event: &impl TouchUpEvent<B>,
    ) -> Result<BlitzMouseButtonEvent, BlitzSmithayError> {
        Ok(BlitzMouseButtonEvent {
            x: 0.0,
            y: 0.0,
            button: MouseEventButton::Main,
            buttons: MouseEventButtons::None,
            mods: self.smithay_modifiers_to_blitz_modifiers(),
        })
    }
    
    fn smithay_touch_motion_to_blitz<B: InputBackend>(
        &self,
        event: &impl TouchMotionEvent<B>,
    ) -> Result<BlitzMouseButtonEvent, BlitzSmithayError> {
        Ok(BlitzMouseButtonEvent {
            x: event.x() as f32,
            y: event.y() as f32,
            button: MouseEventButton::Main,
            buttons: MouseEventButtons::None,
            mods: self.smithay_modifiers_to_blitz_modifiers(),
        })
    }
    
    fn smithay_keycode_to_blitz_code(&self, keycode: u32) -> Code {
        match keycode {
            9 => Code::Escape,
            10 => Code::Digit1,
            11 => Code::Digit2,
            12 => Code::Digit3,
            13 => Code::Digit4,
            14 => Code::Digit5,
            15 => Code::Digit6,
            16 => Code::Digit7,
            17 => Code::Digit8,
            18 => Code::Digit9,
            19 => Code::Digit0,
            22 => Code::Backspace,
            23 => Code::Tab,
            24 => Code::KeyQ,
            25 => Code::KeyW,
            26 => Code::KeyE,
            27 => Code::KeyR,
            28 => Code::KeyT,
            29 => Code::KeyY,
            30 => Code::KeyU,
            31 => Code::KeyI,
            32 => Code::KeyO,
            33 => Code::KeyP,
            36 => Code::Enter,
            37 => Code::ControlLeft,
            38 => Code::KeyA,
            39 => Code::KeyS,
            40 => Code::KeyD,
            41 => Code::KeyF,
            42 => Code::KeyG,
            43 => Code::KeyH,
            44 => Code::KeyJ,
            45 => Code::KeyK,
            46 => Code::KeyL,
            50 => Code::ShiftLeft,
            52 => Code::KeyZ,
            53 => Code::KeyX,
            54 => Code::KeyC,
            55 => Code::KeyV,
            56 => Code::KeyB,
            57 => Code::KeyN,
            58 => Code::KeyM,
            62 => Code::ShiftRight,
            64 => Code::AltLeft,
            65 => Code::Space,
            _ => Code::Unidentified,
        }
    }
    
    fn smithay_keycode_to_blitz_key(&self, keycode: u32) -> Key {
        match keycode {
            9 => Key::Escape,
            10 => Key::Character("1".into()),
            11 => Key::Character("2".into()),
            12 => Key::Character("3".into()),
            13 => Key::Character("4".into()),
            14 => Key::Character("5".into()),
            15 => Key::Character("6".into()),
            16 => Key::Character("7".into()),
            17 => Key::Character("8".into()),
            18 => Key::Character("9".into()),
            19 => Key::Character("0".into()),
            22 => Key::Backspace,
            23 => Key::Tab,
            24 => Key::Character("q".into()),
            25 => Key::Character("w".into()),
            26 => Key::Character("e".into()),
            27 => Key::Character("r".into()),
            28 => Key::Character("t".into()),
            29 => Key::Character("y".into()),
            30 => Key::Character("u".into()),
            31 => Key::Character("i".into()),
            32 => Key::Character("o".into()),
            33 => Key::Character("p".into()),
            36 => Key::Enter,
            37 => Key::Control,
            38 => Key::Character("a".into()),
            39 => Key::Character("s".into()),
            40 => Key::Character("d".into()),
            41 => Key::Character("f".into()),
            42 => Key::Character("g".into()),
            43 => Key::Character("h".into()),
            44 => Key::Character("j".into()),
            45 => Key::Character("k".into()),
            46 => Key::Character("l".into()),
            50 => Key::Shift,
            52 => Key::Character("z".into()),
            53 => Key::Character("x".into()),
            54 => Key::Character("c".into()),
            55 => Key::Character("v".into()),
            56 => Key::Character("b".into()),
            57 => Key::Character("n".into()),
            58 => Key::Character("m".into()),
            62 => Key::Shift,
            64 => Key::Alt,
            65 => Key::Character(" ".into()),
            _ => Key::Unidentified,
        }
    }
    
    fn smithay_button_to_blitz_button(&self, button_code: u32) -> MouseButton {
        match button_code {
            0x110 => MouseButton::Left,
            0x111 => MouseButton::Right,
            0x112 => MouseButton::Middle,
            _ => MouseButton::Other(button_code as u16),
        }
    }
    
    fn smithay_modifiers_to_blitz_modifiers(&self) -> Modifiers {
        let mut modifiers = Modifiers::default();
        
        if self.modifiers_state.ctrl {
            modifiers.insert(Modifiers::CONTROL);
        }
        if self.modifiers_state.alt {
            modifiers.insert(Modifiers::ALT);
        }
        if self.modifiers_state.shift {
            modifiers.insert(Modifiers::SHIFT);
        }
        if self.modifiers_state.logo {
            modifiers.insert(Modifiers::SUPER);
        }
        
        modifiers
    }
}

impl Default for WaylandEventHandler {
    fn default() -> Self {
        Self::new()
    }
}
